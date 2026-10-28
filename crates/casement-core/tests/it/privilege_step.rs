//! The consent machine through `Model::step`: asking once per program and power, no ask storm,
//! an answer kept or lasting one connection, a deadline, the lock, the shell coming and going,
//! Settings changes ending use at once, and the floor.

use crate::support::driver::fresh;
use crate::support::headless;
use casement_core::lock::LockEvent;
use casement_core::policy::{ConnectionRole, GlobalKind, advertised_to};
use casement_core::privilege::{
    ASK_TTL, ASKS_PER_MINUTE, AskSubject, Book, ClientStanding, Edit, Engine, Fallback, Floor,
    FloorRule, OPEN_ASKS, PowerOutcome, ProgramPath, REASK_AFTER, SHELL_WAIT, SandboxAppId,
    ServiceRole, Sheet, Subject, UnixSeconds, Verdict,
};
use casement_core::time::{Mono, TimerKey};
use casement_core::{ClientKey, Effect, Event, Model};
use casement_protocols::vocab::{Answer, AskId, Memory, Power, SubjectKind, WireCode};
use proptest::prelude::*;
use std::time::Duration;

const STAMP: UnixSeconds = UnixSeconds(1_760_000_000);

/// A model, the instant, and the checks that run after every step.
struct Desk {
    model: Model,
    at: Duration,
}

impl Desk {
    fn new() -> Desk {
        Desk {
            model: fresh(),
            at: Duration::ZERO,
        }
    }

    /// A desk whose shell is ready.
    fn with_shell() -> Desk {
        let mut desk = Desk::new();
        desk.send(Event::ShellReady);
        desk
    }

    fn send(&mut self, event: Event) -> Vec<Effect> {
        let (model, effects) = self.model.clone().step(event, Mono(self.at));
        assert_eq!(model.check_invariants(), Ok(()));
        self.model = model;
        effects
    }

    fn wait(&mut self, by: Duration) {
        self.at += by;
    }

    /// The timer for the next deadline fires, as the loop does when it is due.
    fn tick(&mut self) -> Vec<Effect> {
        self.send(Event::TimerFired(TimerKey::AskDeadline))
    }

    fn connect(&mut self, n: u64, role: ConnectionRole) -> Vec<Effect> {
        self.send(Event::ClientConnected {
            client: ClientKey(n),
            peer: porter_core::PeerIdentity::Unproven,
            role,
        })
    }

    /// Client `n` on the public socket, vouched for as `standing`.
    fn public(&mut self, n: u64, standing: ClientStanding) {
        self.connect(n, ConnectionRole::Public);
        self.send(Event::ClientStanding {
            client: ClientKey(n),
            standing,
        });
    }

    fn uses(&mut self, n: u64, power: Power) -> Vec<Effect> {
        self.send(Event::PrivilegeUse {
            client: ClientKey(n),
            power,
        })
    }

    fn answers(&mut self, ask: AskId, answer: Answer) -> Vec<Effect> {
        self.send(Event::PermissionAnswered {
            ask,
            answer,
            at: STAMP,
        })
    }

    /// The id of the question on screen.
    fn shown(&self) -> AskId {
        self.model
            .privilege()
            .asks()
            .iter()
            .find(|ask| ask.sheet == Sheet::Shown)
            .map(|ask| ask.id)
            .expect("a question is on screen")
    }
}

fn path(text: &str) -> ProgramPath {
    ProgramPath::parse(text).expect("a path")
}

fn program(text: &str) -> Subject {
    Subject::Program(path(text))
}

fn on_disk(text: &str) -> ClientStanding {
    ClientStanding::Program { exe: path(text) }
}

fn grim() -> ClientStanding {
    on_disk("/usr/bin/grim")
}

fn unverified() -> ClientStanding {
    ClientStanding::Unverified {
        exe_name: Some("grim".to_owned()),
    }
}

fn outcome(effects: &[Effect], n: u64, power: Power) -> Option<PowerOutcome> {
    effects.iter().rev().find_map(|effect| match effect {
        Effect::Resolve {
            client,
            power: got,
            outcome,
        } if *client == ClientKey(n) && *got == power => Some(*outcome),
        _ => None,
    })
}

fn questions(effects: &[Effect]) -> Vec<(AskId, Power, AskSubject, Memory)> {
    effects
        .iter()
        .filter_map(|effect| match effect {
            Effect::AskPermission {
                ask,
                power,
                subject,
                memory,
            } => Some((*ask, *power, subject.clone(), *memory)),
            _ => None,
        })
        .collect()
}

fn saved(effects: &[Effect]) -> Option<&Book> {
    effects.iter().find_map(|effect| match effect {
        Effect::SaveBook(book) => Some(&**book),
        _ => None,
    })
}

fn withdrawn(effects: &[Effect]) -> Vec<AskId> {
    effects
        .iter()
        .filter_map(|effect| match effect {
            Effect::WithdrawAsk { ask } => Some(*ask),
            _ => None,
        })
        .collect()
}

