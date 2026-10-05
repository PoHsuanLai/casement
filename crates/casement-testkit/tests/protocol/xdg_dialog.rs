//! `xdg_dialog` (P0-H2): the hint is read when the window maps. Ignored until the fixture and
//! the core loop are filled.

use crate::xdg_shell::{boot, map_toplevel, windows};
use casement_core::geometry::PhysicalSize;
use casement_protocols::vocab::{Modality, WindowRole};
use casement_testclient::surface::toplevel::WmBase;
use casement_testclient::surface::toplevel::extras::{DialogError, WmDialog};
use casement_testclient::{Pattern, shm_buffer};

/// Set_modal reaches the model as Modality::Modal on the dialog's window.
#[test]
fn a_modal_dialog_is_stored_as_modal() {
    let (mut fixture, id) = boot();
    let base = WmBase::bind(fixture.client(id), 7);
    let dialogs = WmDialog::bind(fixture.client(id));
    let parent = map_toplevel(&mut fixture, id, &base, (500, 400));

    // A modal dialog of the parent's app is a sheet on the parent.
    let sheet = base.toplevel(fixture.client(id));
    sheet.set_parent(Some(&parent));
    let dialog = dialogs.dialog(fixture.client(id), &sheet);
    dialog.set_modal();
    sheet.commit_bare();
    fixture.pump();
    sheet.ack_latest();
    let buffer = shm_buffer(
        fixture.client(id),
        PhysicalSize { w: 300, h: 200 },
        Pattern::Gradient,
    );
    sheet.show(&buffer);
    fixture.pump();

    // A dialog that is not modal blocks nothing.
    let plain = base.toplevel(fixture.client(id));
    plain.set_parent(Some(&parent));
    let plain_dialog = dialogs.dialog(fixture.client(id), &plain);
    plain_dialog.set_modal();
    plain_dialog.unset_modal();
    plain.commit_bare();
    fixture.pump();
    plain.ack_latest();
    plain.show(&buffer);
    fixture.pump();

    let ids = windows(&fixture);
    let held = fixture.model().windows();
    assert_eq!(held[&ids[1]].modality, Modality::WindowModal);
    assert_eq!(held[&ids[1]].role, WindowRole::Dialog);
    assert_eq!(held[&ids[2]].modality, Modality::Modeless);
    assert_eq!(held[&ids[0]].modality, Modality::None);
}

/// A second `xdg_dialog_v1` for one toplevel is `already_used`; the global binds at version 1.
#[test]
fn a_second_dialog_object_is_already_used() {
    let (mut fixture, id) = boot();
    let base = WmBase::bind(fixture.client(id), 7);
    let dialogs = WmDialog::bind(fixture.client(id));
    let top = base.toplevel(fixture.client(id));
    let _first = dialogs.dialog(fixture.client(id), &top);
    let _second = dialogs.dialog(fixture.client(id), &top);
    fixture.pump();
    let error = fixture
        .client(id)
        .protocol_error()
        .expect("a protocol error");
    assert_eq!(error.interface, "xdg_wm_dialog_v1");
    assert_eq!(error.code, DialogError::AlreadyUsed as u32);
}
