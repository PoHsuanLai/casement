//! Per-app consent at the wire (`casement_wayland::consent`, `quire_shell_v1` version 5): the
//! shell is asked once for a program and a power, the answer is kept for the program, a program
//! the person refused is not shown the globals of that power, and the shell's side of the
//! protocol is bound by its version. The helpers here (a scripted shell that hears the
//! questions and answers them) are what each protocol's own consent tests use.
//!
//! The first use of a power in these tests is a virtual keyboard's creation, the simplest request
//! that is a use.

use casement_core::Event;
use casement_core::policy::ConnectionRole;
use casement_core::privilege::{
    ASK_TTL, Book, ClientStanding, Grant, ProgramPath, SHELL_WAIT, Subject, UnixSeconds, Verdict,
};
use casement_protocols::client::quire_shell_v1::quire_shell_v1::{self as wire, QuireShellV1};
use casement_protocols::vocab::{Answer, Memory, Power, SubjectKind, WireCode};
use casement_testclient::ClientState;
use casement_testclient::ime::virtual_keyboard::VirtualBoard;
use casement_testclient::quire::shell::ShellHandle;
use casement_testkit::{ASKED_PROGRAM, ClientId, Fixture, FixtureSpec, program_standing};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;
use wayland_client::{Connection, QueueHandle, WEnum};

const GOLDEN: &str = include_str!("../../golden/globals.txt");

/// One `permission_ask` the scripted shell heard.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Asked {
    /// The question's id.
    pub(super) ask: u32,
    /// The power.
    pub(super) power: Power,
    /// How well the compositor knows the program.
    pub(super) kind: SubjectKind,
    /// Its path, its app id or its file name.
    pub(super) subject: String,
    /// Whether the answer is kept.
    pub(super) memory: Memory,
}

/// What the scripted shell has heard.
#[derive(Debug, Default)]
struct Heard {
    asks: Vec<Asked>,
    withdrawn: Vec<u32>,
}

/// The shell object's user data: where its events are written.
#[derive(Debug, Clone, Default)]
pub(super) struct ShellEar(Arc<Mutex<Heard>>);

impl ShellEar {
    fn heard(&self) -> MutexGuard<'_, Heard> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// The vocabulary value a wire enum carries, when it names one.
fn vocab<T: WireCode, W: Into<u32>>(wire: WEnum<W>) -> Option<T> {
    match wire {
        WEnum::Value(value) => T::from_code(value.into()).ok(),
        WEnum::Unknown(_) => None,
    }
}

impl wayland_client::Dispatch<QuireShellV1, ShellEar> for ClientState {
    fn event(
        _: &mut Self,
        _: &QuireShellV1,
        event: wire::Event,
        ear: &ShellEar,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            wire::Event::PermissionAsk {
                ask,
                power,
                subject_kind,
                subject,
                memory,
            } => {
                let heard = vocab(power).zip(vocab(subject_kind)).zip(vocab(memory));
                if let Some(((power, kind), memory)) = heard {
                    ear.heard().asks.push(Asked {
                        ask,
                        power,
                        kind,
                        subject,
                        memory,
                    });
                }
            }
            wire::Event::PermissionWithdraw { ask } => ear.heard().withdrawn.push(ask),
            _ => {}
        }
    }
}

/// The shell's connection with `quire_shell_v1` bound at version 5, listening for questions.
pub(super) struct ScriptedShell {
    /// Its connection.
    pub(super) id: ClientId,
    object: QuireShellV1,
    ear: ShellEar,
}

impl ScriptedShell {
    /// Connects the shell and binds `quire_shell_v1` at version 5; the compositor then knows
    /// the shell can be asked.
    pub(super) fn start(fixture: &mut Fixture) -> ScriptedShell {
        let id = fixture.connect(ConnectionRole::Shell);
        let ear = ShellEar::default();
        let object = fixture
            .client(id)
            .bind::<QuireShellV1, _>(5..=5, ear.clone())
            .expect("the shell is offered quire_shell_v1 at version 5");
        fixture.pump();
        ScriptedShell { id, object, ear }
    }

    /// Every question heard so far, in order.
    pub(super) fn asked(&self) -> Vec<Asked> {
        self.ear.heard().asks.clone()
    }

    /// The questions taken back so far, in order.
    pub(super) fn withdrawn(&self) -> Vec<u32> {
        self.ear.heard().withdrawn.clone()
    }

    /// The id of the one question heard.
    pub(super) fn only_ask(&self) -> u32 {
        let asked = self.asked();
        assert_eq!(asked.len(), 1, "one question: {asked:?}");
        asked[0].ask
    }

