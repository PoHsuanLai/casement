//! Running a sequence of ops and checking the five oracles after each.

use super::failure::{FuzzFailure, PanicSite};
use super::host::FuzzHost;
use super::ops::{FuzzOp, FuzzRequest, MAX_REPEAT, ObjectRef};
use super::oracles::{self, Snapshot};
use super::prime;
use crate::fixture::{ClientId, Fixture};
use casement_core::policy::ConnectionRole;
use casement_testclient::TestClient;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::time::Duration;

/// Runs `ops` against `fixture`, checking the five oracles after each; the first failure's
/// description.
pub fn run_ops(fixture: &mut Fixture, ops: &[FuzzOp]) -> Result<(), FuzzFailure> {
    run_on(fixture, ops)
}

/// [`run_ops`] on any host (the runner's own tests use a hand-played compositor).
pub fn run_on<H: FuzzHost>(host: &mut H, ops: &[FuzzOp]) -> Result<(), FuzzFailure> {
    let mut run = Run::start(host)?;
    for (step, op) in ops.iter().enumerate() {
        run.apply(op)
            .and_then(|()| run.check())
            .map_err(|failure| FuzzFailure::AtOp {
                step,
                op: op.clone(),
                failure: Box::new(failure),
            })?;
    }
    Ok(())
}

/// A client the fuzzer acts as.
struct Fuzzer {
    role: ConnectionRole,
    id: ClientId,
}

struct Run<'h, H> {
    host: &'h mut H,
    bystander: ClientId,
    resting: Snapshot,
    fuzzers: Vec<Fuzzer>,
}