#[test]
fn the_shell_and_the_agent_tier_are_trusted_and_a_sandbox_is_not() {
    let mut desk = Desk::with_shell();
    desk.connect(1, ConnectionRole::Shell);
    desk.connect(2, ConnectionRole::Agent);
    desk.connect(
        3,
        ConnectionRole::Sandboxed(porter_core::SandboxFacts::Native),
    );
    for &power in <Power as WireCode>::ALL {
        assert_eq!(
            outcome(&desk.uses(1, power), 1, power),
            Some(PowerOutcome::Allowed)
        );
        assert_eq!(
            outcome(&desk.uses(2, power), 2, power),
            Some(PowerOutcome::Allowed)
        );
        assert_eq!(
            outcome(&desk.uses(3, power), 3, power),
            Some(PowerOutcome::Denied)
        );
    }
}

/// What the desktop protocol tells the shell about the program behind a window (QUIRE-12): the
/// image of a program, the app of a sandbox, the image of a service when it is known, and
/// nothing for anything that cannot be named or has no program of its own.
#[test]
fn the_subject_of_a_client_is_what_the_compositor_established() {
    let service = |exe: Option<ProgramPath>| ClientStanding::Service {
        role: ServiceRole::parse("portal").expect("a role"),
        exe,
    };
    let sandboxed = ClientStanding::Sandboxed {
        app: SandboxAppId::parse("org.gnome.Loupe").expect("an app id"),
        engine: Engine::Flatpak,
    };
    let mut desk = Desk::with_shell();
    desk.public(1, grim());
    desk.public(2, sandboxed);
    desk.public(3, service(Some(path("/usr/libexec/portal"))));
    desk.public(4, service(None));
    desk.public(5, unverified());
    desk.public(6, ClientStanding::unproven());
    desk.connect(7, ConnectionRole::Shell);
    desk.connect(
        8,
        ConnectionRole::Sandboxed(porter_core::SandboxFacts::Native),
    );
    let privilege = desk.model.privilege();
    let subjects: Vec<Option<&str>> = (1..=9)
        .map(|n| privilege.subject_of(ClientKey(n)))
        .collect();
    assert_eq!(
        subjects,
        [
            Some("/usr/bin/grim"),
            Some("org.gnome.Loupe"),
            Some("/usr/libexec/portal"),
            None,
            None,
            None,
            None,
            None,
            None,
        ]
    );
}

/// A client that becomes someone else is reported as that program from then on.
#[test]
fn a_client_reported_as_another_program_has_the_new_subject() {
    let mut desk = Desk::with_shell();
    desk.public(1, grim());
    desk.send(Event::ClientStanding {
        client: ClientKey(1),
        standing: unverified(),
    });
    assert_eq!(desk.model.privilege().subject_of(ClientKey(1)), None);
    desk.send(Event::ClientStanding {
        client: ClientKey(1),
        standing: on_disk("/usr/bin/wl-copy"),
    });
    assert_eq!(
        desk.model.privilege().subject_of(ClientKey(1)),
        Some("/usr/bin/wl-copy")
    );
}

#[test]
fn a_client_nobody_knows_is_refused() {
    let mut desk = Desk::with_shell();
    let effects = desk.uses(9, Power::Screen);
    assert_eq!(
        outcome(&effects, 9, Power::Screen),
        Some(PowerOutcome::Denied)
    );
}

#[test]
fn the_portal_is_trusted_for_the_screen_and_windows_and_asked_for_the_rest() {
    let mut desk = Desk::with_shell();
    let portal = ClientStanding::Service {
        role: ServiceRole::parse("portal").expect("a role"),
        exe: Some(path("/usr/libexec/transom")),
    };
    desk.public(1, portal);
    for power in [Power::Screen, Power::Windows] {
        let effects = desk.uses(1, power);
        assert_eq!(outcome(&effects, 1, power), Some(PowerOutcome::Allowed));
        assert!(questions(&effects).is_empty());
    }
    let effects = desk.uses(1, Power::Keys);
    assert_eq!(outcome(&effects, 1, Power::Keys), None);
    let asked = questions(&effects);
    assert_eq!(asked.len(), 1);
    assert_eq!(asked[0].2.text, "/usr/libexec/transom");
}

/// NEW-FA-2: the connection casement hands the bus-proven portal is trusted for the screen and the
/// windows at once, with no question, and for nothing else.
#[test]
fn a_handed_out_portal_connection_has_the_portals_standing_and_no_more() {
    let mut desk = Desk::with_shell();
    desk.connect(1, ConnectionRole::Portal);
    for power in [Power::Screen, Power::Windows] {
        let effects = desk.uses(1, power);
        assert_eq!(outcome(&effects, 1, power), Some(PowerOutcome::Allowed));
        assert!(questions(&effects).is_empty(), "{power:?}");
    }
    let effects = desk.uses(1, Power::Keys);
    assert_eq!(outcome(&effects, 1, Power::Keys), None, "keys wait");
    assert_eq!(questions(&effects).len(), 1);
}

