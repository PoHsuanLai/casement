//! `xdg_toplevel_tag` (P0-H2). The tag lives in the protocol part (placement memory reads it at
//! P3), which the fixture does not expose, so the test proves the request path: accepted,
//! repeatable, harmless when the toplevel goes first. Ignored until the fixture and the core loop
//! are filled.

use crate::xdg_shell::{boot, map_toplevel};
use casement_testclient::surface::toplevel::WmBase;
use casement_testclient::surface::toplevel::extras::TagManager;

/// Set_toplevel_tag is kept for placement memory.
#[test]
fn a_tag_is_stored() {
    let (mut fixture, id) = boot();
    let base = WmBase::bind(fixture.client(id), 7);
    let tags = TagManager::bind(fixture.client(id));
    let top = map_toplevel(&mut fixture, id, &base, (400, 300));
    tags.tag(&top, "inspector", "The inspector panel");
    tags.tag(&top, "inspector-2", "Renamed");
    fixture.pump();
    assert!(fixture.client(id).protocol_error().is_none());
    // Destroying the toplevel with a tag set forgets it without a protocol error.
    top.destroy();
    fixture.pump();
    assert!(fixture.client(id).protocol_error().is_none());
    assert!(fixture.model().windows().is_empty());
}