    /// Answers question `ask` and lets the compositor act on it.
    pub(super) fn answer(&self, fixture: &mut Fixture, ask: u32, answer: Answer) {
        let wire = wire::Answer::try_from(answer.code()).expect("an answer on the wire");
        self.object.permission_answer(ask, wire);
        fixture.pump();
    }
}

/// The subject of the program `path`.
pub(super) fn program(path: &str) -> Subject {
    Subject::Program(ProgramPath::parse(path).expect("an absolute path"))
}

/// A book that has `verdict` on `power` for the program `path`.
pub(super) fn book_with(path: &str, power: Power, verdict: Verdict) -> Box<Book> {
    Box::new(Book::empty().with(Grant {
        power,
        subject: program(path),
        verdict,
        at: UnixSeconds(1),
    }))
}

/// What the person has kept about `power` for the program `path`.
pub(super) fn kept(fixture: &Fixture, path: &str, power: Power) -> Option<Verdict> {
    fixture
        .model()
        .privilege()
        .book()
        .get(&program(path), power)
}

/// A fixture, its scripted shell, and a client that is a program asked about its powers.
pub(super) fn asked_world() -> (Fixture, ScriptedShell, ClientId) {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let shell = ScriptedShell::start(&mut fixture);
    let client = fixture.connect_via_ask();
    fixture.pump();
    (fixture, shell, client)
}

/// The client `id` creates a virtual keyboard: its first use of the keys power.
fn use_keys(fixture: &mut Fixture, id: ClientId) -> VirtualBoard {
    let board = VirtualBoard::start(fixture.client(id).globals()).expect("a keyboard");
    fixture.pump();
    board
}

/// The first use of a power asks the shell once, naming the program and the power; an allow is
/// kept for the program, so a second connection of it is not asked again.
#[test]
fn the_shell_is_asked_once_and_the_answer_is_kept_for_the_program() {
    let (mut fixture, shell, client) = asked_world();
    let _board = use_keys(&mut fixture, client);
    let ask = shell.only_ask();
    assert_eq!(
        shell.asked()[0],
        Asked {
            ask,
            power: Power::Keys,
            kind: SubjectKind::Program,
            subject: ASKED_PROGRAM.to_owned(),
            memory: Memory::Remembered,
        }
    );
    shell.answer(&mut fixture, ask, Answer::Allow);
    assert_eq!(
        kept(&fixture, ASKED_PROGRAM, Power::Keys),
        Some(Verdict::Allow)
    );

    let again = fixture.connect_via_ask();
    let _second = use_keys(&mut fixture, again);
    assert_eq!(shell.asked().len(), 1, "the program is not asked again");
}

/// Don't Allow is kept too; dismissing the question is not.
#[test]
fn a_refusal_is_kept_and_a_dismissal_is_not() {
    let (mut fixture, shell, client) = asked_world();
    let _board = use_keys(&mut fixture, client);
    shell.answer(&mut fixture, shell.only_ask(), Answer::Deny);
    assert_eq!(
        kept(&fixture, ASKED_PROGRAM, Power::Keys),
        Some(Verdict::Deny)
    );

    let (mut fixture, shell, client) = asked_world();
    let _board = use_keys(&mut fixture, client);
    shell.answer(&mut fixture, shell.only_ask(), Answer::Dismissed);
    assert_eq!(kept(&fixture, ASKED_PROGRAM, Power::Keys), None);
}

/// An answer for a question nobody asked, or one already settled, changes nothing.
#[test]
fn an_answer_for_an_unknown_or_settled_question_is_ignored() {
    let (mut fixture, shell, client) = asked_world();
    shell.answer(&mut fixture, 4242, Answer::Allow);
    let _board = use_keys(&mut fixture, client);
    let ask = shell.only_ask();
    shell.answer(&mut fixture, ask, Answer::Deny);
    shell.answer(&mut fixture, ask, Answer::Allow);
    assert_eq!(
        kept(&fixture, ASKED_PROGRAM, Power::Keys),
        Some(Verdict::Deny)
    );
    assert!(fixture.client(shell.id).protocol_error().is_none());
}

/// A shell that binds after a program asked is shown the question then.
#[test]
fn a_shell_that_starts_late_is_shown_the_question_that_waits() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let client = fixture.connect_via_ask();
    fixture.pump();
    let _board = use_keys(&mut fixture, client);
    let shell = ScriptedShell::start(&mut fixture);
    assert_eq!(shell.asked().len(), 1);
    assert_eq!(shell.asked()[0].power, Power::Keys);
}

/// A question nobody answers is taken back after its time, and nothing is kept.
#[test]
fn an_unanswered_question_is_withdrawn_and_nothing_is_kept() {
    let (mut fixture, shell, client) = asked_world();
    let _board = use_keys(&mut fixture, client);
    let ask = shell.only_ask();
    fixture.advance(ASK_TTL + Duration::from_secs(1));
    assert_eq!(shell.withdrawn(), [ask]);
    assert_eq!(kept(&fixture, ASKED_PROGRAM, Power::Keys), None);
    assert!(fixture.model().privilege().asks().is_empty());
}

