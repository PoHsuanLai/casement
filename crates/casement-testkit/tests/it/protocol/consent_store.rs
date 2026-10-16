//! The privacy files behind consent (`casement-privacy`, `casement::privacy`): an answer the
//! person gives is written to their book and remembered across a restart, the administrator's
//! floor decides first and cannot be overruled, a damaged book is kept aside and the program
//! is asked again, a floor file that is not the administrator's is ignored, and a program that
//! cannot be recognised is never written down. Every test uses real files in a scratch folder
//! (never the real state or `/etc`), a scripted shell for the person, and a fixed date.

use super::consent::{ScriptedShell, advertised, golden_of, program, use_keys};
use casement::privacy::{FloorFiles, Owner, PrivacyFiles};
use casement::time::WallClock;
use casement_core::Event;
use casement_core::privilege::{ClientStanding, UnixSeconds, Verdict};
use casement_protocols::vocab::{Answer, Memory, Power};
use casement_testkit::{ASKED_PROGRAM, Fixture, FixtureSpec, program_standing};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

/// The date every answer in these tests is stamped with.
const TODAY: u64 = 1_760_000_000;

/// A scratch home: casement's state folder and the administrator's folder.
struct Home {
    dir: tempfile::TempDir,
}

impl Home {
    fn new() -> Home {
        Home {
            dir: tempfile::tempdir().expect("a scratch folder"),
        }
    }

    fn me() -> u32 {
        rustix::process::getuid().as_raw()
    }

    fn book(&self) -> PathBuf {
        self.dir.path().join("state/casement/privacy.toml")
    }

    fn system(&self) -> PathBuf {
        self.dir.path().join("system")
    }

    /// The files, with the floor's files expected to belong to `owner`.
    fn files(&self, owner: u32) -> PrivacyFiles {
        PrivacyFiles {
            book: self.book(),
            floor: FloorFiles::in_dir(&self.system(), Owner::User(owner)),
        }
    }

    /// The administrator's `privacy.toml`, with the permission bits `mode`.
    fn floor(&self, text: &str, mode: u32) {
        std::fs::create_dir_all(self.system()).expect("a folder");
        let path = self.system().join("privacy.toml");
        std::fs::write(&path, text).expect("a floor");
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(mode)).expect("a mode");
    }

    /// casement starting (again) on these files, with the shell bound to ask.
    fn start(&self) -> (Fixture, ScriptedShell) {
        self.start_owned_by(Home::me())
    }

    fn start_owned_by(&self, owner: u32) -> (Fixture, ScriptedShell) {
        let spec = FixtureSpec::desktop()
            .with_privacy(self.files(owner))
            .with_wall(WallClock::Fixed(UnixSeconds(TODAY)));
        let mut fixture = Fixture::start(spec).expect("fixture");
        let shell = ScriptedShell::start(&mut fixture);
        (fixture, shell)
    }
}

fn text_of(path: &Path) -> String {
    std::fs::read_to_string(path).expect("a file")
}

/// An answer is written to the person's book, stamped with the date, and a restart remembers it:
/// the program is not asked again, and a refusal is remembered too.
#[test]
fn an_answer_is_written_to_the_book_and_a_restart_remembers_it() {
    let home = Home::new();
    {
        let (mut fixture, shell) = home.start();
        let client = fixture.connect_via_ask();
        let _board = use_keys(&mut fixture, client);
        shell.answer(&mut fixture, shell.only_ask(), Answer::Allow);
        let book = text_of(&home.book());
        assert!(book.contains("power = \"keys\""), "{book}");
        assert!(book.contains("answer = \"allow\""), "{book}");
        assert!(book.contains(&format!("at = {TODAY}")), "{book}");
        assert!(book.contains(ASKED_PROGRAM), "{book}");
    }
    let (mut fixture, shell) = home.start();
    assert_eq!(
        fixture
            .model()
            .privilege()
            .book()
            .get(&program(ASKED_PROGRAM), Power::Keys),
        Some(Verdict::Allow),
        "read back at start"
    );
    let client = fixture.connect_via_ask();
    let _board = use_keys(&mut fixture, client);
    assert!(shell.asked().is_empty(), "the program is not asked again");

    // Settings turns it off: the book follows, and the next start hides the power.
    fixture.feed_events(vec![Event::PrivilegeRevoked {
        subject: program(ASKED_PROGRAM),
        power: Power::Keys,
        at: UnixSeconds(TODAY + 1),
    }]);
    assert!(text_of(&home.book()).contains("answer = \"deny\""));
    let (mut fixture, _shell) = home.start();
    let refused = fixture.connect_via_ask();
    fixture.pump();
    let shown = advertised(&mut fixture, refused);
    assert!(
        golden_of(Power::Keys)
            .iter()
            .all(|interface| !shown.contains(interface)),
        "{shown:?}"
    );
}