/// NEW-FA-2: a portal image the library showed only as a program (`Proof::Exe` reaches the model as
/// a `Program`, not a `Service`) gets no trust: the screen and the windows are asked for like
/// any program's, and the one question names its image.
#[test]
fn the_portals_image_known_only_as_a_program_is_asked_for_the_screen_and_windows() {
    for power in [Power::Screen, Power::Windows] {
        let mut desk = Desk::with_shell();
        desk.public(
            1,
            ClientStanding::Program {
                exe: path("/usr/libexec/transom"),
            },
        );
        let effects = desk.uses(1, power);
        assert_eq!(outcome(&effects, 1, power), None, "{power:?} waits");
        let asked = questions(&effects);
        assert_eq!(asked.len(), 1, "{power:?}");
        assert_eq!(asked[0].2.text, "/usr/libexec/transom");
    }
}

#[test]
fn a_program_is_asked_once_and_the_answer_is_kept() {
    let mut desk = Desk::with_shell();
    desk.public(1, grim());
    let effects = desk.uses(1, Power::Screen);
    let asked = questions(&effects);
    assert_eq!(asked.len(), 1);
    let (ask, power, subject, memory) = asked[0].clone();
    assert_eq!(power, Power::Screen);
    assert_eq!(subject.kind, SubjectKind::Program);
    assert_eq!(subject.text, "/usr/bin/grim");
    assert_eq!(memory, Memory::Remembered);
    assert_eq!(
        outcome(&effects, 1, Power::Screen),
        None,
        "held until the person answers"
    );

    let effects = desk.answers(ask, Answer::Allow);
    assert_eq!(
        outcome(&effects, 1, Power::Screen),
        Some(PowerOutcome::Allowed)
    );
    let book = saved(&effects).expect("the book is saved");
    assert_eq!(
        book.get(&program("/usr/bin/grim"), Power::Screen),
        Some(Verdict::Allow)
    );

    // The same program on another connection is let in without a question.
    desk.public(2, grim());
    let effects = desk.uses(2, Power::Screen);
    assert_eq!(
        outcome(&effects, 2, Power::Screen),
        Some(PowerOutcome::Allowed)
    );
    assert!(questions(&effects).is_empty());
}

#[test]
fn clients_of_one_program_share_one_question() {
    let mut desk = Desk::with_shell();
    for n in 1..=3 {
        desk.public(n, grim());
    }
    let all: Vec<Effect> = (1..=3).flat_map(|n| desk.uses(n, Power::Screen)).collect();
    assert_eq!(questions(&all).len(), 1);
    let views = desk.model.privilege().asks();
    assert_eq!(views.len(), 1);
    assert_eq!(views[0].waiting, 3);

    let effects = desk.answers(desk.shown(), Answer::Allow);
    for n in 1..=3 {
        assert_eq!(
            outcome(&effects, n, Power::Screen),
            Some(PowerOutcome::Allowed)
        );
    }
    assert!(desk.model.privilege().asks().is_empty());
}

#[test]
fn a_refusal_is_kept_too() {
    let mut desk = Desk::with_shell();
    desk.public(1, grim());
    desk.uses(1, Power::Clipboard);
    let effects = desk.answers(desk.shown(), Answer::Deny);
    assert_eq!(
        outcome(&effects, 1, Power::Clipboard),
        Some(PowerOutcome::Denied)
    );
    assert!(saved(&effects).is_some());
    let effects = desk.uses(1, Power::Clipboard);
    assert_eq!(
        outcome(&effects, 1, Power::Clipboard),
        Some(PowerOutcome::Denied)
    );
    assert!(questions(&effects).is_empty());
}

#[test]
fn a_program_that_cannot_be_recognised_gets_allow_once() {
    let mut desk = Desk::with_shell();
    desk.public(1, unverified());
    desk.public(2, unverified());
    let effects = desk.uses(1, Power::Screen);
    let asked = questions(&effects);
    assert_eq!(asked[0].2.kind, SubjectKind::Unrecognised);
    assert_eq!(asked[0].2.text, "grim");
    assert_eq!(asked[0].3, Memory::ThisTimeOnly);

    let effects = desk.answers(asked[0].0, Answer::Allow);
    assert_eq!(
        outcome(&effects, 1, Power::Screen),
        Some(PowerOutcome::Allowed)
    );
    assert!(
        saved(&effects).is_none(),
        "nothing is written for a program that is not recognised"
    );
    assert!(desk.model.privilege().book().is_empty());

    // The same connection keeps the power while it lives; another connection is asked itself.
    let again = desk.uses(1, Power::Screen);
    assert_eq!(
        outcome(&again, 1, Power::Screen),
        Some(PowerOutcome::Allowed)
    );
    let other = desk.uses(2, Power::Screen);
    assert_eq!(
        questions(&other).len(),
        1,
        "a different connection is a different question"
    );

    // A new connection of the same program starts with nothing.
    desk.send(Event::ClientGone {
        client: ClientKey(1),
    });
    desk.public(3, unverified());
    assert_eq!(
        outcome(&desk.uses(3, Power::Screen), 3, Power::Screen),
        None
    );
}

