//! Which globals a connection sees, as one table (I7: no ungated privileged global; the
//! globals that need the person's consent are advertised to a public client and gated where
//! they are used, unless the person already refused them; a sandboxed client sees less than a
//! native one).

use super::power::{Power, PowerSet, power_of};
use porter_core::SandboxFacts;

/// What kind of connection a client arrived on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionRole {
    /// The public socket.
    Public,
    /// A connection handed to the shell (sill, shell-host) through `org.quire.Compositor1`.
    Shell,
    /// A connection handed to the agent tier (cuad).
    Agent,
    /// A connection handed to the desktop portal (transom) for its screenshots and recordings:
    /// it sees what the public socket sees and is trusted for the screen and the windows
    /// (`trusted_powers`), nothing more.
    Portal,
    /// A security-context socket of this sandbox.
    Sandboxed(SandboxFacts),
}

/// Every global casement advertises, one variant per `delegates.rs` global (each delta
/// freeze adds its own; P0-Z2 checks this list against the delegates). P1-Z declared the last
/// six without advertising them: each fill lane creates its global in `Part::new`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum GlobalKind {
    /// `wl_compositor`.
    Compositor,
    /// `wl_subcompositor`.
    Subcompositor,
    /// `wl_shm`.
    Shm,
    /// `wl_output` and `zxdg_output_manager_v1`.
    Output,
    /// `wp_viewporter`.
    Viewporter,
    /// `wp_single_pixel_buffer_manager_v1`.
    SinglePixel,
    /// `wp_alpha_modifier_v1`.
    AlphaModifier,
    /// `wp_presentation`.
    Presentation,
    /// `wp_fractional_scale_manager_v1`.
    FractionalScale,
    /// `wp_fifo_manager_v1`.
    Fifo,
    /// `wp_commit_timing_manager_v1`.
    CommitTiming,
    /// `wp_content_type_manager_v1`.
    ContentType,
    /// `wp_color_manager_v1`.
    ColorManagement,
    /// `xdg_wm_base`.
    XdgShell,
    /// `zxdg_decoration_manager_v1`.
    XdgDecoration,
    /// `xdg_activation_v1`.
    XdgActivation,
    /// `xdg_wm_dialog_v1`.
    XdgDialog,
    /// `xdg_toplevel_icon_manager_v1`.
    XdgToplevelIcon,
    /// `xdg_toplevel_tag_manager_v1`.
    XdgToplevelTag,
    /// `zxdg_exporter_v2` and `zxdg_importer_v2`.
    XdgForeign,
    /// `xdg_system_bell_v1`.
    SystemBell,
    /// `quire_window_manager_v1` (C50).
    QuireWindow,
    /// `zwlr_layer_shell_v1`.
    LayerShell,
    /// `ext_session_lock_manager_v1`.
    SessionLock,
    /// `wl_seat`.
    Seat,
    /// `wp_cursor_shape_manager_v1`.
    CursorShape,
    /// `zwp_pointer_constraints_v1`.
    PointerConstraints,
    /// `zwp_relative_pointer_manager_v1`.
    RelativePointer,
    /// `zwp_pointer_gestures_v1`.
    PointerGestures,
    /// `zwp_keyboard_shortcuts_inhibit_manager_v1`.
    ShortcutsInhibit,
    /// `zwp_tablet_manager_v2`.
    Tablet,
    /// `wl_data_device_manager`.
    DataDevice,
    /// `zwp_primary_selection_device_manager_v1`.
    PrimarySelection,
    /// `ext_data_control_manager_v1`.
    ExtDataControl,
    /// `zwlr_data_control_manager_v1`.
    WlrDataControl,
    /// `zwp_linux_dmabuf_v1`.
    Dmabuf,
    /// `zwp_text_input_manager_v3`.
    TextInput,
    /// `zwp_input_method_manager_v2`.
    InputMethod,
    /// `zwp_virtual_keyboard_manager_v1`.
    VirtualKeyboard,
    /// `ext_idle_notifier_v1`.
    IdleNotify,
    /// `zwp_idle_inhibit_manager_v1`.
    IdleInhibit,
    /// `ext_foreign_toplevel_list_v1`.
    ForeignToplevelList,
    /// `wp_security_context_manager_v1`.
    SecurityContext,
    /// `ext_background_effect_manager_v1`.
    BackgroundEffect,
    /// `zwlr_output_manager_v1` (P1-Z; advertised by P1-M1).
    OutputManagement,
    /// `zwlr_output_power_manager_v1` (P1-Z; advertised by P1-M1).
    OutputPower,
    /// `zwlr_gamma_control_manager_v1` (P1-Z; advertised by P1-M1).
    Gamma,
    /// `zwlr_foreign_toplevel_manager_v1` (P1-Z; advertised by P1-M2).
    WlrToplevel,
    /// `ext_workspace_manager_v1` (P1-Z; advertised by P1-M2).
    Workspace,
    /// `ext_output_image_capture_source_manager_v1` and `ext_image_copy_capture_manager_v1`
    /// (P1-Z; advertised by P1-C1).
    ImageCapture,
    /// `ext_foreign_toplevel_image_capture_source_manager_v1` (P2-Q3: window capture; a public
    /// client sees it, a sandbox does not).
    ToplevelCapture,
    /// `quire_desktop_manager_v1`: the desktop model of apps, windows and Spaces (P2-Z; the
    /// shell manages through it, an agent reads it).
    QuireDesktop,
    /// `quire_shell_v1`: the shell's surface roles and system events (P2-Z; the shell only).
    QuireShell,
}