/// A shell that bound an older version cannot be asked: the program is refused after the wait
/// for a shell, and nothing is kept.
#[test]
fn a_shell_older_than_version_five_is_never_asked() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let id = fixture.connect(ConnectionRole::Shell);
    let _old = ShellHandle::bind(fixture.client(id), 4);
    fixture.pump();
    let client = fixture.connect_via_ask();
    fixture.pump();
    let _board = use_keys(&mut fixture, client);
    fixture.advance(SHELL_WAIT + Duration::from_secs(1));
    assert!(fixture.model().privilege().asks().is_empty());
    assert!(fixture.model().privilege().book().is_empty());
}

/// A program that executed another image after it connected is asked about as an unrecognised
/// one: the question says so (`this_time_only`) and nothing about it is kept.
#[test]
fn a_program_that_changed_its_image_is_asked_as_unrecognised() {
    #[derive(Debug)]
    struct Moved;
    impl casement_wayland::consent::ImageWatch for Moved {
        fn moved(&self) -> Option<ClientStanding> {
            Some(ClientStanding::Unverified {
                exe_name: Some("payload".to_owned()),
            })
        }
    }
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let shell = ScriptedShell::start(&mut fixture);
    let client = fixture.connect_watched(program_standing(ASKED_PROGRAM), Arc::new(Moved));
    fixture.pump();
    let _board = use_keys(&mut fixture, client);
    let asked = shell.asked();
    assert_eq!(asked.len(), 1);
    assert_eq!(asked[0].kind, SubjectKind::Unrecognised);
    assert_eq!(asked[0].subject, "payload");
    assert_eq!(asked[0].memory, Memory::ThisTimeOnly);
    shell.answer(&mut fixture, asked[0].ask, Answer::Allow);
    assert!(fixture.model().privilege().book().is_empty());
}

/// The interfaces the golden lists for `power`.
fn golden_of(power: Power) -> Vec<String> {
    let word = match power {
        Power::Screen => "screen",
        Power::Keys => "keys",
        Power::TypingHelp => "typing_help",
        Power::Clipboard => "clipboard",
        Power::Lock => "lock",
        Power::Displays => "displays",
        Power::Windows => "windows",
    };
    GOLDEN
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .filter_map(|line| {
            let fields: Vec<&str> = line.split_whitespace().collect();
            (fields.get(3) == Some(&word)).then(|| fields[0].to_owned())
        })
        .collect()
}

/// The interfaces `id` is advertised.
fn advertised(fixture: &mut Fixture, id: ClientId) -> Vec<String> {
    fixture
        .client(id)
        .globals()
        .contents()
        .clone_list()
        .into_iter()
        .map(|global| global.interface)
        .collect()
}

/// A program the person already refused is not shown the globals of that power, so its tools
/// say "not supported" instead of waiting for an answer that will not come; every other global
/// is shown, and a refusal that arrives while it is connected hides nothing more.
#[test]
fn the_globals_of_a_power_refused_before_connecting_are_not_advertised() {
    for power in <Power as WireCode>::ALL.iter().copied() {
        let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
        let everything = {
            let free = fixture.connect_via_ask();
            fixture.pump();
            advertised(&mut fixture, free)
        };
        fixture.feed_events(vec![Event::BookLoaded(book_with(
            ASKED_PROGRAM,
            power,
            Verdict::Deny,
        ))]);
        let refused = fixture.connect_via_ask();
        fixture.pump();
        let shown = advertised(&mut fixture, refused);
        let hidden = golden_of(power);
        assert!(!hidden.is_empty(), "{power:?} gates a global");
        assert!(
            hidden.iter().all(|interface| !shown.contains(interface)),
            "{power:?}: {shown:?}"
        );
        let expected: Vec<&String> = everything
            .iter()
            .filter(|interface| !hidden.contains(interface))
            .collect();
        assert_eq!(shown.iter().collect::<Vec<_>>(), expected, "{power:?}");
    }
}

/// A refusal that arrives while the program is connected leaves the globals advertised; its
/// uses are refused instead.
#[test]
fn a_refusal_that_arrives_later_hides_nothing() {
    let (mut fixture, _shell, client) = asked_world();
    let before = advertised(&mut fixture, client);
    fixture.feed_events(vec![Event::BookLoaded(book_with(
        ASKED_PROGRAM,
        Power::Keys,
        Verdict::Deny,
    ))]);
    assert_eq!(advertised(&mut fixture, client), before);
}