#[test]
fn an_interpreter_is_asked_for_its_connection_only() {
    let mut desk = Desk::with_shell();
    desk.public(1, on_disk("/usr/bin/python3.13"));
    let effects = desk.uses(1, Power::Keys);
    let asked = questions(&effects);
    assert_eq!(asked[0].2.kind, SubjectKind::Program);
    assert_eq!(asked[0].3, Memory::ThisTimeOnly);
    let effects = desk.answers(asked[0].0, Answer::Allow);
    assert!(saved(&effects).is_none());
    assert_eq!(
        outcome(&effects, 1, Power::Keys),
        Some(PowerOutcome::Allowed)
    );
}

#[test]
fn a_dismissed_question_is_not_asked_again_for_a_while() {
    let mut desk = Desk::with_shell();
    desk.public(1, grim());
    desk.uses(1, Power::Screen);
    let effects = desk.answers(desk.shown(), Answer::Dismissed);
    assert_eq!(
        outcome(&effects, 1, Power::Screen),
        Some(PowerOutcome::Denied)
    );
    assert!(saved(&effects).is_none());

    desk.wait(Duration::from_secs(60));
    let effects = desk.uses(1, Power::Screen);
    assert_eq!(
        outcome(&effects, 1, Power::Screen),
        Some(PowerOutcome::Denied)
    );
    assert!(questions(&effects).is_empty(), "no nagging");

    desk.wait(REASK_AFTER);
    let effects = desk.uses(1, Power::Screen);
    assert_eq!(questions(&effects).len(), 1, "asked again after the pause");
}

/// NEW-CAS-5: the pause after "Don't Allow" belongs to the program, not to the connection it
/// was on: reconnecting does not bring the question back, whether the image has a name or not.
#[test]
fn a_program_that_reconnects_after_a_refusal_is_not_asked_again() {
    let nameless = ClientStanding::Unverified { exe_name: None };
    for (name, standing) in [("a named image", unverified()), ("no name", nameless)] {
        let mut desk = Desk::with_shell();
        desk.public(1, standing.clone());
        desk.uses(1, Power::Screen);
        desk.answers(desk.shown(), Answer::Deny);
        desk.send(Event::ClientGone {
            client: ClientKey(1),
        });

        desk.public(2, standing.clone());
        let effects = desk.uses(2, Power::Screen);
        assert!(questions(&effects).is_empty(), "{name}: no question again");
        assert_eq!(
            outcome(&effects, 2, Power::Screen),
            Some(PowerOutcome::Denied),
            "{name}"
        );
        // Another power is a different question, and so is a program of another name.
        assert_eq!(
            questions(&desk.uses(2, Power::Keys)).len(),
            1,
            "{name}: another power"
        );

        desk.wait(REASK_AFTER);
        desk.public(3, standing);
        assert_eq!(
            questions(&desk.uses(3, Power::Screen)).len(),
            1,
            "{name}: asked again after the pause"
        );
    }
    let mut desk = Desk::with_shell();
    desk.public(1, unverified());
    desk.uses(1, Power::Screen);
    desk.answers(desk.shown(), Answer::Deny);
    desk.public(
        2,
        ClientStanding::Unverified {
            exe_name: Some("payload".to_owned()),
        },
    );
    assert_eq!(
        questions(&desk.uses(2, Power::Screen)).len(),
        1,
        "a program of another name is asked"
    );
}

#[test]
fn a_question_nobody_answers_times_out_and_a_late_answer_changes_nothing() {
    let mut desk = Desk::with_shell();
    desk.public(1, grim());
    let effects = desk.uses(1, Power::Screen);
    let (ask, ..) = questions(&effects)[0].clone();
    let due = effects
        .iter()
        .find_map(|effect| match effect {
            Effect::ArmTimer(due) if due.key == TimerKey::AskDeadline => Some(due.clone()),
            _ => None,
        })
        .expect("a timer wakes the machine at the deadline");
    assert_eq!(due.at, Mono(desk.at + ASK_TTL));

    desk.wait(ASK_TTL);
    let effects = desk.tick();
    assert_eq!(withdrawn(&effects), vec![ask]);
    assert_eq!(
        outcome(&effects, 1, Power::Screen),
        Some(PowerOutcome::Denied)
    );
    assert!(desk.model.privilege().asks().is_empty());
    assert!(!desk.model.timers().is_armed(&TimerKey::AskDeadline));

    let late = desk.answers(ask, Answer::Allow);
    assert!(late.is_empty(), "the question is gone");
    assert!(desk.model.privilege().book().is_empty());
}

#[test]
fn an_answer_that_is_not_for_a_question_on_screen_is_ignored() {
    let mut desk = Desk::with_shell();
    desk.public(1, grim());
    desk.public(2, on_disk("/usr/bin/wtype"));
    desk.uses(1, Power::Screen);
    desk.uses(2, Power::Keys);
    let queued = desk
        .model
        .privilege()
        .asks()
        .into_iter()
        .find(|ask| ask.sheet == Sheet::Queued)
        .expect("a second question waits")
        .id;
    assert!(
        desk.answers(queued, Answer::Allow).is_empty(),
        "not on screen yet"
    );
    assert!(
        desk.answers(AskId::new(999), Answer::Allow).is_empty(),
        "never asked"
    );

    // The first answer brings the second question up.
    let effects = desk.answers(desk.shown(), Answer::Deny);
    assert_eq!(questions(&effects).len(), 1);
    assert_eq!(questions(&effects)[0].0, queued);
}