impl<'h, H: FuzzHost> Run<'h, H> {
    fn start(host: &'h mut H) -> Result<Run<'h, H>, FuzzFailure> {
        let bystander = host.connect(ConnectionRole::Public);
        oracles::alive_pump(host)?;
        let resting = oracles::snapshot(host, bystander);
        Ok(Run {
            host,
            bystander,
            resting,
            fuzzers: Vec::new(),
        })
    }

    fn apply(&mut self, op: &FuzzOp) -> Result<(), FuzzFailure> {
        match op {
            FuzzOp::Advance(ms) => {
                let by = Duration::from_millis(u64::from(*ms));
                catch_unwind(AssertUnwindSafe(|| self.host.advance(by)))
                    .map_err(|_| FuzzFailure::Panicked(PanicSite::Advance))
            }
            FuzzOp::Request {
                role,
                object,
                request,
            } => {
                // Priming and sending pump the host too, so a panic there is the same failure.
                catch_unwind(AssertUnwindSafe(|| {
                    let id = self.fuzzer(role);
                    self.send(id, *object, request);
                }))
                .map_err(|_| FuzzFailure::Panicked(PanicSite::Request))?;
                oracles::alive_pump(self.host)
            }
        }
    }

    /// A live client of `role`, made and primed if there is none.
    fn fuzzer(&mut self, role: &ConnectionRole) -> ClientId {
        let alive = self
            .fuzzers
            .iter()
            .find(|f| &f.role == role && self.host.client(f.id).is_connected())
            .map(|f| f.id);
        if let Some(id) = alive {
            return id;
        }
        let id = self.host.connect(role.clone());
        prime::prime(self.host, id);
        self.fuzzers.push(Fuzzer {
            role: role.clone(),
            id,
        });
        id
    }

    fn send(&mut self, id: ClientId, object: ObjectRef, request: &FuzzRequest) {
        match request {
            FuzzRequest::Repeated { times, request } => {
                for _ in 0..(*times).min(MAX_REPEAT) {
                    self.send(id, object, request);
                }
            }
            FuzzRequest::Wire {
                interface,
                opcode,
                args,
            } => {
                let foreign = self.host.client(self.bystander).live_objects();
                let client = self.host.client(id);
                let (target, interface) = resolve(client, &foreign, object, interface);
                client.send_wire(target, &interface, *opcode, args);
            }
        }
    }

    /// The five oracles.
    fn check(&mut self) -> Result<(), FuzzFailure> {
        oracles::invariants(self.host)?;
        oracles::bystander_ok(self.host, self.bystander, &self.resting)?;
        let known: Vec<(ClientId, ConnectionRole)> =
            std::iter::once((self.bystander, ConnectionRole::Public))
                .chain(self.fuzzers.iter().map(|f| (f.id, f.role.clone())))
                .collect();
        for (id, role) in known {
            oracles::documented_error(self.host, id)?;
            oracles::privilege(self.host, id, &role)?;
        }
        Ok(())
    }
}

/// The object id and interface a request goes to: `object` read in the sender's own tables (or
/// the bystander's, for a foreign one). A `Valid` or `Destroyed` pick prefers objects of the
/// asked interface and otherwise takes the nth of any, so a sparse client still gets fuzzed.
fn resolve(
    client: &TestClient,
    foreign: &[(u32, String)],
    object: ObjectRef,
    interface: &str,
) -> (u32, String) {
    let nth = |list: Vec<(u32, String)>, n: u32| -> Option<(u32, String)> {
        let at = n as usize % list.len().max(1);
        list.into_iter().nth(at)
    };
    let preferring = |list: Vec<(u32, String)>, n: u32| -> Option<(u32, String)> {
        let same: Vec<_> = list
            .iter()
            .filter(|(_, name)| name == interface)
            .cloned()
            .collect();
        nth(if same.is_empty() { list } else { same }, n)
    };
    let picked = match object {
        ObjectRef::Null => Some((0, interface.to_owned())),
        ObjectRef::Valid(n) => preferring(client.live_objects(), n),
        ObjectRef::Destroyed(n) => preferring(client.destroyed_objects(), n),
        ObjectRef::Foreign(n) => preferring(foreign.to_vec(), n),
    };
    picked.unwrap_or((1, "wl_display".to_owned()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fuzz::scripted::{Fault, Scripted};
    use porter_core::SandboxFacts;

    fn on(role: ConnectionRole, object: ObjectRef, interface: &str, opcode: u16) -> FuzzOp {
        FuzzOp::Request {
            role,
            object,
            request: FuzzRequest::Wire {
                interface: interface.to_owned(),
                opcode,
                args: vec![1, 2],
            },
        }
    }

    fn sandboxed() -> ConnectionRole {
        ConnectionRole::Sandboxed(SandboxFacts::Flatpak {
            app: "org.example.Fuzz".to_owned(),
            instance: "1".to_owned(),
        })
    }

    #[test]
    fn a_well_behaved_compositor_passes_requests_on_every_kind_of_object() {
        let ops = [
            on(
                ConnectionRole::Public,
                ObjectRef::Valid(0),
                "wl_compositor",
                0,
            ),
            on(ConnectionRole::Public, ObjectRef::Null, "wl_surface", 1),
            on(
                ConnectionRole::Public,
                ObjectRef::Destroyed(3),
                "wl_region",
                0,
            ),
            on(sandboxed(), ObjectRef::Foreign(1), "wl_shm", 0),
            FuzzOp::Advance(16),
            on(ConnectionRole::Shell, ObjectRef::Valid(5), "xdg_wm_base", 2),
        ];
        run_on(&mut Scripted::new(Fault::None), &ops).expect("every oracle holds");
    }

    #[test]
    fn a_flood_of_one_request_is_sent_whole() {
        let ops = [FuzzOp::Request {
            role: ConnectionRole::Public,
            object: ObjectRef::Valid(0),
            request: FuzzRequest::Repeated {
                times: 500,
                request: Box::new(FuzzRequest::Wire {
                    interface: "wl_compositor".to_owned(),
                    opcode: 0,
                    args: vec![1],
                }),
            },
        }];
        run_on(&mut Scripted::new(Fault::None), &ops).expect("every oracle holds");
    }

    fn failure(fault: Fault) -> String {
        let ops = [
            on(ConnectionRole::Public, ObjectRef::Null, "wl_surface", 0),
            on(sandboxed(), ObjectRef::Valid(0), "wl_compositor", 0),
        ];
        run_on(&mut Scripted::new(fault), &ops).expect_err("the oracle fires")
            .to_string()
    }

    #[test]
    fn an_undocumented_protocol_error_fails_oracle_two() {
        assert!(failure(Fault::UndocumentedError).contains("oracle 2"));
    }

    #[test]
    fn a_deaf_bystander_fails_oracle_three() {
        assert!(failure(Fault::Deaf).contains("oracle 3"));
    }

    #[test]
    fn a_broken_invariant_fails_oracle_four() {
        assert!(failure(Fault::BrokenInvariant).contains("oracle 4"));
    }

    #[test]
    fn a_privileged_global_on_a_public_connection_fails_oracle_five() {
        assert!(failure(Fault::LeaksPrivileged).contains("oracle 5"));
    }

    #[test]
    fn a_panicking_compositor_fails_oracle_one() {
        assert!(failure(Fault::Panics).contains("oracle 1"));
    }
}
