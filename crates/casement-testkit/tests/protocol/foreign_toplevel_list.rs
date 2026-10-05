//! `foreign_toplevel_list` (P0-H8). Runs once the fixture and test client core land.

use casement_core::policy::ConnectionRole;
use casement_testclient::manage::foreign_toplevel_list::ToplevelList;
use casement_testkit::{Fixture, FixtureSpec};
use porter_core::SandboxFacts;

/// Each mapped window appears with title and app id; closing it sends closed.
#[test]
#[ignore = "body is todo!(): write it with casement_testclient::surface::toplevel (now on master)"]
fn windows_are_listed_and_closed() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let watcher = fixture.connect(ConnectionRole::Public);
    fixture.pump();
    let mut list = ToplevelList::start(fixture.client(watcher).globals()).expect("the list");
    fixture.pump();
    list.pump().expect("pump");
    assert!(list.listed().is_empty(), "no window yet");
    todo!(
        "P0-H8: map a toplevel titled 'Inbox' with app id 'org.example.Mail' through the P0-T-client toplevel helper, pump, \
         assert list.listed() == [Listed {{ identifier: 'w<id>', title: 'Inbox', app_id: 'org.example.Mail', listing: Open }}], \
         retitle it and assert a second `done` carries the new title, destroy it and assert listing == Closed"
    )
}

/// A Sandboxed connection does not see ext_foreign_toplevel_list_v1 (I7).
#[test]
fn the_list_is_hidden_from_sandboxed_clients() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let client = fixture.connect(ConnectionRole::Sandboxed(SandboxFacts::Flatpak {
        app: "org.example.App".to_owned(),
        instance: "1".to_owned(),
    }));
    fixture.pump();
    let hidden = fixture
        .client(client)
        .globals()
        .contents()
        .with_list(|list| {
            !list
                .iter()
                .any(|global| global.interface == "ext_foreign_toplevel_list_v1")
        });
    assert!(hidden);
}
