//! The frozen aggregate of every protocol's sub-state, and the global filter (I7).

use crate::client::{ClientMint, ClientState};
#[cfg(feature = "quire")]
use crate::quire;
use crate::{
    alpha_modifier, background_effect, capture_source, color_management, commit_timing, compositor,
    content_type, cursor_shape, data_device, dmabuf, ext_data_control, fifo, foreign_toplevel_list,
    fractional_scale, gamma, idle_inhibit, idle_notify, image_capture, input_method, layer_shell,
    output, output_management, output_power, pointer_constraints, pointer_gestures, presentation,
    primary_selection, quire_window, relative_pointer, seat, security_context, session_lock, shm,
    shortcuts_inhibit, single_pixel, subcompositor, system_bell, tablet, text_input, viewporter,
    virtual_keyboard, wlr_data_control, wlr_toplevel, workspace, xdg_activation, xdg_decoration,
    xdg_dialog, xdg_foreign, xdg_shell, xdg_toplevel_icon, xdg_toplevel_tag,
};
use casement_core::WindowId;
use casement_core::policy::{ConnectionRole, GlobalKind, PowerSet, advertised_to};
use smithay::input::SeatHandler;
use smithay::reexports::wayland_server::Client;

/// One field per protocol module: what the composition root holds as `State::wl`. Lanes fill
/// each module's functions; the field set changes only in a freeze (each delta freeze adds its
/// modules' parts). Generic over the composition root's state type `D` only because smithay's
/// seat is.
#[derive(Debug)]
pub struct WaylandStates<D: SeatHandler + 'static> {
    /// Client keys minted as clients connect.
    pub clients: ClientMint,
    /// `wl_compositor`.
    pub compositor: compositor::CompositorPart,
    /// `wl_subcompositor`.
    pub subcompositor: subcompositor::SubcompositorPart,
    /// `wl_shm`.
    pub shm: shm::ShmPart,
    /// `wl_output`, `zxdg_output_manager_v1`.
    pub output: output::OutputPart,
    /// `wp_viewporter`.
    pub viewporter: viewporter::ViewporterPart,
    /// `wp_single_pixel_buffer_manager_v1`.
    pub single_pixel: single_pixel::SinglePixelPart,
    /// `wp_alpha_modifier_v1`.
    pub alpha_modifier: alpha_modifier::AlphaModifierPart,
    /// `wp_presentation`.
    pub presentation: presentation::PresentationPart,
    /// `wp_fractional_scale_manager_v1`.
    pub fractional_scale: fractional_scale::FractionalScalePart,
    /// `wp_fifo_manager_v1`.
    pub fifo: fifo::FifoPart,
    /// `wp_commit_timing_manager_v1`.
    pub commit_timing: commit_timing::CommitTimingPart,
    /// `wp_color_manager_v1`.
    pub color_management: color_management::ColorPart,
    /// `wp_content_type_manager_v1`.
    pub content_type: content_type::ContentTypePart,
    /// `xdg_wm_base`.
    pub xdg_shell: xdg_shell::XdgShellPart,
    /// `zxdg_decoration_manager_v1`.
    pub xdg_decoration: xdg_decoration::XdgDecorationPart,
    /// `xdg_activation_v1`.
    pub xdg_activation: xdg_activation::XdgActivationPart,
    /// `xdg_wm_dialog_v1`.
    pub xdg_dialog: xdg_dialog::XdgDialogPart,
    /// `xdg_toplevel_icon_manager_v1`.
    pub xdg_toplevel_icon: xdg_toplevel_icon::XdgToplevelIconPart,
    /// `xdg_toplevel_tag_manager_v1`.
    pub xdg_toplevel_tag: xdg_toplevel_tag::XdgToplevelTagPart,
    /// `zxdg_exporter_v2`, `zxdg_importer_v2`.
    pub xdg_foreign: xdg_foreign::XdgForeignPart,
    /// `xdg_system_bell_v1`.
    pub system_bell: system_bell::SystemBellPart,
    /// `quire_window_manager_v1` (C50).
    pub quire_window: quire_window::QuireWindowPart,
    /// `zwlr_layer_shell_v1`.
    pub layer_shell: layer_shell::LayerShellPart,
    /// `ext_session_lock_manager_v1`.
    pub session_lock: session_lock::SessionLockPart,
    /// `wl_seat`.
    pub seat: seat::SeatPart<D>,
    /// `wp_cursor_shape_manager_v1`.
    pub cursor_shape: cursor_shape::CursorShapePart,
    /// `zwp_pointer_constraints_v1`.
    pub pointer_constraints: pointer_constraints::PointerConstraintsPart,
    /// `zwp_relative_pointer_manager_v1`.
    pub relative_pointer: relative_pointer::RelativePointerPart,
    /// `zwp_pointer_gestures_v1`.
    pub pointer_gestures: pointer_gestures::PointerGesturesPart,
    /// `zwp_keyboard_shortcuts_inhibit_manager_v1`.
    pub shortcuts_inhibit: shortcuts_inhibit::ShortcutsInhibitPart,
    /// `zwp_tablet_manager_v2`.
    pub tablet: tablet::TabletPart,
    /// `wl_data_device_manager`.
    pub data_device: data_device::DataDevicePart,
    /// `zwp_primary_selection_device_manager_v1`.
    pub primary_selection: primary_selection::PrimarySelectionPart,
    /// `ext_data_control_manager_v1`.
    pub ext_data_control: ext_data_control::ExtDataControlPart,
    /// `zwlr_data_control_manager_v1`.
    pub wlr_data_control: wlr_data_control::WlrDataControlPart,
    /// `zwp_linux_dmabuf_v1` (advertised only with `--dmabuf soft` and from P1 on hardware).
    pub dmabuf: dmabuf::DmabufPart,
    /// `zwp_text_input_manager_v3`.
    pub text_input: text_input::TextInputPart,
    /// `zwp_input_method_manager_v2`.
    pub input_method: input_method::InputMethodPart,
    /// `zwp_virtual_keyboard_manager_v1`.
    pub virtual_keyboard: virtual_keyboard::VirtualKeyboardPart,
    /// `ext_idle_notifier_v1`.
    pub idle_notify: idle_notify::IdleNotifyPart,
    /// `zwp_idle_inhibit_manager_v1`.
    pub idle_inhibit: idle_inhibit::IdleInhibitPart,
    /// `ext_foreign_toplevel_list_v1`.
    pub foreign_toplevel_list: foreign_toplevel_list::ForeignToplevelListPart,
    /// `wp_security_context_manager_v1`.
    pub security_context: security_context::SecurityContextPart,
    /// `ext_background_effect_manager_v1`.
    pub background_effect: background_effect::BackgroundEffectPart,
    /// `zwlr_output_manager_v1` (declared at P1-Z, advertised by P1-M1).
    pub output_management: output_management::OutputManagementPart,
    /// `zwlr_output_power_manager_v1` (declared at P1-Z, advertised by P1-M1).
    pub output_power: output_power::OutputPowerPart,
    /// `zwlr_gamma_control_manager_v1` (declared at P1-Z, advertised by P1-M1).
    pub gamma: gamma::GammaPart,
    /// `zwlr_foreign_toplevel_manager_v1` (declared at P1-Z, advertised by P1-M2).
    pub wlr_toplevel: wlr_toplevel::WlrToplevelPart,
    /// `ext_workspace_manager_v1` (declared at P1-Z, advertised by P1-M2).
    pub workspace: workspace::WorkspacePart,
    /// `ext_output_image_capture_source_manager_v1`, `ext_image_copy_capture_manager_v1`
    /// (declared at P1-Z, advertised by P1-C1).
    pub image_capture: image_capture::ImageCapturePart,
    /// `ext_foreign_toplevel_image_capture_source_manager_v1` (declared at P2-Z, advertised by
    /// P2-Q3).
    pub capture_source: capture_source::ToplevelCapturePart,
    /// `zwlr_screencopy_manager_v1`: the older output capture (`grim`).
    pub screencopy: crate::screencopy::ScreencopyPart,
    /// `quire_desktop_manager_v1` (declared at P2-Z, advertised by P2-Q1; shell and agent
    /// connections only).
    #[cfg(feature = "quire")]
    pub quire_desktop: quire::desktop::QuireDesktopPart,
    /// `quire_shell_v1` (declared at P2-Z, advertised by P2-Q2; shell connections only).
    #[cfg(feature = "quire")]
    pub quire_shell: quire::shell::QuireShellPart,
}

