//! `foreign_toplevel_list` (P0-H8). Runs once the fixture and test client core land.

use crate::xdg_shell::windows;
use casement_core::geometry::PhysicalSize;
use casement_core::policy::ConnectionRole;
use casement_testclient::manage::foreign_toplevel_list::{Listed, Listing, ToplevelList};
use casement_testclient::surface::toplevel::WmBase;
use casement_testclient::{Pattern, shm_buffer};
use casement_testkit::{Fixture, FixtureSpec};

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
