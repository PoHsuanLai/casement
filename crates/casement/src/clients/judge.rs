//! Who the program on a Wayland socket is, read when it connects: the process is held by a
//! descriptor, its facts are read from `/proc`, and they are judged against the desktop's
//! caller table (`/etc/quire/desktop-callers.toml`, root-owned or not believed).

use super::Standing;
use super::image::PinnedImage;
use super::standing::{client_standing, maker_of, peer_of};
use crate::privacy::{SystemDir, writable_image};
use casement_core::privilege::{ClientStanding, Floor};
use casement_ipc::dbus::peer::{Pinned, Prober, Table, judge};
use casement_ipc::dbus::{RealProc, TableFiles};
use casement_wayland::consent::Unwatched;
use casement_wayland::security_context::Maker;
use porter_core::PeerIdentity;
use std::os::unix::net::UnixStream;
use std::sync::Arc;

/// What is known of a connection's program the moment it arrives.
#[derive(Debug, Clone)]
pub struct Arrival {
    /// The app its windows belong to.
    pub peer: PeerIdentity,
    /// How well its program is known.
    pub standing: Standing,
}

impl Arrival {
    /// A program nothing could be learned of: asked about every power, every time.
    pub fn unknown() -> Arrival {
        Arrival {
            peer: PeerIdentity::Unproven,
            standing: Standing::Judged {
                standing: ClientStanding::unproven(),
                watch: Arc::new(Unwatched),
                maker: Maker::Other,
            },
        }
    }
}

/// The desktop's own list of its programs: `/etc/quire/desktop-callers.toml` and its drop-ins,
/// and, in a build for the jail (`testing`, `debug-inject`) whose run has an injected folder
/// (`--system-dir`), the test rows of `desktop-callers.toml` there, after them (`dev_callers`).
/// This is the one place the table is made. A table that cannot be loaded (missing, or not owned
/// by the administrator alone) is an empty one, where no program is one of the desktop's
/// services: it is logged once and the compositor runs, with nobody privileged and the shell
/// unable to connect.
pub fn desktop_table(system: &SystemDir) -> Table {
    with_test_rows(system_table(), system)
}

/// The administrator's table, or the empty one when it cannot be used.
fn system_table() -> Table {
    match TableFiles::desktop().load(&RealProc::system()) {
        Ok(table) => table,
        Err(error) => {
            tracing::error!(
                %error,
                "the desktop's list of its own programs could not be used; no program is taken for one of them, and the shell cannot connect"
            );
            Table::empty()
        }
    }
}

/// `shipped` with the test rows of an injected folder after it; only a build for the jail has
/// them, every other build returns `shipped` as it is.
fn with_test_rows(shipped: Table, system: &SystemDir) -> Table {
    #[cfg(any(feature = "testing", feature = "debug-inject"))]
    {
        super::dev_callers::merged(shipped, system)
    }
    #[cfg(not(any(feature = "testing", feature = "debug-inject")))]
    {
        let _ = system;
        shipped
    }
}

/// Judges the programs that connect against the desktop's caller table.
#[derive(Debug)]
pub struct Judge {
    prober: Prober<RealProc>,
    table: Table,
    /// Where the administrator's floor is read from, for [`writable_image`].
    system: SystemDir,
}

impl Judge {
    /// A judge over `table`, reading the system's `/proc`, for a run whose floor is in `system`.
    pub fn new(table: Table, system: SystemDir) -> Judge {
        Judge {
            prober: Prober::system(),
            table,
            system,
        }
    }

    /// A judge over the desktop's own table ([`desktop_table`]).
    pub fn desktop(system: SystemDir) -> Judge {
        Judge::new(desktop_table(&system), system)
    }