/// Which globals a connection sees: the decision function takes the global, the connection's
/// role and the powers it was refused at connect time (`casement_core::policy::advertised_to`,
/// I7). Copied into every filtered global's `can_view`.
#[derive(Debug, Clone, Copy, Default)]
pub struct GlobalFilter;

impl GlobalFilter {
    /// The filter the compositor runs with: the model's I7 table, with the powers a public
    /// client was refused at accept hidden from it.
    pub fn policy() -> GlobalFilter {
        GlobalFilter
    }

    /// Whether `client` sees `global`. A client casement did not insert (no [`ClientState`])
    /// is treated as arriving through the public socket, refused nothing: it cannot use a
    /// power anyway ([`crate::consent::admit`] refuses it).
    pub fn sees(self, global: GlobalKind, client: &Client) -> bool {
        match ClientState::of(client) {
            Some(state) => advertised_to(global, &state.role, &state.hidden),
            None => advertised_to(global, &ConnectionRole::Public, &PowerSet::default()),
        }
    }

    /// `can_view` for one global, as smithay's filtered constructors take it.
    pub fn for_global(
        self,
        global: GlobalKind,
    ) -> impl for<'c> Fn(&'c Client) -> bool + Send + Sync + 'static {
        move |client| self.sees(global, client)
    }
}

/// A wire-side failure: an effect named something the protocol layer no longer has.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum WaylandFault {
    /// The window is not known to this protocol module.
    #[error("unknown window {0:?}")]
    UnknownWindow(WindowId),
    /// The surface was destroyed before the effect arrived.
    #[error("surface gone")]
    SurfaceGone,
    /// The keymap could not be compiled (the seat's keyboard cannot start).
    #[error("keymap: {0}")]
    Keymap(String),
    /// A capture source or session this protocol module does not know (its number).
    #[error("unknown capture {0}")]
    UnknownCapture(u64),
}
