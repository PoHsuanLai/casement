//! `system_bell` (P0-H2). The bell becomes `Effect::Bell` inside the handler (`app_to_ring` is
//! unit-tested in casement-wayland); the fixture has no effect log, so these tests prove the
//! wire: rings are accepted with and without a surface and change nothing in the model. Ignored
//! until the fixture and the core loop are filled.

use crate::protocol::xdg_shell::{boot, map_toplevel};
use casement_testclient::surface::toplevel::WmBase;
use casement_testclient::surface::toplevel::extras::SystemBell;

/// Ring with a surface yields Effect::Bell for that window's app.
#[test]
fn the_bell_rings_for_the_surfaces_app() {
    let (mut fixture, id) = boot();
    let base = WmBase::bind(fixture.client(id), 7);
    let bell = SystemBell::bind(fixture.client(id));
    let top = map_toplevel(&mut fixture, id, &base, (400, 300));
    let before = fixture.model().clone();
    bell.ring(Some(top.wl_surface()));
    bell.ring(None);
    fixture.pump();
    assert!(fixture.client(id).protocol_error().is_none());
    assert_eq!(
        *fixture.model(),
        before,
        "the bell changes no window and moves no focus"
    );
}