#[test]
fn nothing_is_asked_until_a_shell_can_ask_and_not_after_the_wait() {
    let mut desk = Desk::new();
    desk.public(1, grim());
    let effects = desk.uses(1, Power::Screen);
    assert!(questions(&effects).is_empty(), "no shell yet");
    assert_eq!(outcome(&effects, 1, Power::Screen), None, "held");

    let effects = desk.send(Event::ShellReady);
    assert_eq!(questions(&effects).len(), 1, "the shell arrived in time");

    // A desk whose shell never comes.
    let mut late = Desk::new();
    late.public(1, grim());
    late.wait(SHELL_WAIT);
    let effects = late.uses(1, Power::Screen);
    assert_eq!(
        outcome(&effects, 1, Power::Screen),
        Some(PowerOutcome::Denied)
    );
    assert!(late.model.privilege().asks().is_empty());
}

#[test]
fn a_held_question_is_dropped_when_the_shell_never_comes() {
    let mut desk = Desk::new();
    desk.public(1, grim());
    desk.uses(1, Power::Screen);
    desk.wait(SHELL_WAIT);
    let effects = desk.tick();
    assert_eq!(
        outcome(&effects, 1, Power::Screen),
        Some(PowerOutcome::Denied)
    );
    assert!(desk.model.privilege().asks().is_empty());
}

#[test]
fn a_question_on_a_shell_that_went_waits_for_the_next_one() {
    let mut desk = Desk::with_shell();
    desk.public(1, grim());
    let effects = desk.uses(1, Power::Screen);
    let (ask, ..) = questions(&effects)[0].clone();
    desk.send(Event::ShellGone);
    assert_eq!(desk.model.privilege().asks()[0].sheet, Sheet::Queued);
    let effects = desk.send(Event::ShellReady);
    assert_eq!(
        questions(&effects).iter().map(|q| q.0).collect::<Vec<_>>(),
        vec![ask]
    );
}

#[test]
fn the_lock_screen_is_not_a_place_to_ask() {
    let mut desk = Desk::with_shell();
    desk.send(Event::OutputPlugged(headless("HEADLESS-1")));
    desk.public(1, grim());
    desk.send(Event::Lock(LockEvent::Requested {
        client: ClientKey(7),
    }));
    let effects = desk.uses(1, Power::Screen);
    assert_eq!(
        outcome(&effects, 1, Power::Screen),
        Some(PowerOutcome::Denied)
    );
    assert!(questions(&effects).is_empty());
    assert!(desk.model.privilege().asks().is_empty());
}

#[test]
fn a_question_stays_open_when_its_client_goes() {
    let mut desk = Desk::with_shell();
    desk.public(1, grim());
    desk.uses(1, Power::Screen);
    desk.send(Event::ClientGone {
        client: ClientKey(1),
    });
    let views = desk.model.privilege().asks();
    assert_eq!(views.len(), 1);
    assert_eq!(views[0].waiting, 0);
    let effects = desk.answers(desk.shown(), Answer::Allow);
    assert!(saved(&effects).is_some(), "the answer is for next time");
    desk.public(2, grim());
    assert_eq!(
        outcome(&desk.uses(2, Power::Screen), 2, Power::Screen),
        Some(PowerOutcome::Allowed)
    );
}

#[test]
fn repeated_use_does_not_make_more_questions() {
    let mut desk = Desk::with_shell();
    desk.public(1, grim());
    let all: Vec<Effect> = (0..50).flat_map(|_| desk.uses(1, Power::Screen)).collect();
    assert_eq!(questions(&all).len(), 1);
    assert_eq!(desk.model.privilege().asks().len(), 1);
}

#[test]
fn a_client_cycling_through_powers_is_cut_off() {
    let mut desk = Desk::with_shell();
    desk.public(1, grim());
    let powers = <Power as WireCode>::ALL;
    for &power in &powers[..ASKS_PER_MINUTE] {
        assert_eq!(outcome(&desk.uses(1, power), 1, power), None);
    }
    let next = powers[ASKS_PER_MINUTE];
    assert_eq!(
        outcome(&desk.uses(1, next), 1, next),
        Some(PowerOutcome::Denied)
    );
    assert_eq!(desk.model.privilege().asks().len(), ASKS_PER_MINUTE);

    desk.wait(Duration::from_secs(61));
    // A minute later the same client may ask for another power.
    desk.answers(desk.shown(), Answer::Dismissed);
    assert_ne!(
        outcome(&desk.uses(1, next), 1, next),
        Some(PowerOutcome::Denied)
    );
}