    /// Who the program on the other end of `stream` is, under the administrator's `floor` as the
    /// model holds it.
    pub fn arrival(&self, stream: &UnixStream, floor: &Floor) -> Arrival {
        let Ok(pinned) = Pinned::of_stream(stream) else {
            return Arrival::unknown();
        };
        let Ok(facts) = self.prober.probe(&pinned, None) else {
            return Arrival::unknown();
        };
        let found = judge(&facts, &self.table);
        let standing = writable_image(
            &self.system,
            floor,
            &found,
            &facts.exe,
            client_standing(&found, &facts.exe),
        );
        Arrival {
            peer: peer_of(&found),
            standing: Standing::Judged {
                standing,
                watch: Arc::new(PinnedImage::new(pinned, self.prober.clone(), facts.exe)),
                maker: maker_of(&found),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(any(feature = "testing", feature = "debug-inject"))]
    use casement_core::privilege::ProgramPath;
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn a_program_is_held_looked_at_and_judged_when_it_connects() {
        let (ours, _theirs) = UnixStream::pair().expect("a socket pair");
        let arrival = Judge::new(Table::empty(), SystemDir::Etc).arrival(&ours, &Floor::empty());
        let Standing::Judged {
            standing, watch, ..
        } = arrival.standing
        else {
            panic!("a public client is always judged");
        };
        // This test binary is no desktop service (the table is empty): a program the system
        // owns, or one the person can write (the build folder), or one that could not be read.
        assert!(
            matches!(
                standing,
                ClientStanding::Program { .. } | ClientStanding::Unverified { .. }
            ),
            "{standing:?}"
        );
        assert!(matches!(arrival.peer, PeerIdentity::Unproven));
        assert!(watch.moved().is_none(), "it still runs the image it had");
    }

    /// What a judge made of this test binary (a program the person can write, in the build
    /// folder), under a floor that names the binary.
    #[cfg(any(feature = "testing", feature = "debug-inject"))]
    fn standing_under(system: SystemDir) -> (ClientStanding, ProgramPath) {
        use casement_core::privilege::{FloorRule, Subject, Verdict};
        let me = std::env::current_exe().expect("this binary");
        let me = ProgramPath::parse(&me.to_string_lossy()).expect("a plain path");
        let floor = Floor::empty().with_rule(FloorRule {
            power: casement_protocols::vocab::Power::Screen,
            subject: Subject::Program(me.clone()),
            verdict: Verdict::Allow,
        });
        let (ours, _theirs) = UnixStream::pair().expect("a socket pair");
        let arrival = Judge::new(Table::empty(), system).arrival(&ours, &floor);
        let Standing::Judged { standing, .. } = arrival.standing else {
            panic!("a public client is always judged");
        };
        (standing, me)
    }

    #[test]
    #[cfg(any(feature = "testing", feature = "debug-inject"))]
    fn a_floor_in_an_injected_folder_names_the_binary_that_connects_and_no_other_floor_does() {
        let (named, me) = standing_under(SystemDir::At("/scratch/floor".into()));
        assert_eq!(named, ClientStanding::Program { exe: me });
        let (etc, _) = standing_under(SystemDir::Etc);
        let (ours, _theirs) = UnixStream::pair().expect("a socket pair");
        let Standing::Judged {
            standing: plain, ..
        } = Judge::new(Table::empty(), SystemDir::Etc)
            .arrival(&ours, &Floor::empty())
            .standing
        else {
            panic!("a public client is always judged");
        };
        assert_eq!(etc, plain, "the floor changed nothing");
    }

    /// The build's shape decides whether the injected folder's test rows are read at all: a
    /// build without `testing` and `debug-inject` returns the system's table whatever the folder
    /// holds, so no file can make a program a service there.
    #[test]
    fn only_a_build_for_the_jail_reads_test_rows_from_the_injected_folder() {
        let dir = tempfile::tempdir().expect("a scratch folder");
        let rows = "[[peer]]\napp = \"org.quire.Shell\"\nrole = \"shell\"\nexe = [\"/scratch/bin/sill\"]\n";
        let file = dir.path().join("desktop-callers.toml");
        std::fs::write(&file, rows).expect("the rows file");
        std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o600)).expect("its mode");
        let at = SystemDir::At(dir.path().to_path_buf());
        let want = usize::from(cfg!(any(feature = "testing", feature = "debug-inject")));
        assert_eq!(with_test_rows(Table::empty(), &at).rows().len(), want);
        assert!(
            with_test_rows(Table::empty(), &SystemDir::Etc)
                .rows()
                .is_empty()
        );
    }
}
