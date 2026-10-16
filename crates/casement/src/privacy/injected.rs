//! Test builds only (`testing`, `debug-inject`): a program run from the person's build folder
//! is named by a row of the floor in the injected folder (`--system-dir`). A real run asks
//! about such a program every time and no record names it, because anyone who can write the
//! image can make it do anything; a scenario's floor has to name them or the scenario waits on
//! a question nobody answers. The other test-only exception is `injected_owner` (`mod.rs`).

use super::SystemDir;
use casement_core::privilege::{ClientStanding, Floor, ProgramPath, Subject};
use casement_ipc::dbus::peer::{Exe, Standing, WhyNot};

/// `standing`, or the program itself when `found` is a program the person can write, the
/// floor was read from an injected folder, and a row of it names the program's image. The
/// model then gives it the floor's verdict like any program the system owns. The person's book
/// is not asked: a record of theirs cannot name such a program.
pub(super) fn named_by_floor(
    system: &SystemDir,
    floor: &Floor,
    found: &Standing,
    exe: &Exe,
    standing: ClientStanding,
) -> ClientStanding {
    match (system, found, exe) {
        (
            SystemDir::At(_),
            Standing::Unverified {
                why: WhyNot::ImageWritable,
                ..
            },
            Exe::Read { path, .. },
        ) => match ProgramPath::parse(&path.to_text()) {
            Ok(program) if names(floor, &program) => ClientStanding::Program { exe: program },
            _ => standing,
        },
        _ => standing,
    }
}

/// Whether a row of `floor` is about `program`.
fn names(floor: &Floor, program: &ProgramPath) -> bool {
    let subject = Subject::Program(program.clone());
    floor.rules().iter().any(|rule| rule.subject == subject)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clients::client_standing;
    use casement_core::privilege::{
        Book, Decision, FloorRule, Grant, UnixSeconds, Verdict, decide,
    };
    use casement_ipc::dbus::peer::{ExePath, FileTrust};
    use casement_protocols::vocab::{Memory, Power};
    use std::path::PathBuf;

    const BUILT: &str = "/home/me/build/debug/transom";

    /// The judgement of a program run from a build folder, and its image. (The image's trust
    /// is already folded into the judgement, which is all `named_by_floor` reads.)
    fn writable() -> (Standing, Exe) {
        let exe = Exe::Read {
            path: ExePath::new(BUILT).expect("a path"),
            trust: FileTrust::RootOnly,
        };
        let found = Standing::Unverified {
            exe_name: Some("transom".to_owned()),
            scope_app: None,
            why: WhyNot::ImageWritable,
        };
        (found, exe)
    }

    fn built() -> Subject {
        Subject::Program(ProgramPath::parse(BUILT).expect("a path"))
    }

    fn floor_naming(subject: Subject) -> Floor {
        Floor::empty().with_rule(FloorRule {
            power: Power::Screen,
            subject,
            verdict: Verdict::Allow,
        })
    }

    fn system_at() -> SystemDir {
        SystemDir::At(PathBuf::from("/scratch/floor"))
    }

    /// What the model decides for the program, as the compositor judges it.
    fn decision(system: &SystemDir, floor: &Floor, book: &Book) -> Decision {
        let (found, exe) = writable();
        let standing = named_by_floor(system, floor, &found, &exe, client_standing(&found, &exe));
        decide(floor, book, &standing.identity(), Power::Screen)
    }

    #[test]
    fn a_row_of_the_injected_floor_decides_for_a_program_the_person_can_write() {
        let floor = floor_naming(built());
        assert_eq!(
            decision(&system_at(), &floor, &Book::empty()),
            Decision::Open
        );
    }

    #[test]
    fn nothing_else_names_a_program_the_person_can_write() {
        let asked = Decision::Ask(Memory::ThisTimeOnly);
        let book = Book::empty().with(Grant {
            power: Power::Screen,
            subject: built(),
            verdict: Verdict::Allow,
            at: UnixSeconds(1),
        });
        let other = Subject::Program(ProgramPath::parse("/usr/bin/grim").expect("a path"));
        let cases = [
            (
                "a floor that is not the injected one",
                SystemDir::Etc,
                floor_naming(built()),
                Book::empty(),
            ),
            (
                "a book row, whatever the floor",
                system_at(),
                Floor::empty(),
                book,
            ),
            (
                "a row about another program",
                system_at(),
                floor_naming(other),
                Book::empty(),
            ),
        ];
        for (case, system, floor, book) in cases {
            assert_eq!(decision(&system, &floor, &book), asked, "{case}");
        }
    }

    #[test]
    fn only_a_program_the_person_can_write_is_named() {
        let (_, exe) = writable();
        let gone = Standing::Unverified {
            exe_name: None,
            scope_app: None,
            why: WhyNot::PeerGone,
        };
        let floor = floor_naming(built());
        let standing = ClientStanding::unproven();
        let judged = named_by_floor(&system_at(), &floor, &gone, &exe, standing.clone());
        assert_eq!(judged, standing);
    }
}
