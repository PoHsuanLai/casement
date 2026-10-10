//! `SpaceSync` over `FakeSpaceRegistry`: every desktop is registered once with its plain name,
//! a Space made later goes alone, a refusal ends the asking, and a Space the service was not
//! ready for is asked again at the next change.

use casement::space_sync::SpaceSync;
use casement_core::output::{EdidFacts, EdidSerial};
use casement_core::space::{Space, SpaceBook, SpaceContent, SpaceIndex, TaskToken};
use casement_core::{OutputKey, OutputName};
use casement_ipc::space_registry::RegisterFault;
use casement_testkit::fakes::FakeSpaceRegistry;
use porter_core::SpaceId;

fn output() -> OutputKey {
    let edid = EdidFacts {
        make: String::new(),
        model: String::new(),
        serial: EdidSerial::Unknown,
    };
    OutputKey::of(&edid, &OutputName::parse("HDMI-A-1").expect("name"))
}

fn space(id: &str, content: SpaceContent, position: u32) -> Space {
    Space {
        id: SpaceId::parse(id).expect("an id"),
        output: output(),
        content,
        position: SpaceIndex(position),
    }
}

fn desktops(ids: &[&str]) -> SpaceBook {
    ids.iter()
        .zip(0u32..)
        .fold(SpaceBook::empty(), |book, (id, position)| {
            book.with_space(space(id, SpaceContent::Desktop, position))
        })
}

fn synced() -> (SpaceSync, FakeSpaceRegistry) {
    let fake = FakeSpaceRegistry::new();
    (SpaceSync::new(Box::new(fake.clone())), fake)
}

#[test]
fn the_loaded_desktops_go_in_one_batch_with_their_plain_names() {
    let (mut sync, fake) = synced();
    sync.sync(&desktops(&["s-a", "s-b"]));
    let batches = fake.batches();
    assert_eq!(batches.len(), 1);
    let names: Vec<(&str, &str)> = batches[0]
        .iter()
        .map(|r| (r.id.as_str(), r.name.as_str()))
        .collect();
    assert_eq!(names, [("s-a", "Desktop 1"), ("s-b", "Desktop 2")]);
}

#[test]
fn nothing_is_sent_twice_and_a_new_space_goes_alone() {
    let (mut sync, fake) = synced();
    let book = desktops(&["s-a"]);
    sync.sync(&book);
    sync.sync(&book);
    sync.sync(&book.with_space(space("s-b", SpaceContent::Desktop, 1)));
    assert_eq!(fake.ids(), ["s-a", "s-b"]);
    assert_eq!(fake.batches().len(), 2);
}

#[test]
fn only_desktops_are_registered() {
    let (mut sync, fake) = synced();
    let book = desktops(&["s-a"]).with_space(space(
        "s-agent",
        SpaceContent::Agent(TaskToken("t".to_owned())),
        1,
    ));
    sync.sync(&book);
    assert_eq!(fake.ids(), ["s-a"]);
}

#[test]
fn a_refusal_ends_the_asking() {
    let (mut sync, fake) = synced();
    fake.fail_next(RegisterFault::Refused);
    sync.sync(&desktops(&["s-a"]));
    sync.sync(&desktops(&["s-a", "s-b"]));
    assert_eq!(fake.ids(), ["s-a"], "nothing after the refusal");
}

#[test]
fn a_space_the_service_was_not_ready_for_is_asked_again_at_the_next_change() {
    let (mut sync, fake) = synced();
    fake.fail_next(RegisterFault::Later);
    sync.sync(&desktops(&["s-a"]));
    sync.sync(&desktops(&["s-a", "s-b"]));
    assert_eq!(fake.ids(), ["s-a", "s-a", "s-b"]);
}

#[test]
fn an_empty_book_registers_nothing() {
    let (mut sync, fake) = synced();
    sync.sync(&SpaceBook::empty());
    assert!(fake.batches().is_empty());
}
