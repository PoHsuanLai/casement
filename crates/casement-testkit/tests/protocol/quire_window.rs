//! `quire_window` (P0-H2, C50): hints in (the double-buffered ones at the toplevel's commit),
//! emphasis and placement out, the documented errors, and what destroying the object forgets.
//! Ignored until the fixture and the core loop are filled.

use crate::xdg_shell::{
    boot, check_golden, click, golden, lines_of, map_toplevel, pointer, windows,
};
use casement_core::policy::ConnectionRole;
use casement_core::window::{A11yRoot, FrameHint};
use casement_protocols::vocab::{Edited as ModelEdited, FrameMode as ModelFrame};
use casement_testclient::quire::window::{
    Edited, Emphasis, FrameMode, ManagerError, PlaceTarget, PlacementMark, WindowError,
    WindowManager, WindowRole,
};
use casement_testclient::surface::toplevel::{WmBase, bind_pointer};

/// Quire_window_manager_v1 version 1 is advertised to public clients (C50: shell-host's probe).
#[test]
fn the_global_is_advertised_from_p0() {
    let (mut fixture, id) = boot();
    let advertised = fixture
        .client(id)
        .globals()
        .contents()
        .clone_list()
        .into_iter()
        .find(|global| global.interface == "quire_window_manager_v1")
        .expect("the global is advertised to a public client");
    assert_eq!(advertised.version, 1);
    let _ = WindowManager::bind(fixture.client(id), 1);
    fixture.pump();
    assert!(fixture.client(id).protocol_error().is_none());
}

/// The key window receives emphasis(key), the others inactive.
#[test]
#[ignore = "P0-B-headless: Fixture::start; P0-B-loop: core_loop, exec"]
fn emphasis_follows_focus() {
    let (mut fixture, id) = boot();
    let base = WmBase::bind(fixture.client(id), 7);
    let manager = WindowManager::bind(fixture.client(id), 1);
    let first = map_toplevel(&mut fixture, id, &base, (400, 300));
    let second = map_toplevel(&mut fixture, id, &base, (300, 200));
    let first_ext = manager.window(fixture.client(id), &first);
    let second_ext = manager.window(fixture.client(id), &second);
    fixture.pump();
    // Created once: emphasis and placement are sent at creation.
    assert_eq!(first_ext.emphasis(), Some(Emphasis::Inactive));
    assert_eq!(first_ext.placement(), Some(PlacementMark::Free));
    let ids = windows(&fixture);
    let device = pointer(&mut fixture);
    click(&mut fixture, device, ids[0]);
    assert_eq!(first_ext.emphasis(), Some(Emphasis::Key));
    assert_eq!(second_ext.emphasis(), Some(Emphasis::Inactive));
    click(&mut fixture, device, ids[1]);
    assert_eq!(first_ext.emphasis(), Some(Emphasis::Inactive));
    assert_eq!(second_ext.emphasis(), Some(Emphasis::Key));
}

/// A second get_window on one toplevel raises already_constructed.
#[test]
fn a_second_get_window_is_already_constructed() {
    let (mut fixture, id) = boot();
    let base = WmBase::bind(fixture.client(id), 7);
    let manager = WindowManager::bind(fixture.client(id), 1);
    let top = base.toplevel(fixture.client(id));
    let _first = manager.window(fixture.client(id), &top);
    let _second = manager.window(fixture.client(id), &top);
    fixture.pump();
    let error = fixture
        .client(id)
        .protocol_error()
        .expect("a protocol error");
    assert_eq!(error.interface, "quire_window_manager_v1");
    assert_eq!(error.code, ManagerError::AlreadyConstructed as u32);
}

/// Request_placement with a press serial is stored in the model, not acted on at P0.
#[test]
#[ignore = "P0-B-headless: Fixture::start; P0-B-loop: core_loop, exec"]
fn request_placement_is_stored() {
    let (mut fixture, id) = boot();
    let base = WmBase::bind(fixture.client(id), 7);
    let manager = WindowManager::bind(fixture.client(id), 1);
    let pointer_log = bind_pointer(fixture.client(id));
    fixture.pump();
    let top = map_toplevel(&mut fixture, id, &base, (400, 300));
    let ext = manager.window(fixture.client(id), &top);
    let window = windows(&fixture)[0];
    let device = pointer(&mut fixture);
    click(&mut fixture, device, window);
    click(&mut fixture, device, window);
    let serial = pointer_log.last_press().expect("a press was delivered");
    let before = fixture.model().windows()[&window].state.clone();
    ext.request_placement(PlaceTarget::Left, serial);
    ext.request_placement(PlaceTarget::Fill, serial.wrapping_add(1000));
    fixture.pump();
    assert!(
        fixture.client(id).protocol_error().is_none(),
        "a bad serial is ignored, not an error"
    );
    assert_eq!(
        fixture.model().windows()[&window].state,
        before,
        "P0 does not act on placement"
    );
}