#[test]
fn only_a_few_questions_are_open_at_once() {
    let mut desk = Desk::with_shell();
    for n in 1..=(OPEN_ASKS as u64 + 1) {
        desk.public(n, unverified());
    }
    for n in 1..=OPEN_ASKS as u64 {
        assert_eq!(
            outcome(&desk.uses(n, Power::Screen), n, Power::Screen),
            None
        );
    }
    let over = OPEN_ASKS as u64 + 1;
    assert_eq!(
        outcome(&desk.uses(over, Power::Screen), over, Power::Screen),
        Some(PowerOutcome::Denied)
    );
    assert_eq!(desk.model.privilege().asks().len(), OPEN_ASKS);
}

#[test]
fn switching_a_program_off_ends_its_use_at_once() {
    let mut desk = Desk::with_shell();
    desk.public(1, grim());
    desk.public(2, grim());
    desk.public(3, on_disk("/usr/bin/wtype"));
    desk.uses(1, Power::Screen);
    desk.answers(desk.shown(), Answer::Allow);
    desk.uses(2, Power::Screen);
    desk.uses(3, Power::Screen);
    desk.answers(desk.shown(), Answer::Allow);

    let effects = desk.send(Event::PrivilegeEdited {
        edit: Edit::Deny(program("/usr/bin/grim"), Power::Screen),
        at: STAMP,
    });
    assert_eq!(
        outcome(&effects, 1, Power::Screen),
        Some(PowerOutcome::Revoked)
    );
    assert_eq!(
        outcome(&effects, 2, Power::Screen),
        Some(PowerOutcome::Revoked)
    );
    assert_eq!(
        outcome(&effects, 3, Power::Screen),
        None,
        "another program keeps its answer"
    );
    let book = saved(&effects).expect("the book is saved");
    assert_eq!(
        book.get(&program("/usr/bin/grim"), Power::Screen),
        Some(Verdict::Deny)
    );

    assert_eq!(
        outcome(&desk.uses(1, Power::Screen), 1, Power::Screen),
        Some(PowerOutcome::Denied)
    );
    desk.public(4, grim());
    assert_eq!(
        outcome(&desk.uses(4, Power::Screen), 4, Power::Screen),
        Some(PowerOutcome::Denied)
    );
}

#[test]
fn switching_a_program_on_lets_it_in_and_forgetting_asks_again() {
    let mut desk = Desk::with_shell();
    desk.public(1, grim());
    desk.uses(1, Power::Keys);
    // Settings switches it on while the question is open: the question is taken back.
    let effects = desk.send(Event::PrivilegeEdited {
        edit: Edit::Allow(program("/usr/bin/grim"), Power::Keys),
        at: STAMP,
    });
    assert_eq!(
        outcome(&effects, 1, Power::Keys),
        Some(PowerOutcome::Allowed)
    );
    assert_eq!(withdrawn(&effects).len(), 1);
    assert!(desk.model.privilege().asks().is_empty());

    let effects = desk.send(Event::PrivilegeEdited {
        edit: Edit::Forget(program("/usr/bin/grim"), Power::Keys),
        at: STAMP,
    });
    assert_eq!(
        outcome(&effects, 1, Power::Keys),
        Some(PowerOutcome::Revoked)
    );
    assert!(desk.model.privilege().book().is_empty());
    assert_eq!(questions(&desk.uses(1, Power::Keys)).len(), 1);
}

#[test]
fn switching_off_an_allow_once_ends_it_too() {
    let mut desk = Desk::with_shell();
    desk.public(1, on_disk("/usr/bin/python3"));
    desk.uses(1, Power::Keys);
    desk.answers(desk.shown(), Answer::Allow);
    let effects = desk.send(Event::PrivilegeEdited {
        edit: Edit::Deny(program("/usr/bin/python3"), Power::Keys),
        at: STAMP,
    });
    assert_eq!(
        outcome(&effects, 1, Power::Keys),
        Some(PowerOutcome::Revoked)
    );
    assert!(
        saved(&effects).is_none(),
        "an interpreter is never written to the book"
    );
}

#[test]
fn a_client_that_turns_out_to_be_another_program_loses_what_it_had() {
    let mut desk = Desk::with_shell();
    desk.public(1, grim());
    desk.uses(1, Power::Screen);
    desk.answers(desk.shown(), Answer::Allow);
    desk.uses(1, Power::Keys);
    let effects = desk.send(Event::ClientStanding {
        client: ClientKey(1),
        standing: ClientStanding::Unverified { exe_name: None },
    });
    assert_eq!(
        outcome(&effects, 1, Power::Screen),
        Some(PowerOutcome::Revoked)
    );
    assert_eq!(
        outcome(&effects, 1, Power::Keys),
        Some(PowerOutcome::Denied)
    );
    let effects = desk.uses(1, Power::Screen);
    assert_eq!(
        outcome(&effects, 1, Power::Screen),
        None,
        "now asked as a stranger"
    );
    assert_eq!(desk.model.privilege().asks().len(), 2);
}