impl GlobalKind {
    /// Every global, in declaration order. The golden table in the tests lists one row per
    /// entry, so a global added to the table is a row to review.
    pub fn all() -> impl Iterator<Item = GlobalKind> + Clone {
        GLOBALS.iter().map(|global| global.kind)
    }

    /// This global's row in the table.
    pub(super) fn entry(self) -> &'static Global {
        &GLOBALS[self as usize]
    }
}

/// Which connections may see a global at all, before a sandbox narrows it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum Tier {
    /// Every kind of connection.
    Everyone,
    /// The shell and the agent tier (never the public socket).
    ShellAndAgent,
    /// The shell only.
    ShellOnly,
}

/// One row of the globals table: everything the connection policy knows about a global.
pub(super) struct Global {
    kind: GlobalKind,
    /// Which roles may see it before a sandbox narrows it.
    tier: Tier,
    /// Whether a sandboxed client sees it.
    sandboxed: bool,
    /// The power it needs consent for; none for a global every client may use. Layer shell and
    /// the security-context manager stay without one: layer shell's keyboard grab is closed in
    /// the model, and a security-context client is a sandbox, which sees none of these globals.
    pub(super) power: Option<Power>,
}

const fn row(kind: GlobalKind, tier: Tier, sandboxed: bool, power: Option<Power>) -> Global {
    Global {
        kind,
        tier,
        sandboxed,
        power,
    }
}

/// The globals table, one row per [`GlobalKind`] in declaration order (checked at compile time
/// below, so a row cannot drift from the enum). Everything is `Everyone` but the two `quire`
/// management protocols. Whoever adds a global adds its row and decides on purpose whether a
/// sandbox sees it, who else may, and which power gates it (I7).
const GLOBALS: &[Global] = &[
    row(GlobalKind::Compositor, Tier::Everyone, true, None),
    row(GlobalKind::Subcompositor, Tier::Everyone, true, None),
    row(GlobalKind::Shm, Tier::Everyone, true, None),
    row(GlobalKind::Output, Tier::Everyone, true, None),
    row(GlobalKind::Viewporter, Tier::Everyone, true, None),
    row(GlobalKind::SinglePixel, Tier::Everyone, true, None),
    row(GlobalKind::AlphaModifier, Tier::Everyone, true, None),
    row(GlobalKind::Presentation, Tier::Everyone, true, None),
    row(GlobalKind::FractionalScale, Tier::Everyone, true, None),
    row(GlobalKind::Fifo, Tier::Everyone, true, None),
    row(GlobalKind::CommitTiming, Tier::Everyone, true, None),
    row(GlobalKind::ContentType, Tier::Everyone, true, None),
    row(GlobalKind::ColorManagement, Tier::Everyone, true, None),
    row(GlobalKind::XdgShell, Tier::Everyone, true, None),
    row(GlobalKind::XdgDecoration, Tier::Everyone, true, None),
    row(GlobalKind::XdgActivation, Tier::Everyone, true, None),
    row(GlobalKind::XdgDialog, Tier::Everyone, true, None),
    row(GlobalKind::XdgToplevelIcon, Tier::Everyone, true, None),
    row(GlobalKind::XdgToplevelTag, Tier::Everyone, true, None),
    row(GlobalKind::XdgForeign, Tier::Everyone, true, None),
    row(GlobalKind::SystemBell, Tier::Everyone, true, None),
    row(GlobalKind::QuireWindow, Tier::Everyone, true, None),
    row(GlobalKind::LayerShell, Tier::Everyone, false, None),
    row(
        GlobalKind::SessionLock,
        Tier::Everyone,
        false,
        Some(Power::Lock),
    ),
    row(GlobalKind::Seat, Tier::Everyone, true, None),
    row(GlobalKind::CursorShape, Tier::Everyone, true, None),
    row(GlobalKind::PointerConstraints, Tier::Everyone, true, None),
    row(GlobalKind::RelativePointer, Tier::Everyone, true, None),
    row(GlobalKind::PointerGestures, Tier::Everyone, true, None),
    row(GlobalKind::ShortcutsInhibit, Tier::Everyone, true, None),
    row(GlobalKind::Tablet, Tier::Everyone, true, None),
    row(GlobalKind::DataDevice, Tier::Everyone, true, None),
    row(GlobalKind::PrimarySelection, Tier::Everyone, true, None),
    row(
        GlobalKind::ExtDataControl,
        Tier::Everyone,
        false,
        Some(Power::Clipboard),
    ),
    row(
        GlobalKind::WlrDataControl,
        Tier::Everyone,
        false,
        Some(Power::Clipboard),
    ),
    row(GlobalKind::Dmabuf, Tier::Everyone, true, None),
    row(GlobalKind::TextInput, Tier::Everyone, true, None),
    row(
        GlobalKind::InputMethod,
        Tier::Everyone,
        false,
        Some(Power::TypingHelp),
    ),
    row(
        GlobalKind::VirtualKeyboard,
        Tier::Everyone,
        false,
        Some(Power::Keys),
    ),
    row(GlobalKind::IdleNotify, Tier::Everyone, true, None),
    row(GlobalKind::IdleInhibit, Tier::Everyone, true, None),
    row(
        GlobalKind::ForeignToplevelList,
        Tier::Everyone,
        false,
        Some(Power::Windows),
    ),
    row(GlobalKind::SecurityContext, Tier::Everyone, false, None),
    row(GlobalKind::BackgroundEffect, Tier::Everyone, true, None),
    row(
        GlobalKind::OutputManagement,
        Tier::Everyone,
        false,
        Some(Power::Displays),
    ),
    row(
        GlobalKind::OutputPower,
        Tier::Everyone,
        false,
        Some(Power::Displays),
    ),
    row(
        GlobalKind::Gamma,
        Tier::Everyone,
        false,
        Some(Power::Displays),
    ),
    row(
        GlobalKind::WlrToplevel,
        Tier::Everyone,
        false,
        Some(Power::Windows),
    ),
    row(
        GlobalKind::Workspace,
        Tier::Everyone,
        false,
        Some(Power::Windows),
    ),
    row(
        GlobalKind::ImageCapture,
        Tier::Everyone,
        false,
        Some(Power::Screen),
    ),
    row(
        GlobalKind::ToplevelCapture,
        Tier::Everyone,
        false,
        Some(Power::Screen),
    ),
    row(GlobalKind::QuireDesktop, Tier::ShellAndAgent, false, None),
    row(GlobalKind::QuireShell, Tier::ShellOnly, false, None),
];

