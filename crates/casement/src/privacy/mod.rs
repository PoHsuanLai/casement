//! Per-app consent, the composition root's half: how the compositor and Settings keep in step
//! with the model's book (`casement_core::privilege`), and where the two privacy files are
//! (`casement-privacy`: the person's book, the administrator's floor).
//!
//! The book is read when casement starts and never again: the model's copy is the authority
//! and the file is rewritten after every change. The floor is read at start and again when
//! casement is told to (`casementctl privacy reload`, which sends `SIGHUP`).

mod host;
#[cfg(any(feature = "testing", feature = "debug-inject"))]
mod injected;

pub use casement_privacy::{
    FilePrivacyStore, FloorFiles, FloorText, Listing, NoPrivacyStore, Owner, PrivacyFiles,
    PrivacyStore, StoreError, SystemDir, list, load_book, load_floor, save_book,
};
pub use host::{edited, keep, publish, reload_floor, start};

use casement_core::privilege::{ClientStanding, Floor};
use casement_ipc::dbus::peer::{Exe, Standing};

/// The floor in `system`: `/etc/quire`, or an injected folder whose files this user owns in a
/// build for the jail (a test or a scenario cannot make a root-owned floor) and the
/// administrator anywhere else.
pub fn floor_in(system: &SystemDir) -> FloorFiles {
    system.floor(injected_owner())
}

/// Who owns a floor in an injected folder.
fn injected_owner() -> Owner {
    #[cfg(any(feature = "testing", feature = "debug-inject"))]
    {
        Owner::User(rustix::process::getuid().as_raw())
    }
    #[cfg(not(any(feature = "testing", feature = "debug-inject")))]
    {
        Owner::Root
    }
}

/// How the model takes a program whose image the person can write, given `standing` (what
/// `clients::client_standing` made of it: asked every time, never named by a record). A build
/// for the jail takes one that the floor in an injected folder names as that program, so a
/// scenario can pre-approve what it runs from its build folder; every other build, and every
/// floor not injected, leaves `standing` as it is. This is the second exception beside
/// [`injected_owner`], and like it exists only under `testing` or `debug-inject`.
pub fn writable_image(
    system: &SystemDir,
    floor: &Floor,
    found: &Standing,
    exe: &Exe,
    standing: ClientStanding,
) -> ClientStanding {
    #[cfg(any(feature = "testing", feature = "debug-inject"))]
    {
        injected::named_by_floor(system, floor, found, exe, standing)
    }
    #[cfg(not(any(feature = "testing", feature = "debug-inject")))]
    {
        let _ = (system, floor, found, exe);
        standing
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use casement_core::privilege::{FloorRule, ProgramPath, Subject, Verdict};
    use casement_ipc::dbus::peer::{ExePath, FileTrust, WhyNot};
    use casement_protocols::vocab::Power;
    use std::path::PathBuf;

    const BUILT: &str = "/home/me/build/debug/transom";

    /// The build's shape decides whether an injected floor can name a program the person can
    /// write: a release build asks about it every time whatever the floor says.
    #[test]
    fn only_a_build_for_the_jail_lets_the_floor_name_a_program_the_person_can_write() {
        let program = ProgramPath::parse(BUILT).expect("a path");
        let floor = Floor::empty().with_rule(FloorRule {
            power: Power::Screen,
            subject: Subject::Program(program.clone()),
            verdict: Verdict::Allow,
        });
        let found = Standing::Unverified {
            exe_name: Some("transom".to_owned()),
            scope_app: None,
            why: WhyNot::ImageWritable,
        };
        let exe = Exe::Read {
            path: ExePath::new(BUILT).expect("a path"),
            trust: FileTrust::RootOnly,
        };
        let asked = ClientStanding::Unverified {
            exe_name: Some("transom".to_owned()),
        };
        let judged = writable_image(
            &SystemDir::At(PathBuf::from("/scratch/floor")),
            &floor,
            &found,
            &exe,
            asked.clone(),
        );
        let want = if cfg!(any(feature = "testing", feature = "debug-inject")) {
            ClientStanding::Program { exe: program }
        } else {
            asked
        };
        assert_eq!(judged, want);
    }
}