fn floor_denying_screen_to_grim() -> Floor {
    Floor::empty().with_rule(FloorRule {
        power: Power::Screen,
        subject: program("/usr/bin/grim"),
        verdict: Verdict::Deny,
    })
}

#[test]
fn the_administrators_refusal_wins_and_hides_the_global() {
    let mut desk = Desk::with_shell();
    desk.send(Event::FloorLoaded(Box::new(floor_denying_screen_to_grim())));
    desk.public(1, grim());
    let effects = desk.uses(1, Power::Screen);
    assert_eq!(
        outcome(&effects, 1, Power::Screen),
        Some(PowerOutcome::Denied)
    );
    assert!(questions(&effects).is_empty());

    let hidden = desk
        .model
        .privilege()
        .hidden_for(&ConnectionRole::Public, &grim());
    assert!(hidden.contains(Power::Screen));
    assert_eq!(hidden.iter().count(), 1);
    assert!(!advertised_to(
        GlobalKind::ImageCapture,
        &ConnectionRole::Public,
        &hidden
    ));
    assert!(advertised_to(
        GlobalKind::VirtualKeyboard,
        &ConnectionRole::Public,
        &hidden
    ));
    let stranger = desk
        .model
        .privilege()
        .hidden_for(&ConnectionRole::Public, &unverified());
    assert!(stranger.is_empty());
}

#[test]
fn a_floor_loaded_later_takes_the_power_from_a_program_that_had_it() {
    let mut desk = Desk::with_shell();
    desk.public(1, grim());
    desk.uses(1, Power::Screen);
    desk.answers(desk.shown(), Answer::Allow);
    let effects = desk.send(Event::FloorLoaded(Box::new(floor_denying_screen_to_grim())));
    assert_eq!(
        outcome(&effects, 1, Power::Screen),
        Some(PowerOutcome::Revoked)
    );
}

#[test]
fn a_floor_default_of_deny_refuses_without_asking_and_a_pre_approval_lets_in() {
    let mut desk = Desk::with_shell();
    let floor = Floor::empty()
        .with_fallback(Power::Lock, Fallback::Deny)
        .with_rule(FloorRule {
            power: Power::Clipboard,
            subject: program("/usr/bin/cliphist"),
            verdict: Verdict::Allow,
        });
    desk.send(Event::FloorLoaded(Box::new(floor)));
    desk.public(1, grim());
    desk.public(2, on_disk("/usr/bin/cliphist"));
    assert_eq!(
        outcome(&desk.uses(1, Power::Lock), 1, Power::Lock),
        Some(PowerOutcome::Denied)
    );
    assert_eq!(
        outcome(&desk.uses(2, Power::Clipboard), 2, Power::Clipboard),
        Some(PowerOutcome::Allowed)
    );
    // A person can still say no to a pre-approval.
    let effects = desk.send(Event::PrivilegeEdited {
        edit: Edit::Deny(program("/usr/bin/cliphist"), Power::Clipboard),
        at: STAMP,
    });
    assert_eq!(
        outcome(&effects, 2, Power::Clipboard),
        Some(PowerOutcome::Revoked)
    );
}

#[test]
fn the_book_read_at_start_is_obeyed() {
    let mut desk = Desk::with_shell();
    let text = "version = 1\n[[grant]]\npower = \"screen\"\nanswer = \"allow\"\nat = 1\nsubject = { program = \"/usr/bin/grim\" }\n";
    let book = Book::from_toml(text).expect("reads").value;
    desk.send(Event::BookLoaded(Box::new(book)));
    desk.public(1, grim());
    assert_eq!(
        outcome(&desk.uses(1, Power::Screen), 1, Power::Screen),
        Some(PowerOutcome::Allowed)
    );
    assert_eq!(desk.model.privilege().rows().len(), 1);
}

mod storms {
    use super::*;

    #[derive(Debug, Clone)]
    enum Act {
        Use(u64, Power),
        Answer(u32, Answer),
        Wait(u64),
        Tick,
        ShellReady,
        ShellGone,
        Gone(u64),
        Revoke(usize, Power),
        Forget(usize, Power),
        Grant(usize, Power),
    }

    const PROGRAMS: [&str; 3] = ["/usr/bin/grim", "/usr/bin/wtype", "/usr/bin/python3"];

    fn standing_of(n: u64) -> ClientStanding {
        match n % 4 {
            0 => unverified(),
            k => on_disk(PROGRAMS[(k - 1) as usize]),
        }
    }

    fn any_power() -> impl Strategy<Value = Power> {
        proptest::sample::select(<Power as WireCode>::ALL.to_vec())
    }

    fn any_answer() -> impl Strategy<Value = Answer> {
        prop_oneof![
            Just(Answer::Allow),
            Just(Answer::Deny),
            Just(Answer::Dismissed)
        ]
    }

