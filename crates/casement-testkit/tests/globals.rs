//! The globals golden (`tests/golden/globals.txt`): its shape now, the fixture's registry
//! against it once the fixture runs.

use casement_core::policy::{ConnectionRole, GlobalKind};
use casement_testkit::{Fixture, FixtureSpec};
use std::collections::BTreeSet;

const GLOBALS: &str = include_str!("golden/globals.txt");

/// The golden's rows: interface, version, module.
fn rows() -> Vec<(String, u32, String)> {
    GLOBALS
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|line| {
            let fields: Vec<&str> = line.split_whitespace().collect();
            assert_eq!(fields.len(), 3, "row {line:?}");
            (
                fields[0].to_owned(),
                fields[1].parse().expect("a version"),
                fields[2].to_owned(),
            )
        })
        .collect()
}

/// The module each P0 `GlobalKind` lives in: the golden must have a row for each.
fn module_of(kind: GlobalKind) -> &'static str {
    match kind {
        GlobalKind::Compositor => "compositor",
        GlobalKind::Subcompositor => "subcompositor",
        GlobalKind::Shm => "shm",
        GlobalKind::Output => "output",
        GlobalKind::Viewporter => "viewporter",
        GlobalKind::SinglePixel => "single_pixel",
        GlobalKind::AlphaModifier => "alpha_modifier",
        GlobalKind::Presentation => "presentation",
        GlobalKind::FractionalScale => "fractional_scale",
        GlobalKind::Fifo => "fifo",
        GlobalKind::CommitTiming => "commit_timing",
        GlobalKind::ContentType => "content_type",
        GlobalKind::XdgShell => "xdg_shell",
        GlobalKind::XdgDecoration => "xdg_decoration",
        GlobalKind::XdgActivation => "xdg_activation",
        GlobalKind::XdgDialog => "xdg_dialog",
        GlobalKind::XdgToplevelIcon => "xdg_toplevel_icon",
        GlobalKind::XdgToplevelTag => "xdg_toplevel_tag",
        GlobalKind::XdgForeign => "xdg_foreign",
        GlobalKind::SystemBell => "system_bell",
        GlobalKind::QuireWindow => "quire_window",
        GlobalKind::LayerShell => "layer_shell",
        GlobalKind::SessionLock => "session_lock",
        GlobalKind::Seat => "seat",
        GlobalKind::CursorShape => "cursor_shape",
        GlobalKind::PointerConstraints => "pointer_constraints",
        GlobalKind::RelativePointer => "relative_pointer",
        GlobalKind::PointerGestures => "pointer_gestures",
        GlobalKind::ShortcutsInhibit => "shortcuts_inhibit",
        GlobalKind::Tablet => "tablet",
        GlobalKind::DataDevice => "data_device",
        GlobalKind::PrimarySelection => "primary_selection",
        GlobalKind::ExtDataControl => "ext_data_control",
        GlobalKind::WlrDataControl => "wlr_data_control",
        GlobalKind::Dmabuf => "dmabuf",
        GlobalKind::TextInput => "text_input",
        GlobalKind::InputMethod => "input_method",
        GlobalKind::VirtualKeyboard => "virtual_keyboard",
        GlobalKind::IdleNotify => "idle_notify",
        GlobalKind::IdleInhibit => "idle_inhibit",
        GlobalKind::ForeignToplevelList => "foreign_toplevel_list",
        GlobalKind::SecurityContext => "security_context",
        GlobalKind::BackgroundEffect => "background_effect",
    }
}

/// Every P0 global kind (the `match` above fails to compile when one is added).
const KINDS: [GlobalKind; 43] = [
    GlobalKind::Compositor,
    GlobalKind::Subcompositor,
    GlobalKind::Shm,
    GlobalKind::Output,
    GlobalKind::Viewporter,
    GlobalKind::SinglePixel,
    GlobalKind::AlphaModifier,
    GlobalKind::Presentation,
    GlobalKind::FractionalScale,
    GlobalKind::Fifo,
    GlobalKind::CommitTiming,
    GlobalKind::ContentType,
    GlobalKind::XdgShell,
    GlobalKind::XdgDecoration,
    GlobalKind::XdgActivation,
    GlobalKind::XdgDialog,
    GlobalKind::XdgToplevelIcon,
    GlobalKind::XdgToplevelTag,
    GlobalKind::XdgForeign,
    GlobalKind::SystemBell,
    GlobalKind::QuireWindow,
    GlobalKind::LayerShell,
    GlobalKind::SessionLock,
    GlobalKind::Seat,
    GlobalKind::CursorShape,
    GlobalKind::PointerConstraints,
    GlobalKind::RelativePointer,
    GlobalKind::PointerGestures,
    GlobalKind::ShortcutsInhibit,
    GlobalKind::Tablet,
    GlobalKind::DataDevice,
    GlobalKind::PrimarySelection,
    GlobalKind::ExtDataControl,
    GlobalKind::WlrDataControl,
    GlobalKind::Dmabuf,
    GlobalKind::TextInput,
    GlobalKind::InputMethod,
    GlobalKind::VirtualKeyboard,
    GlobalKind::IdleNotify,
    GlobalKind::IdleInhibit,
    GlobalKind::ForeignToplevelList,
    GlobalKind::SecurityContext,
    GlobalKind::BackgroundEffect,
];

#[test]
fn every_global_kind_but_dmabuf_has_a_golden_row_and_rows_are_unique() {
    let rows = rows();
    let modules: BTreeSet<&str> = rows.iter().map(|(_, _, module)| module.as_str()).collect();
    for kind in KINDS {
        // zwp_linux_dmabuf_v1 is advertised only with --dmabuf soft (a comment row).
        if kind != GlobalKind::Dmabuf {
            let module = module_of(kind);
            assert!(
                modules.contains(module),
                "no globals.txt row for {kind:?} ({module})"
            );
        }
    }
    let interfaces: BTreeSet<&str> = rows
        .iter()
        .map(|(interface, _, _)| interface.as_str())
        .collect();
    assert_eq!(interfaces.len(), rows.len(), "an interface is listed twice");
    assert!(rows.iter().all(|(_, version, _)| *version >= 1));
}

/// The default fixture advertises exactly the golden's globals at its versions to a public
/// client (one wl_output for its one output).
#[test]
fn the_default_fixture_advertises_the_golden_globals() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let client = fixture.connect(ConnectionRole::Public);
    // Sorted lists, not sets: a global advertised twice must not hide.
    let mut advertised: Vec<(String, u32)> = fixture
        .client(client)
        .globals()
        .contents()
        .clone_list()
        .into_iter()
        .map(|global| (global.interface, global.version))
        .collect();
    advertised.sort();
    let mut golden: Vec<(String, u32)> = rows()
        .into_iter()
        .map(|(interface, version, _)| (interface, version))
        .collect();
    golden.sort();
    assert_eq!(advertised, golden);
}