/// The administrator's floor decides first: a program it refuses is not shown the power, is not
/// asked, and the person's own switch cannot overrule it. Read again on request.
#[test]
fn the_floor_denies_and_cannot_be_overruled() {
    let home = Home::new();
    home.floor(
        &format!(
            "[[rule]]\npower = \"keys\"\nsubject = {{ program = \"{ASKED_PROGRAM}\" }}\nanswer = \"deny\"\n"
        ),
        0o644,
    );
    let (mut fixture, shell) = home.start();
    let keys = golden_of(Power::Keys);
    let hidden_from_a_new_client = |fixture: &mut Fixture| {
        let client = fixture.connect_via_ask();
        fixture.pump();
        let shown = advertised(fixture, client);
        keys.iter().all(|interface| !shown.contains(interface))
    };
    assert!(
        hidden_from_a_new_client(&mut fixture),
        "refused from the start"
    );
    fixture.feed_events(vec![Event::PrivilegeGranted {
        subject: program(ASKED_PROGRAM),
        power: Power::Keys,
        at: UnixSeconds(TODAY),
    }]);
    assert!(
        hidden_from_a_new_client(&mut fixture),
        "the person's switch does not overrule it"
    );
    assert!(shell.asked().is_empty(), "and nobody was asked");

    // The administrator lifts it and asks casement to read the floor again.
    home.floor("# nothing\n", 0o644);
    fixture.reload_floor();
    assert!(!hidden_from_a_new_client(&mut fixture), "read again");
}

/// A book that cannot be read is kept under another name, whole, and the program is asked
/// again; the new book is written from the answer.
#[test]
fn a_damaged_book_is_set_aside_and_the_program_is_asked_again() {
    let home = Home::new();
    std::fs::create_dir_all(home.book().parent().expect("a folder")).expect("a folder");
    std::fs::write(home.book(), "version = 1\n[[grant\n").expect("a damaged book");
    let (mut fixture, shell) = home.start();
    let kept = home
        .book()
        .with_file_name(format!("privacy.toml.bad-{TODAY}"));
    assert_eq!(text_of(&kept), "version = 1\n[[grant\n", "kept whole");
    let client = fixture.connect_via_ask();
    let _board = use_keys(&mut fixture, client);
    assert_eq!(shell.asked().len(), 1, "asked again");
    shell.answer(&mut fixture, shell.only_ask(), Answer::Allow);
    assert!(
        text_of(&home.book()).contains("answer = \"allow\""),
        "a new book"
    );
}

/// A floor file that is not the administrator's alone (someone else owns it, or the group can
/// write it) is not read: the compositor starts, nothing is hidden, and the program is asked.
#[test]
fn a_floor_file_with_the_wrong_owner_is_ignored_safely() {
    // (case, who the files must belong to, the file's mode)
    let cases = [
        ("owned by someone else", Home::me() + 1, 0o644),
        ("writable by the group", Home::me(), 0o664),
    ];
    for (case, owner, mode) in cases {
        let home = Home::new();
        home.floor("[default]\nkeys = \"deny\"\nscreen = \"deny\"\n", mode);
        let (mut fixture, shell) = home.start_owned_by(owner);
        let client = fixture.connect_via_ask();
        fixture.pump();
        let shown = advertised(&mut fixture, client);
        for power in [Power::Keys, Power::Screen] {
            assert!(
                golden_of(power)
                    .iter()
                    .all(|interface| shown.contains(interface)),
                "{case}: {power:?} is not hidden"
            );
        }
        let _board = use_keys(&mut fixture, client);
        assert_eq!(shell.asked().len(), 1, "{case}: asked as with no floor");
    }
}

/// A program the compositor cannot recognise (an interpreter, a program the person can write)
/// is asked as "Allow Once": the answer lasts the connection and is never written to the book,
/// so it is asked again after a restart.
#[test]
fn an_interpreter_or_a_user_writable_program_is_never_written_to_the_book() {
    let cases: [(&str, ClientStanding); 2] = [
        ("an interpreter", program_standing("/usr/bin/python3.13")),
        (
            "a program the person can write",
            ClientStanding::Unverified {
                exe_name: Some("payload".to_owned()),
            },
        ),
    ];
    for (case, standing) in cases {
        let home = Home::new();
        {
            let (mut fixture, shell) = home.start();
            let client = fixture.connect_standing(standing.clone());
            let _board = use_keys(&mut fixture, client);
            let asked = shell.asked();
            assert_eq!(asked.len(), 1, "{case}");
            assert_eq!(asked[0].memory, Memory::ThisTimeOnly, "{case}");
            shell.answer(&mut fixture, asked[0].ask, Answer::Allow);
            assert!(fixture.model().privilege().book().is_empty(), "{case}");
            assert!(!home.book().exists(), "{case}: nothing was written");
        }
        let (mut fixture, shell) = home.start();
        let client = fixture.connect_standing(standing);
        let _board = use_keys(&mut fixture, client);
        assert_eq!(
            shell.asked().len(),
            1,
            "{case}: asked again after a restart"
        );
    }
}
