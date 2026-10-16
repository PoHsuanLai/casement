//! `foreign_toplevel_list` (P0-H8). Runs once the fixture and test client core land.

use crate::protocol::consent::{ScriptedShell, asked_world, decided, kept};
use crate::protocol::wlr_toplevel::map_titled;
use crate::protocol::xdg_shell::windows;
use casement_core::geometry::PhysicalSize;
use casement_core::policy::ConnectionRole;
use casement_core::privilege::Verdict;
use casement_protocols::vocab::{Answer, Power};
use casement_testclient::manage::foreign_toplevel_list::{Listed, Listing, ToplevelList};
use casement_testclient::surface::toplevel::{Toplevel, WmBase};
use casement_testclient::{Pattern, ShmBuffer, shm_buffer};
use casement_testkit::{ASKED_PROGRAM, ClientId, Fixture, FixtureSpec};

/// Each mapped window appears with title and app id; a new title is a second `done`; closing it
/// sends closed.
#[test]
fn windows_are_listed_and_closed() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let watcher = fixture.connect(ConnectionRole::Public);
    fixture.pump();
    let mut list = ToplevelList::start(fixture.client(watcher).globals()).expect("the list");
    fixture.pump();
    list.pump().expect("pump");
    assert!(list.listed().is_empty(), "no window yet");

    let owner = fixture.connect(ConnectionRole::Public);
    fixture.pump();
    let base = WmBase::bind(fixture.client(owner), 7);
    let top = base.toplevel(fixture.client(owner));
    top.set_title("Inbox");
    top.set_app_id("org.example.Mail");
    top.commit_bare();
    fixture.pump();
    top.ack_latest();
    let buffer = shm_buffer(
        fixture.client(owner),
        PhysicalSize { w: 320, h: 200 },
        Pattern::Gradient,
    );
    top.show(&buffer);
    fixture.pump();
    list.pump().expect("pump");
    let window = windows(&fixture)[0];
    let listed = |title: &str, listing: Listing| Listed {
        identifier: format!("w{}", window.0),
        title: title.to_owned(),
        app_id: "org.example.Mail".to_owned(),
        listing,
    };
    assert_eq!(list.listed(), [listed("Inbox", Listing::Open)]);

    top.set_title("Drafts");
    top.surface.commit();
    fixture.pump();
    list.pump().expect("pump");
    assert_eq!(
        list.listed(),
        [listed("Drafts", Listing::Open)],
        "a new title is announced under the same identifier"
    );

    top.destroy();
    fixture.pump();
    list.pump().expect("pump");
    assert_eq!(list.listed(), [listed("Drafts", Listing::Closed)]);
}

// ---- Per-app consent (`Power::Windows`): smithay serves this protocol whole, so it holds a
// ---- list bound while the answer is out and the gate hook settles it.

/// The desktop with the scripted shell, a window "Inbox" mapped by a client the desktop trusts
/// (what must stay alive comes back), and a program that is asked about its powers.
fn asked_with_window() -> (Fixture, ScriptedShell, ClientId, Toplevel, ShmBuffer) {
    let (mut fixture, shell, asked) = asked_world();
    let owner = fixture.connect(ConnectionRole::Public);
    fixture.pump();
    let base = WmBase::bind(fixture.client(owner), 7);
    let (top, buffer) = map_titled(&mut fixture, owner, &base, "Inbox");
    (fixture, shell, asked, top, buffer)
}

/// Binds a list on `id` and reads what the compositor sent it.
fn watch(fixture: &mut Fixture, id: ClientId) -> ToplevelList {
    let mut list = ToplevelList::start(fixture.client(id).globals()).expect("the list");
    settle(fixture, &mut list);
    list
}

/// Pumps the compositor and the list client until both are quiet.
fn settle(fixture: &mut Fixture, list: &mut ToplevelList) {
    for _ in 0..2 {
        fixture.pump();
        list.pump().expect("pump");
    }
}

/// The titles a list shows.
fn titles(list: &ToplevelList) -> Vec<String> {
    list.listed()
        .into_iter()
        .map(|listed| listed.title)
        .collect()
}

/// A program the person already allowed is listed the windows without a question.
#[test]
fn a_list_the_person_allowed_works_without_a_question() {
    let (mut fixture, shell, asked, top, _buffer) = asked_with_window();
    decided(&mut fixture, Power::Windows, Verdict::Allow);
    let mut list = watch(&mut fixture, asked);
    assert!(shell.asked().is_empty(), "the book answers");
    assert_eq!(titles(&list), ["Inbox"]);
    top.set_title("Drafts");
    top.surface.commit();
    settle(&mut fixture, &mut list);
    assert_eq!(titles(&list), ["Drafts"], "and kept up to date");
}

/// A program the person already refused gets a finished list and never a window, without
/// being asked again.
#[test]
fn a_list_the_person_refused_is_finished_and_empty() {
    let (mut fixture, shell, asked, top, _buffer) = asked_with_window();
    decided(&mut fixture, Power::Windows, Verdict::Deny);
    let mut list = watch(&mut fixture, asked);
    assert!(shell.asked().is_empty(), "the book answers");
    assert!(list.finished());
    top.set_title("Drafts");
    top.surface.commit();
    settle(&mut fixture, &mut list);
    assert!(list.listed().is_empty(), "no window ever reaches it");
}

/// Asked, then allowed: the list is empty while the question is open, then lists every window
/// and keeps up with them.
#[test]
fn a_list_bound_while_asked_is_released_when_allowed() {
    let (mut fixture, shell, asked, top, _buffer) = asked_with_window();
    let mut list = watch(&mut fixture, asked);
    assert_eq!(shell.asked().len(), 1);
    assert_eq!(shell.asked()[0].power, Power::Windows);
    assert!(list.listed().is_empty(), "no window while it waits");
    assert!(!list.finished());

    shell.answer(&mut fixture, shell.only_ask(), Answer::Allow);
    settle(&mut fixture, &mut list);
    assert_eq!(titles(&list), ["Inbox"]);
    assert!(!list.finished());
    assert_eq!(
        kept(&fixture, ASKED_PROGRAM, Power::Windows),
        Some(Verdict::Allow)
    );

    top.set_title("Drafts");
    top.surface.commit();
    settle(&mut fixture, &mut list);
    assert_eq!(titles(&list), ["Drafts"], "and kept up to date");
}

/// Asked, then refused: the list is finished and never lists a window.
#[test]
fn a_list_bound_while_asked_is_finished_when_refused() {
    let (mut fixture, shell, asked, top, _buffer) = asked_with_window();
    let mut list = watch(&mut fixture, asked);
    shell.answer(&mut fixture, shell.only_ask(), Answer::Deny);
    settle(&mut fixture, &mut list);
    assert!(list.finished());
    top.set_title("Drafts");
    top.surface.commit();
    settle(&mut fixture, &mut list);
    assert!(list.listed().is_empty());
    assert_eq!(
        kept(&fixture, ASKED_PROGRAM, Power::Windows),
        Some(Verdict::Deny)
    );
}

/// A client that hangs up while its list waits is forgotten, and an answer that comes after it
/// is gone changes nothing for anyone.
#[test]
fn a_client_that_leaves_while_its_list_waits_is_forgotten() {
    let (mut fixture, shell, asked, _top, _buffer) = asked_with_window();
    let _list = watch(&mut fixture, asked);
    assert_eq!(fixture.held_connections(), 1);
    fixture.disconnect(asked);
    assert_eq!(fixture.held_connections(), 0);
    shell.answer(&mut fixture, shell.only_ask(), Answer::Allow);
    assert!(fixture.client(shell.id).protocol_error().is_none());
}
