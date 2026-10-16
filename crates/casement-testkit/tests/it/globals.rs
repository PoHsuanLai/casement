//! The globals golden (`tests/golden/globals.txt`): its shape now, the fixture's registry
//! against it once the fixture runs. The power column is the one `power_of` gives the global,
//! so a global that needs the person's consent cannot be listed as free.

use casement_core::policy::{ConnectionRole, GlobalKind, Power, power_of};
use casement_testkit::{Fixture, FixtureSpec};
use std::collections::BTreeSet;

const GLOBALS: &str = include_str!("../golden/globals.txt");

/// One golden row.
struct Row {
    interface: String,
    version: u32,
    module: String,
    power: String,
}

/// The golden's rows.
fn rows() -> Vec<Row> {
    GLOBALS
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|line| {
            let fields: Vec<&str> = line.split_whitespace().collect();
            assert_eq!(fields.len(), 4, "row {line:?}");
            Row {
                interface: fields[0].to_owned(),
                version: fields[1].parse().expect("a version"),
                module: fields[2].to_owned(),
                power: fields[3].to_owned(),
            }
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
        GlobalKind::ColorManagement => "color_management",
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
        // P1 globals, advertised behind their powers.
        GlobalKind::OutputManagement => "output_management",
        GlobalKind::OutputPower => "output_power",
        GlobalKind::Gamma => "gamma",
        GlobalKind::WlrToplevel => "wlr_toplevel",
        GlobalKind::Workspace => "workspace",
        GlobalKind::ImageCapture => "image_capture",
        // P2-Q3: the toplevel capture source is advertised to the public socket (golden row);
        // the two quire protocols never are, so the public golden has no row for them.
        GlobalKind::ToplevelCapture => "capture_source",
        GlobalKind::QuireDesktop => "quire_desktop",
        GlobalKind::QuireShell => "quire_shell",
    }
}

/// Every P0 global kind (the `match` above fails to compile when one is added).
const KINDS: [GlobalKind; 51] = [
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
    GlobalKind::ColorManagement,
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
    GlobalKind::OutputManagement,
    GlobalKind::OutputPower,
    GlobalKind::Gamma,
    GlobalKind::WlrToplevel,
    GlobalKind::Workspace,
    GlobalKind::ImageCapture,
    GlobalKind::ToplevelCapture,
];

#[test]
fn every_global_kind_but_dmabuf_has_a_golden_row_and_rows_are_unique() {
    let rows = rows();
    let modules: BTreeSet<&str> = rows.iter().map(|row| row.module.as_str()).collect();
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
    let interfaces: BTreeSet<&str> = rows.iter().map(|row| row.interface.as_str()).collect();
    assert_eq!(interfaces.len(), rows.len(), "an interface is listed twice");
    assert!(rows.iter().all(|row| row.version >= 1));
}

/// The word the golden uses for `power`.
fn power_word(power: Option<Power>) -> &'static str {
    match power {
        None => "-",
        Some(Power::Screen) => "screen",
        Some(Power::Keys) => "keys",
        Some(Power::TypingHelp) => "typing_help",
        Some(Power::Clipboard) => "clipboard",
        Some(Power::Lock) => "lock",
        Some(Power::Displays) => "displays",
        Some(Power::Windows) => "windows",
    }
}

/// The global kind a golden module serves: `zwlr_screencopy_manager_v1` has its own module and
/// the capture kind of the ext managers.
fn kind_of_module(module: &str) -> GlobalKind {
    let module = match module {
        "screencopy" => "image_capture",
        other => other,
    };
    KINDS
        .into_iter()
        .find(|kind| module_of(*kind) == module)
        .unwrap_or_else(|| panic!("no global kind lives in module {module}"))
}

/// Every golden row names the power that gates the global (`-` for none): a privileged global
/// cannot be listed as free, and the public socket is shown all of them (the answer when
/// nothing is refused; the golden is what a public client is advertised).
#[test]
fn the_golden_names_the_power_of_every_global() {
    for row in rows() {
        let power = power_of(kind_of_module(&row.module));
        assert_eq!(row.power, power_word(power), "{}", row.interface);
    }
}

/// A public client is shown the global of each power until the person or the administrator
/// refuses the power: the seven powers each gate at least one advertised global.
#[test]
fn every_power_gates_a_global_the_public_socket_is_shown() {
    let rows = rows();
    for power in [
        Power::Screen,
        Power::Keys,
        Power::TypingHelp,
        Power::Clipboard,
        Power::Lock,
        Power::Displays,
        Power::Windows,
    ] {
        let word = power_word(Some(power));
        assert!(
            rows.iter().any(|row| row.power == word),
            "no global of {word} in the golden"
        );
    }
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
        .map(|row| (row.interface, row.version))
        .collect();
    golden.sort();
    assert_eq!(advertised, golden);
}
