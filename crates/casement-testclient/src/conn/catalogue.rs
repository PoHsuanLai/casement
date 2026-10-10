//! Every interface the clients may meet, by name: the roots below (each global casement
//! advertises, plus the core ones) and everything their messages create or name. The tap decodes
//! with it and the fuzz generator picks requests from it; an interface not listed here still
//! passes through, it is only shown undecoded.

use std::collections::BTreeMap;
use std::sync::OnceLock;
use wayland_backend::protocol::Interface;
use wayland_client::Proxy;

/// The interface called `name`, when the catalogue knows it.
pub(super) fn lookup(name: &str) -> Option<&'static Interface> {
    all().get(name).copied()
}

/// Every known interface, by name.
pub(super) fn all() -> &'static BTreeMap<&'static str, &'static Interface> {
    static ALL: OnceLock<BTreeMap<&'static str, &'static Interface>> = OnceLock::new();
    ALL.get_or_init(|| {
        let mut found = BTreeMap::new();
        let mut pending: Vec<&'static Interface> = roots();
        while let Some(interface) = pending.pop() {
            if found.insert(interface.name, interface).is_none() {
                for message in interface.requests.iter().chain(interface.events) {
                    pending.extend(message.child_interface);
                    pending.extend(message.arg_interfaces.iter().copied());
                }
            }
        }
        found
    })
}

macro_rules! roots_of {
    ($($proxy:ty),* $(,)?) => {
        vec![$(<$proxy as Proxy>::interface()),*]
    };
}