    fn any_act() -> impl Strategy<Value = Act> {
        prop_oneof![
            4 => (1u64..=6, any_power()).prop_map(|(n, power)| Act::Use(n, power)),
            2 => (1u32..8, any_answer()).prop_map(|(ask, answer)| Act::Answer(ask, answer)),
            2 => (0u64..200).prop_map(Act::Wait),
            1 => Just(Act::Tick),
            1 => Just(Act::ShellReady),
            1 => Just(Act::ShellGone),
            1 => (1u64..=6).prop_map(Act::Gone),
            1 => (0usize..3, any_power()).prop_map(|(p, power)| Act::Revoke(p, power)),
            1 => (0usize..3, any_power()).prop_map(|(p, power)| Act::Forget(p, power)),
            1 => (0usize..3, any_power()).prop_map(|(p, power)| Act::Grant(p, power)),
        ]
    }

    fn event_of(act: &Act) -> Option<Event> {
        let subject = |p: &usize| program(PROGRAMS[*p]);
        Some(match act {
            Act::Use(n, power) => Event::PrivilegeUse {
                client: ClientKey(*n),
                power: *power,
            },
            Act::Answer(ask, answer) => Event::PermissionAnswered {
                ask: AskId::new(*ask),
                answer: *answer,
                at: STAMP,
            },
            Act::Tick => Event::TimerFired(TimerKey::AskDeadline),
            Act::ShellReady => Event::ShellReady,
            Act::ShellGone => Event::ShellGone,
            Act::Gone(n) => Event::ClientGone {
                client: ClientKey(*n),
            },
            Act::Revoke(p, power) => Event::PrivilegeEdited {
                edit: Edit::Deny(subject(p), *power),
                at: STAMP,
            },
            Act::Forget(p, power) => Event::PrivilegeEdited {
                edit: Edit::Forget(subject(p), *power),
                at: STAMP,
            },
            Act::Grant(p, power) => Event::PrivilegeEdited {
                edit: Edit::Allow(subject(p), *power),
                at: STAMP,
            },
            Act::Wait(_) => return None,
        })
    }

    fn started() -> Desk {
        let mut desk = Desk::new();
        desk.send(Event::FloorLoaded(Box::new(floor_denying_screen_to_grim())));
        for n in 1..=6 {
            desk.public(n, standing_of(n));
        }
        desk
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(400))]

        /// Whatever happens in whatever order: the machine stays in step, at most one question
        /// is put to the shell at a time, no question is put twice at once, the administrator's
        /// refusal is never overruled, and nothing is allowed for a refused ask.
        #[test]
        fn nothing_storms_and_a_refusal_holds(acts in proptest::collection::vec(any_act(), 1..80)) {
            let mut desk = started();
            for act in acts {
                let effects = match event_of(&act) {
                    Some(event) => desk.send(event),
                    None => {
                        if let Act::Wait(secs) = act {
                            desk.wait(Duration::from_secs(secs));
                        }
                        Vec::new()
                    }
                };
                prop_assert!(questions(&effects).len() <= 1, "{:?}", act);
                let shown = desk.model.privilege().asks().iter()
                    .filter(|ask| ask.sheet == Sheet::Shown).count();
                prop_assert!(shown <= 1);
                prop_assert!(desk.model.privilege().asks().len() <= OPEN_ASKS);
                for n in [1u64, 5] {
                    // Clients 1 and 5 are grim (n % 4 == 1): the floor refuses it the screen.
                    prop_assert_ne!(
                        outcome(&effects, n, Power::Screen),
                        Some(PowerOutcome::Allowed),
                        "{:?}", act
                    );
                }
            }
        }

        /// Using powers over and over, with nobody answering, puts one question on screen and
        /// lets nothing in.
        #[test]
        fn using_without_answers_asks_once_and_allows_nothing(
            uses in proptest::collection::vec((1u64..=6, any_power()), 1..120)
        ) {
            let mut desk = Desk::with_shell();
            for n in 1..=6 {
                desk.public(n, standing_of(n));
            }
            let all: Vec<Effect> = uses
                .into_iter()
                .flat_map(|(n, power)| desk.uses(n, power))
                .collect();
            prop_assert!(questions(&all).len() <= 1);
            let allowed = all.iter().any(|effect| matches!(
                effect,
                Effect::Resolve { outcome: PowerOutcome::Allowed, .. }
            ));
            prop_assert!(!allowed);
            prop_assert!(desk.model.privilege().asks().len() <= OPEN_ASKS);
        }

        /// Once a program is switched off, no connection of it gets the power again until it is
        /// switched on.
        #[test]
        fn revoking_ends_use(power in any_power(), uses in proptest::collection::vec(1u64..=6, 1..12)) {
            let mut desk = Desk::with_shell();
            for n in 1..=6 {
                desk.public(n, standing_of(n));
            }
            desk.send(Event::PrivilegeEdited { edit: Edit::Allow(program(PROGRAMS[1]), power), at: STAMP });
            desk.send(Event::PrivilegeEdited { edit: Edit::Deny(program(PROGRAMS[1]), power), at: STAMP });
            for n in uses.into_iter().filter(|n| n % 4 == 2) {
                let effects = desk.uses(n, power);
                prop_assert_eq!(outcome(&effects, n, power), Some(PowerOutcome::Denied));
            }
        }
    }
}