const _: () = {
    let mut at = 0;
    while at < GLOBALS.len() {
        assert!(GLOBALS[at].kind as usize == at);
        at += 1;
    }
    assert!(GLOBALS.len() == GlobalKind::QuireShell as usize + 1);
};

/// Whether a connection of `role` sees `global` when nothing is hidden from it by consent:
/// [`advertised_to`] with no refused powers. A sandboxed client sees none of the clipboard
/// managers, the toplevel list, the input-method and virtual-keyboard managers, the
/// security-context manager, layer shell, the session lock, output and window management,
/// workspaces, capture and the desktop and shell protocols. The shell connection sees
/// everything. The agent connection sees everything but `quire_shell_v1`; the public socket
/// sees neither `quire_desktop_manager_v1` nor `quire_shell_v1` (P2-Z).
///
/// The table `GLOBALS` has one row per global: a new global fails to compile until it has a
/// row that decides whether a sandbox sees it, its tier and its power (I7).
pub fn advertised(global: GlobalKind, role: &ConnectionRole) -> bool {
    advertised_to(global, role, &PowerSet::default())
}

/// Whether a connection of `role` sees `global`, given the powers its program was refused at
/// connect time (`hidden`: the person's or the administrator's no). A public client is not
/// shown a global of a refused power, so a tool says "not supported" instead of waiting for an
/// answer that will not come. Every other connection ignores `hidden`: the shell and the agent
/// are trusted, and a sandbox already sees only what its table says.
///
/// A global that needs consent but was not refused is advertised and gated where it is used
/// (I7: there is no ungated privileged global).
pub fn advertised_to(global: GlobalKind, role: &ConnectionRole, hidden: &PowerSet) -> bool {
    match (by_role(global, role), role, power_of(global)) {
        (true, ConnectionRole::Public, Some(power)) if hidden.contains(power) => false,
        (seen, _, _) => seen,
    }
}

/// What the connection's role alone allows.
fn by_role(global: GlobalKind, role: &ConnectionRole) -> bool {
    match role {
        ConnectionRole::Shell => true,
        ConnectionRole::Agent => match global.entry().tier {
            Tier::ShellOnly => false,
            Tier::ShellAndAgent | Tier::Everyone => true,
        },
        ConnectionRole::Public | ConnectionRole::Portal => match global.entry().tier {
            Tier::ShellOnly | Tier::ShellAndAgent => false,
            Tier::Everyone => true,
        },
        ConnectionRole::Sandboxed(_) => global.entry().sandboxed,
    }
}