/// `dialog` and `shell` are not roles a client may ask for (`bad_role`); a role it may ask for
/// reaches the model.
#[test]
fn roles_are_checked() {
    let (mut fixture, id) = boot();
    let base = WmBase::bind(fixture.client(id), 7);
    let manager = WindowManager::bind(fixture.client(id), 1);
    let top = map_toplevel(&mut fixture, id, &base, (400, 300));
    let ext = manager.window(fixture.client(id), &top);
    ext.set_role(WindowRole::Utility);
    fixture.pump();
    let window = windows(&fixture)[0];
    assert_eq!(
        fixture.model().windows()[&window].role,
        casement_protocols::vocab::WindowRole::Utility
    );
    ext.set_role(WindowRole::Dialog);
    fixture.pump();
    let error = fixture
        .client(id)
        .protocol_error()
        .expect("a protocol error");
    assert_eq!(error.interface, "quire_window_v1");
    assert_eq!(error.code, WindowError::BadRole as u32);
}

/// Once the toplevel is destroyed every request but `destroy` is `defunct_toplevel`.
#[test]
fn a_defunct_toplevel_is_an_error() {
    let (mut fixture, id) = boot();
    let base = WmBase::bind(fixture.client(id), 7);
    let manager = WindowManager::bind(fixture.client(id), 1);
    let top = map_toplevel(&mut fixture, id, &base, (400, 300));
    let ext = manager.window(fixture.client(id), &top);
    fixture.pump();
    top.destroy();
    fixture.pump();
    assert!(
        fixture.client(id).protocol_error().is_none(),
        "destroying the toplevel first is allowed"
    );
    ext.set_frame(FrameMode::CompositorShadow, 10);
    fixture.pump();
    let error = fixture
        .client(id)
        .protocol_error()
        .expect("a protocol error");
    assert_eq!(error.code, WindowError::DefunctToplevel as u32);
}

/// Frame hints are double-buffered (they apply at the toplevel's next commit); the rest apply at
/// once; destroying the object forgets them all.
#[test]
fn hints_apply_on_commit_and_are_forgotten_on_destroy() {
    let (mut fixture, id) = boot();
    let base = WmBase::bind(fixture.client(id), 7);
    let manager = WindowManager::bind(fixture.client(id), 1);
    let top = map_toplevel(&mut fixture, id, &base, (400, 300));
    let ext = manager.window(fixture.client(id), &top);
    let window = windows(&fixture)[0];
    ext.set_frame(FrameMode::CompositorShadow, 12);
    ext.set_edited(Edited::Unsaved);
    ext.set_a11y_root(":1.5", "/org/a11y/root");
    fixture.pump();
    let hints = fixture.model().windows()[&window].hints.clone();
    assert_eq!(hints.edited, ModelEdited::Unsaved, "edited applies at once");
    assert_eq!(
        hints.a11y,
        A11yRoot::At {
            bus_name: ":1.5".to_owned(),
            object_path: "/org/a11y/root".to_owned()
        }
    );
    assert_eq!(
        hints.frame.mode,
        ModelFrame::Client,
        "the frame waits for the commit"
    );
    top.surface.commit();
    fixture.pump();
    assert_eq!(
        fixture.model().windows()[&window].hints.frame,
        FrameHint {
            mode: ModelFrame::CompositorShadow,
            corner_radius: 12
        }
    );
    ext.object.destroy();
    fixture.pump();
    let forgotten = fixture.model().windows()[&window].hints.clone();
    assert_eq!(forgotten.edited, ModelEdited::Saved);
    assert_eq!(forgotten.a11y, A11yRoot::Unknown);
    assert_eq!(forgotten.frame.mode, ModelFrame::Client);
    let _ = ConnectionRole::Public;
}

/// A new extension object is told its emphasis and placement once, at creation: the transcript
/// matches tests/golden/transcripts/quire_window-create.txt.
#[test]
fn the_creation_transcript_matches_its_golden() {
    let (mut fixture, id) = boot();
    let base = WmBase::bind(fixture.client(id), 7);
    let manager = WindowManager::bind(fixture.client(id), 1);
    let top = base.toplevel(fixture.client(id));
    let _ext = manager.window(fixture.client(id), &top);
    fixture.pump();
    let text = lines_of(&mut fixture, id, &["quire_window_v1#"]);
    check_golden(&golden("quire_window-create.txt"), &text).expect("the creation flow");
}
