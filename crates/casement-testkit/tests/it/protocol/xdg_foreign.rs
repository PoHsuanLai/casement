//! `xdg_foreign` (P0-H2): an imported parent reaches the model through `xdg_shell::on_parent`
//! (the model only parents windows of one app, so the two clients declare one app id).

use crate::protocol::xdg_shell::{boot, map_named, windows};
use casement_core::policy::ConnectionRole;
use casement_core::window::Parent;
use casement_testclient::surface::toplevel::WmBase;
use casement_testclient::surface::toplevel::extras::Foreign;

const APP: &str = "org.example.Shared";

/// A dialog of client B parented to client A's exported toplevel gets that parent in the model;
/// when the export goes away the dialog is an orphan again.
#[test]
fn an_imported_parent_becomes_the_parent() {
    let (mut fixture, owner) = boot();
    let guest = fixture.connect(ConnectionRole::Public);
    fixture.pump();
    let owner_base = WmBase::bind(fixture.client(owner), 7);
    let guest_base = WmBase::bind(fixture.client(guest), 7);
    let owner_foreign = Foreign::bind(fixture.client(owner));
    let guest_foreign = Foreign::bind(fixture.client(guest));

    let parent = map_named(&mut fixture, owner, &owner_base, (500, 400), Some(APP));
    let (exported, handle) = owner_foreign.export(fixture.client(owner), &parent);
    fixture.pump();
    let handle = handle
        .with(|seen| seen.clone())
        .expect("the export's handle arrives");

    let dialog = guest_base.toplevel(fixture.client(guest));
    dialog.set_app_id(APP);
    let (_imported, imported_log) =
        guest_foreign.import_parent_of(fixture.client(guest), &handle, &dialog);
    dialog.commit_bare();
    fixture.pump();
    dialog.ack_latest();
    let buffer = casement_testclient::shm_buffer(
        fixture.client(guest),
        casement_core::geometry::PhysicalSize { w: 300, h: 200 },
        casement_testclient::Pattern::Gradient,
    );
    dialog.show(&buffer);
    fixture.pump();

    let ids = windows(&fixture);
    assert_eq!(ids.len(), 2);
    assert_eq!(
        fixture.model().windows()[&ids[1]].parent,
        Parent::Window(ids[0])
    );

    exported.destroy();
    fixture.pump();
    assert_eq!(
        imported_log.with(|seen| seen.destroyed),
        1,
        "the import is told its export is gone"
    );
    assert_eq!(fixture.model().windows()[&ids[1]].parent, Parent::None);
    assert!(fixture.client(owner).protocol_error().is_none());
    assert!(fixture.client(guest).protocol_error().is_none());
}

/// A handle nobody exported is answered with `destroyed` at once, and it is no protocol error.
#[test]
fn an_unknown_handle_is_destroyed_at_once() {
    let (mut fixture, id) = boot();
    let base = WmBase::bind(fixture.client(id), 7);
    let foreign = Foreign::bind(fixture.client(id));
    let top = base.toplevel(fixture.client(id));
    let (_imported, log) = foreign.import_parent_of(fixture.client(id), "no-such-handle", &top);
    fixture.pump();
    assert_eq!(log.with(|seen| seen.destroyed), 1);
    assert!(fixture.client(id).protocol_error().is_none());
}

/// An export whose toplevel is destroyed is no longer valid: its import is told, and the
/// handle names nothing for a later import.
#[test]
fn a_destroyed_exported_toplevel_destroys_its_imports() {
    let (mut fixture, owner) = boot();
    let guest = fixture.connect(ConnectionRole::Public);
    fixture.pump();
    let owner_base = WmBase::bind(fixture.client(owner), 7);
    let guest_base = WmBase::bind(fixture.client(guest), 7);
    let owner_foreign = Foreign::bind(fixture.client(owner));
    let guest_foreign = Foreign::bind(fixture.client(guest));
    let parent = map_named(&mut fixture, owner, &owner_base, (500, 400), Some(APP));
    let (_exported, handle) = owner_foreign.export(fixture.client(owner), &parent);
    fixture.pump();
    let handle = handle.with(|seen| seen.clone()).expect("a handle");
    let dialog = guest_base.toplevel(fixture.client(guest));
    let (_imported, log) = guest_foreign.import_parent_of(fixture.client(guest), &handle, &dialog);
    fixture.pump();
    assert_eq!(log.with(|seen| seen.destroyed), 0, "the export is alive");
    parent.destroy();
    fixture.pump();
    assert_eq!(log.with(|seen| seen.destroyed), 1);
    assert!(fixture.client(owner).protocol_error().is_none());
    assert!(fixture.client(guest).protocol_error().is_none());
}

/// CASW-06: a client may hold `MAX_EXPORTS` exports; the next `export_toplevel` cuts it with
/// `wl_display.no_memory` (code 2).
#[test]
fn a_client_that_exports_without_end_is_cut() {
    let (mut fixture, owner) = boot();
    let base = WmBase::bind(fixture.client(owner), 7);
    let foreign = Foreign::bind(fixture.client(owner));
    let parent = map_named(&mut fixture, owner, &base, (500, 400), Some(APP));
    let mut made = Vec::new();
    for _ in 0..casement_wayland::limits::MAX_EXPORTS {
        made.push(foreign.export(fixture.client(owner), &parent));
    }
    fixture.pump();
    assert!(
        fixture.client(owner).protocol_error().is_none(),
        "up to the cap is allowed"
    );
    made.push(foreign.export(fixture.client(owner), &parent));
    fixture.pump();
    // The cut closes the connection (the system backend sends no error first).
    assert!(
        !fixture.client(owner).is_connected(),
        "the client past the cap is cut"
    );
}
