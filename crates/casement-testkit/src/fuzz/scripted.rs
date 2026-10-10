//! A compositor played by hand, for the runner's own tests: it speaks just enough wire to hand
//! out globals and answer syncs, refuses ids it never heard of with `wl_display.error`, and can
//! be told to misbehave in each way an oracle exists to catch.

use super::host::FuzzHost;
use crate::fixture::ClientId;
use casement_core::InvariantBreak;
use casement_core::WindowId;
use casement_core::policy::ConnectionRole;
use casement_testclient::{Pumped, TestClient};
use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::os::fd::OwnedFd;
use std::os::unix::net::UnixStream;
use std::time::Duration;

/// How the stand-in goes wrong.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Fault {
    /// Nothing: it behaves.
    None,
    /// A protocol error with a code its interface does not document.
    UndocumentedError,
    /// Stops answering syncs.
    Deaf,
    /// Offers the shell global to everyone.
    LeaksPrivileged,
    /// Panics when pumped after the first client request.
    Panics,
    /// Says an invariant is broken.
    BrokenInvariant,
}

const GLOBALS: [(&str, u32); 4] = [
    ("wl_compositor", 5),
    ("wl_shm", 2),
    ("xdg_wm_base", 7),
    ("zwlr_layer_shell_v1", 5),
];

/// Ids below this are objects the stand-in will accept requests on.
const KNOWN_BELOW: u32 = 200;

struct Seat {
    client: TestClient,
    wire: UnixStream,
    registry: u32,
    role: ConnectionRole,
    saw_request: bool,
}

pub(super) struct Scripted {
    seats: BTreeMap<ClientId, Seat>,
    next: u32,
    fault: Fault,
}

impl Scripted {
    pub(super) fn new(fault: Fault) -> Scripted {
        Scripted {
            seats: BTreeMap::new(),
            next: 1,
            fault,
        }
    }
}

fn message(object: u32, opcode: u16, payload: &[u8]) -> Vec<u8> {
    let size = (8 + payload.len()) as u32;
    let mut bytes = object.to_ne_bytes().to_vec();
    bytes.extend(((size << 16) | u32::from(opcode)).to_ne_bytes());
    bytes.extend(payload);
    bytes
}

fn string(text: &str) -> Vec<u8> {
    let mut bytes = ((text.len() + 1) as u32).to_ne_bytes().to_vec();
    bytes.extend(text.as_bytes());
    bytes.push(0);
    while !bytes.len().is_multiple_of(4) {
        bytes.push(0);
    }
    bytes
}

fn word(bytes: &[u8], at: usize) -> u32 {
    let mut four = [0u8; 4];
    four.copy_from_slice(&bytes[at..at + 4]);
    u32::from_ne_bytes(four)
}

impl Seat {
    /// Reads requests and answers them; true if anything was read.
    fn serve(&mut self, fault: Fault) -> bool {
        let mut buffer = [0u8; 65536];
        let n = match self.wire.read(&mut buffer) {
            Ok(n) if n > 0 => n,
            _ => return false,
        };
        let bytes = &buffer[..n];
        let mut at = 0;
        while at + 8 <= bytes.len() {
            let object = word(bytes, at);
            let second = word(bytes, at + 4);
            let size = ((second >> 16) as usize).max(8);
            if at + size > bytes.len() {
                break;
            }
            let opcode = (second & 0xffff) as u16;
            let payload = &bytes[at + 8..at + size];
            self.answer(object, opcode, payload, fault);
            at += size;
        }
        true
    }

    fn answer(&mut self, object: u32, opcode: u16, payload: &[u8], fault: Fault) {
        let mut reply = Vec::new();
        match (object, opcode) {
            (1, 1) => {
                self.registry = word(payload, 0);
                for (name, (interface, version)) in GLOBALS.iter().enumerate() {
                    let sandboxed = matches!(self.role, ConnectionRole::Sandboxed(_));
                    if sandboxed && *interface == "zwlr_layer_shell_v1" {
                        continue;
                    }
                    reply.extend(global(self.registry, name as u32 + 1, interface, *version));
                }
                let leaks = fault == Fault::LeaksPrivileged
                    && !matches!(self.role, ConnectionRole::Shell | ConnectionRole::Agent);
                if leaks {
                    reply.extend(global(self.registry, 99, "quire_shell_v1", 1));
                }
            }
            (1, 0) if fault != Fault::Deaf => {
                let callback = word(payload, 0);
                reply.extend(message(callback, 0, &0u32.to_ne_bytes()));
                reply.extend(message(1, 1, &callback.to_ne_bytes()));
            }
            (1, 0) => {}
            (_, _) if object == 0 || object >= KNOWN_BELOW => {
                // wl_display.error(object 1, invalid_object, text)
                let code: u32 = if fault == Fault::UndocumentedError {
                    77
                } else {
                    0
                };
                let mut body = 1u32.to_ne_bytes().to_vec();
                body.extend(code.to_ne_bytes());
                body.extend(string("no such object"));
                reply.extend(message(1, 0, &body));
            }
            _ => self.saw_request = true,
        }
        // A reply that cannot be written is a client that has gone; nothing to do about it.
        let _ = self.wire.write_all(&reply);
    }
}

fn global(registry: u32, name: u32, interface: &str, version: u32) -> Vec<u8> {
    let mut payload = name.to_ne_bytes().to_vec();
    payload.extend(string(interface));
    payload.extend(version.to_ne_bytes());
    message(registry, 0, &payload)
}

impl FuzzHost for Scripted {
    fn connect(&mut self, role: ConnectionRole) -> ClientId {
        let (ours, wire) = UnixStream::pair().expect("pair");
        wire.set_nonblocking(true).expect("nonblocking");
        let client = TestClient::over(OwnedFd::from(ours)).expect("client");
        let id = ClientId(self.next);
        self.next += 1;
        self.seats.insert(
            id,
            Seat {
                client,
                wire,
                registry: 0,
                role,
                saw_request: false,
            },
        );
        self.pump();
        id
    }

    fn client(&mut self, id: ClientId) -> &mut TestClient {
        &mut self
            .seats
            .get_mut(&id)
            .expect("a client of this host")
            .client
    }

    fn pump(&mut self) -> Pumped {
        let fault = self.fault;
        let mut moved = Pumped::Quiet;
        for _ in 0..16 {
            let mut round = Pumped::Quiet;
            for seat in self.seats.values_mut() {
                if seat.serve(fault) {
                    round = Pumped::Moved;
                }
                round = round.or(seat.client.pump());
                assert!(
                    !(fault == Fault::Panics && seat.saw_request),
                    "the stand-in compositor panics on a request"
                );
            }
            if round == Pumped::Quiet {
                return moved;
            }
            moved = Pumped::Moved;
        }
        moved
    }

    fn advance(&mut self, _by: Duration) {
        self.pump();
    }

    fn invariants(&self) -> Result<(), InvariantBreak> {
        match self.fault {
            Fault::BrokenInvariant => Err(InvariantBreak::KeyWindowNotMapped(WindowId(1))),
            _ => Ok(()),
        }
    }
}
