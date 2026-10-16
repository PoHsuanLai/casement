//! `input_method` (P0-H7).

use crate::protocol::consent::{ScriptedShell, decided, kept};
use crate::protocol::text_input::Typing;
use casement_core::privilege::Verdict;
use casement_protocols::vocab::{Answer, Power};
use casement_testclient::ime::input_method::InputMethodEvent;
use casement_testclient::ime::text_input::TextInputEvent;
use casement_testkit::{ASKED_PROGRAM, Fixture, FixtureSpec};

/// The input method's commit_string reaches the focused text field: `commit_string` then `done`
/// carrying the serial of the field's own last `commit`.
#[test]
fn an_input_method_commits_to_the_focused_field() {
    let mut typing = Typing::start();
    typing.enable_and_grab();
    typing.ime.commit_string("你").expect("commit_string");
    typing.ime.commit_batch().expect("commit");
    typing.pump();
    let events: Vec<_> = typing
        .ime
        .field_events()
        .iter()
        .map(|line| line.event.clone())
        .collect();
    assert_eq!(
        events[events.len().saturating_sub(2)..],
        [
            TextInputEvent::CommitString(Some("你".to_owned())),
            TextInputEvent::Done(1)
        ]
    );
}

// ---- Per-app consent (`Power::TypingHelp`): smithay serves this protocol whole, so it holds an
// ---- input method made while the answer is out and the gate hook settles it.

/// A document window and an input method on one connection of a program that is asked about
/// its powers, with the scripted shell. With a `decision` the person had already decided
/// before the input method binds.
fn asked_typing(decision: Option<Verdict>) -> (Typing, ScriptedShell) {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let shell = ScriptedShell::start(&mut fixture);
    let client = fixture.connect_via_ask();
    fixture.pump();
    if let Some(verdict) = decision {
        decided(&mut fixture, Power::TypingHelp, verdict);
    }
    (Typing::start_in(fixture, client), shell)
}

/// Whether the input method has been sent `event`.
fn told(typing: &Typing, event: InputMethodEvent) -> bool {
    typing
        .ime
        .method_events()
        .iter()
        .any(|line| line.event == event)
}

/// The input method commits a string: whether the field hears it.
fn commits_reach_the_field(typing: &mut Typing) -> bool {
    typing.ime.commit_string("你").expect("commit_string");
    typing.ime.commit_batch().expect("commit");
    typing.pump();
    typing
        .ime
        .field_events()
        .iter()
        .any(|line| line.event == TextInputEvent::CommitString(Some("你".to_owned())))
}

/// An input method of a program the person already allowed works without a question.
#[test]
fn an_input_method_the_person_allowed_works_without_a_question() {
    let (mut typing, shell) = asked_typing(Some(Verdict::Allow));
    typing.enable_and_grab();
    assert!(shell.asked().is_empty(), "the book answers");
    assert!(commits_reach_the_field(&mut typing));
}

/// An input method of a program the person already refused is told `unavailable` and never
/// activated, without a question.
#[test]
fn an_input_method_the_person_refused_is_never_activated() {
    let (mut typing, shell) = asked_typing(Some(Verdict::Deny));
    typing.ime.enable().expect("enable");
    typing.pump();
    typing.pump();
    assert!(shell.asked().is_empty(), "the book answers");
    assert!(told(&typing, InputMethodEvent::Unavailable));
    assert!(!told(&typing, InputMethodEvent::Activate));
}

/// Asked, then allowed: the input method is activated for the field that was enabled while the
/// question was open, and its text reaches the field.
#[test]
fn an_input_method_made_while_asked_is_released_when_allowed() {
    let (mut typing, shell) = asked_typing(None);
    assert_eq!(shell.asked().len(), 1);
    assert_eq!(shell.asked()[0].power, Power::TypingHelp);
    typing.ime.enable().expect("enable");
    typing.pump();
    typing.pump();
    assert!(
        !told(&typing, InputMethodEvent::Activate),
        "nothing while it waits"
    );
    assert!(!told(&typing, InputMethodEvent::Unavailable));

    shell.answer(&mut typing.fixture, shell.only_ask(), Answer::Allow);
    typing.pump();
    typing.pump();
    assert!(told(&typing, InputMethodEvent::Activate));
    assert!(!told(&typing, InputMethodEvent::Unavailable));
    assert!(commits_reach_the_field(&mut typing));
    assert_eq!(
        kept(&typing.fixture, ASKED_PROGRAM, Power::TypingHelp),
        Some(Verdict::Allow)
    );
}

/// Asked, then refused: the input method is told `unavailable`, is never activated and its
/// text goes nowhere.
#[test]
fn an_input_method_made_while_asked_is_unavailable_when_refused() {
    let (mut typing, shell) = asked_typing(None);
    typing.ime.enable().expect("enable");
    typing.pump();
    shell.answer(&mut typing.fixture, shell.only_ask(), Answer::Deny);
    typing.pump();
    typing.pump();
    assert!(told(&typing, InputMethodEvent::Unavailable));
    assert!(!told(&typing, InputMethodEvent::Activate));
    assert!(!commits_reach_the_field(&mut typing));
    assert_eq!(
        kept(&typing.fixture, ASKED_PROGRAM, Power::TypingHelp),
        Some(Verdict::Deny)
    );
}

/// A power the person takes back ends the input method the program already had: it is told
/// `unavailable`. Waits for the smithay revoke hook (FINDINGS: pa2-cwl2).
#[test]
#[ignore = "pa2-cwl2: smithay revoke hook"]
fn an_input_method_the_person_switches_off_is_unavailable() {
    let (mut typing, _shell) = asked_typing(Some(Verdict::Allow));
    typing.ime.enable().expect("enable");
    typing.pump();
    typing.pump();
    assert!(told(&typing, InputMethodEvent::Activate));
    assert!(!told(&typing, InputMethodEvent::Unavailable));
    decided(&mut typing.fixture, Power::TypingHelp, Verdict::Deny);
    typing.pump();
    assert!(told(&typing, InputMethodEvent::Unavailable));
}

/// A client that hangs up while its input method waits is forgotten, and an answer that comes
/// after it is gone changes nothing for anyone.
#[test]
fn a_client_that_leaves_while_its_input_method_waits_is_forgotten() {
    let (mut typing, shell) = asked_typing(None);
    assert_eq!(typing.fixture.held_connections(), 1);
    typing.fixture.disconnect(typing.client);
    assert_eq!(typing.fixture.held_connections(), 0);
    shell.answer(&mut typing.fixture, shell.only_ask(), Answer::Allow);
    assert!(typing.fixture.client(shell.id).protocol_error().is_none());
}