fn roots() -> Vec<&'static Interface> {
    use casement_protocols::client as quire;
    use wayland_client::protocol as core;
    use wayland_protocols::{ext, wp, xdg};
    use wayland_protocols_misc as misc;
    use wayland_protocols_wlr as wlr;
    roots_of![
        core::wl_display::WlDisplay,
        core::wl_registry::WlRegistry,
        core::wl_callback::WlCallback,
        core::wl_compositor::WlCompositor,
        core::wl_subcompositor::WlSubcompositor,
        core::wl_shm::WlShm,
        core::wl_output::WlOutput,
        core::wl_seat::WlSeat,
        core::wl_data_device_manager::WlDataDeviceManager,
        wp::viewporter::client::wp_viewporter::WpViewporter,
        wp::single_pixel_buffer::v1::client::wp_single_pixel_buffer_manager_v1::WpSinglePixelBufferManagerV1,
        wp::alpha_modifier::v1::client::wp_alpha_modifier_v1::WpAlphaModifierV1,
        wp::presentation_time::client::wp_presentation::WpPresentation,
        wp::fractional_scale::v1::client::wp_fractional_scale_manager_v1::WpFractionalScaleManagerV1,
        wp::fifo::v1::client::wp_fifo_manager_v1::WpFifoManagerV1,
        wp::commit_timing::v1::client::wp_commit_timing_manager_v1::WpCommitTimingManagerV1,
        wp::content_type::v1::client::wp_content_type_manager_v1::WpContentTypeManagerV1,
        wp::cursor_shape::v1::client::wp_cursor_shape_manager_v1::WpCursorShapeManagerV1,
        wp::security_context::v1::client::wp_security_context_manager_v1::WpSecurityContextManagerV1,
        wp::linux_dmabuf::zv1::client::zwp_linux_dmabuf_v1::ZwpLinuxDmabufV1,
        wp::pointer_constraints::zv1::client::zwp_pointer_constraints_v1::ZwpPointerConstraintsV1,
        wp::relative_pointer::zv1::client::zwp_relative_pointer_manager_v1::ZwpRelativePointerManagerV1,
        wp::pointer_gestures::zv1::client::zwp_pointer_gestures_v1::ZwpPointerGesturesV1,
        wp::keyboard_shortcuts_inhibit::zv1::client::zwp_keyboard_shortcuts_inhibit_manager_v1::ZwpKeyboardShortcutsInhibitManagerV1,
        wp::tablet::zv2::client::zwp_tablet_manager_v2::ZwpTabletManagerV2,
        wp::primary_selection::zv1::client::zwp_primary_selection_device_manager_v1::ZwpPrimarySelectionDeviceManagerV1,
        wp::text_input::zv3::client::zwp_text_input_manager_v3::ZwpTextInputManagerV3,
        wp::idle_inhibit::zv1::client::zwp_idle_inhibit_manager_v1::ZwpIdleInhibitManagerV1,
        xdg::shell::client::xdg_wm_base::XdgWmBase,
        xdg::decoration::zv1::client::zxdg_decoration_manager_v1::ZxdgDecorationManagerV1,
        xdg::activation::v1::client::xdg_activation_v1::XdgActivationV1,
        xdg::dialog::v1::client::xdg_wm_dialog_v1::XdgWmDialogV1,
        xdg::toplevel_icon::v1::client::xdg_toplevel_icon_manager_v1::XdgToplevelIconManagerV1,
        xdg::toplevel_tag::v1::client::xdg_toplevel_tag_manager_v1::XdgToplevelTagManagerV1,
        xdg::foreign::zv2::client::zxdg_exporter_v2::ZxdgExporterV2,
        xdg::foreign::zv2::client::zxdg_importer_v2::ZxdgImporterV2,
        xdg::system_bell::v1::client::xdg_system_bell_v1::XdgSystemBellV1,
        xdg::xdg_output::zv1::client::zxdg_output_manager_v1::ZxdgOutputManagerV1,
        ext::session_lock::v1::client::ext_session_lock_manager_v1::ExtSessionLockManagerV1,
        ext::data_control::v1::client::ext_data_control_manager_v1::ExtDataControlManagerV1,
        ext::idle_notify::v1::client::ext_idle_notifier_v1::ExtIdleNotifierV1,
        ext::foreign_toplevel_list::v1::client::ext_foreign_toplevel_list_v1::ExtForeignToplevelListV1,
        ext::background_effect::v1::client::ext_background_effect_manager_v1::ExtBackgroundEffectManagerV1,
        ext::workspace::v1::client::ext_workspace_manager_v1::ExtWorkspaceManagerV1,
        wlr::layer_shell::v1::client::zwlr_layer_shell_v1::ZwlrLayerShellV1,
        wlr::data_control::v1::client::zwlr_data_control_manager_v1::ZwlrDataControlManagerV1,
        wlr::foreign_toplevel::v1::client::zwlr_foreign_toplevel_manager_v1::ZwlrForeignToplevelManagerV1,
        misc::zwp_input_method_v2::client::zwp_input_method_manager_v2::ZwpInputMethodManagerV2,
        misc::zwp_virtual_keyboard_v1::client::zwp_virtual_keyboard_manager_v1::ZwpVirtualKeyboardManagerV1,
        quire::quire_window_v1::quire_window_v1::QuireWindowV1,
        quire::quire_window_v1::quire_window_manager_v1::QuireWindowManagerV1,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_core_interfaces_and_their_children_are_known() {
        for name in [
            "wl_display",
            "wl_registry",
            "wl_surface",
            "wl_region",
            "wl_shm_pool",
            "wl_buffer",
            "wl_keyboard",
            "xdg_surface",
            "xdg_toplevel",
            "xdg_popup",
            "xdg_positioner",
        ] {
            assert!(lookup(name).is_some(), "{name} is not in the catalogue");
        }
    }

    #[test]
    fn every_global_in_the_golden_but_dmabuf_has_an_entry() {
        for name in [
            "wl_compositor",
            "wp_viewporter",
            "xdg_wm_base",
            "zwlr_layer_shell_v1",
            "ext_session_lock_manager_v1",
            "zwp_input_method_manager_v2",
            "wp_cursor_shape_manager_v1",
            "quire_window_manager_v1",
        ] {
            assert!(lookup(name).is_some(), "{name} is not in the catalogue");
        }
    }
}
