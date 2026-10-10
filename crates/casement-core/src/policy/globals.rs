//! Which globals a connection sees, as one table (I7: no privileged global on the public
//! socket; a sandboxed client sees less than a native one).

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

/// Whether a connection sees a global.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Advertise {
    /// It does.
    Yes,
    /// It does not.
    No,
}

/// Whether a connection of `role` sees `global`. A sandboxed client sees none of the
/// clipboard managers, the toplevel list, the input-method and virtual-keyboard managers, the
/// security-context manager, layer shell, the session lock, output and window management,
/// workspaces, capture and the desktop and shell protocols. The shell connection sees
/// everything. The agent connection sees everything but `quire_shell_v1`; the public socket
/// sees neither `quire_desktop_manager_v1` nor `quire_shell_v1` (P2-Z).
///
/// The match on `global` in `sandbox_sees` is exhaustive on purpose: a new global fails to
/// compile until someone decides whether a sandbox sees it (I7). Whoever adds a privileged
/// global also gives it a tier in `tier_of`.
pub fn advertised(global: GlobalKind, role: &ConnectionRole) -> Advertise {
    match role {
        ConnectionRole::Shell => Advertise::Yes,
        ConnectionRole::Agent => match tier_of(global) {
            Tier::ShellOnly => Advertise::No,
            Tier::ShellAndAgent | Tier::Everyone => Advertise::Yes,
        },
        ConnectionRole::Public => match tier_of(global) {
            Tier::ShellOnly | Tier::ShellAndAgent => Advertise::No,
            Tier::Everyone => Advertise::Yes,
        },
        ConnectionRole::Sandboxed(_) => sandbox_sees(global),
    }
}

/// Which connections may see a global at all, before a sandbox narrows it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Tier {
    /// Every kind of connection.
    Everyone,
    /// The shell and the agent tier (never the public socket).
    ShellAndAgent,
    /// The shell only.
    ShellOnly,
}

/// The tier of `global`: everything is `Everyone` but the two `quire` management protocols.
fn tier_of(global: GlobalKind) -> Tier {
    match global {
        GlobalKind::QuireShell => Tier::ShellOnly,
        GlobalKind::QuireDesktop => Tier::ShellAndAgent,
        _ => Tier::Everyone,
    }
}

fn sandbox_sees(global: GlobalKind) -> Advertise {
    match global {
        GlobalKind::Compositor
        | GlobalKind::Subcompositor
        | GlobalKind::Shm
        | GlobalKind::Output
        | GlobalKind::Viewporter
        | GlobalKind::SinglePixel
        | GlobalKind::AlphaModifier
        | GlobalKind::Presentation
        | GlobalKind::FractionalScale
        | GlobalKind::Fifo
        | GlobalKind::CommitTiming
        | GlobalKind::ContentType
        | GlobalKind::XdgShell
        | GlobalKind::XdgDecoration
        | GlobalKind::XdgActivation
        | GlobalKind::XdgDialog
        | GlobalKind::XdgToplevelIcon
        | GlobalKind::XdgToplevelTag
        | GlobalKind::XdgForeign
        | GlobalKind::SystemBell
        | GlobalKind::QuireWindow
        | GlobalKind::Seat
        | GlobalKind::CursorShape
        | GlobalKind::PointerConstraints
        | GlobalKind::RelativePointer
        | GlobalKind::PointerGestures
        | GlobalKind::ShortcutsInhibit
        | GlobalKind::Tablet
        | GlobalKind::DataDevice
        | GlobalKind::PrimarySelection
        | GlobalKind::Dmabuf
        | GlobalKind::TextInput
        | GlobalKind::IdleNotify
        | GlobalKind::IdleInhibit
        | GlobalKind::BackgroundEffect => Advertise::Yes,
        GlobalKind::LayerShell
        | GlobalKind::SessionLock
        | GlobalKind::ExtDataControl
        | GlobalKind::WlrDataControl
        | GlobalKind::InputMethod
        | GlobalKind::VirtualKeyboard
        | GlobalKind::ForeignToplevelList
        | GlobalKind::SecurityContext
        | GlobalKind::OutputManagement
        | GlobalKind::OutputPower
        | GlobalKind::Gamma
        | GlobalKind::WlrToplevel
        | GlobalKind::Workspace
        | GlobalKind::ImageCapture
        | GlobalKind::ToplevelCapture
        | GlobalKind::QuireDesktop
        | GlobalKind::QuireShell => Advertise::No,
    }
}
