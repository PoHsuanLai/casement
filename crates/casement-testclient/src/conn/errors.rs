//! Which protocol errors are documented: every `error` enum of every interface the catalogue
//! reaches, as the generated bindings know it. The fuzz oracle "a documented protocol error or
//! nothing" asks here.

/// Whether `code` is an entry of the `error` enum of `interface`. `None` when the interface has
/// no `error` enum (any error it posts is undocumented) or is not in the catalogue.
pub(super) fn is_documented(interface: &str, code: u32) -> Option<bool> {
    macro_rules! table {
        ($(($name:literal, $error:ty)),* $(,)?) => {
            match interface {
                $($name => Some(<$error>::try_from(code).is_ok()),)*
                _ => None,
            }
        };
    }
    table![
        ("wl_display", wayland_client::protocol::wl_display::Error),
        // A bind of an unknown global or an impossible version is refused on the registry with
        // the core error codes (wl_registry has no error enum of its own).
        ("wl_registry", wayland_client::protocol::wl_display::Error),
        ("wl_shm_pool", wayland_client::protocol::wl_shm_pool::Error),
        ("wl_shm", wayland_client::protocol::wl_shm::Error),
        ("wl_data_offer", wayland_client::protocol::wl_data_offer::Error),
        ("wl_data_source", wayland_client::protocol::wl_data_source::Error),
        ("wl_data_device", wayland_client::protocol::wl_data_device::Error),
        ("wl_shell", wayland_client::protocol::wl_shell::Error),
        ("wl_surface", wayland_client::protocol::wl_surface::Error),
        ("wl_seat", wayland_client::protocol::wl_seat::Error),
        ("wl_pointer", wayland_client::protocol::wl_pointer::Error),
        ("wl_subcompositor", wayland_client::protocol::wl_subcompositor::Error),
        ("wl_subsurface", wayland_client::protocol::wl_subsurface::Error),
        ("wl_fixes", wayland_client::protocol::wl_fixes::Error),
        ("xdg_wm_base", wayland_protocols::xdg::shell::client::xdg_wm_base::Error),
        ("xdg_positioner", wayland_protocols::xdg::shell::client::xdg_positioner::Error),
        ("xdg_surface", wayland_protocols::xdg::shell::client::xdg_surface::Error),
        ("xdg_toplevel", wayland_protocols::xdg::shell::client::xdg_toplevel::Error),
        ("xdg_popup", wayland_protocols::xdg::shell::client::xdg_popup::Error),
        ("wp_viewporter", wayland_protocols::wp::viewporter::client::wp_viewporter::Error),
        ("wp_viewport", wayland_protocols::wp::viewporter::client::wp_viewport::Error),
        ("wp_presentation", wayland_protocols::wp::presentation_time::client::wp_presentation::Error),
        ("zwp_tablet_tool_v2", wayland_protocols::wp::tablet::zv2::client::zwp_tablet_tool_v2::Error),
        ("zwp_linux_buffer_params_v1", wayland_protocols::wp::linux_dmabuf::zv1::client::zwp_linux_buffer_params_v1::Error),
        ("wp_alpha_modifier_v1", wayland_protocols::wp::alpha_modifier::v1::client::wp_alpha_modifier_v1::Error),
        ("wp_alpha_modifier_surface_v1", wayland_protocols::wp::alpha_modifier::v1::client::wp_alpha_modifier_surface_v1::Error),
        ("wp_commit_timing_manager_v1", wayland_protocols::wp::commit_timing::v1::client::wp_commit_timing_manager_v1::Error),
        ("wp_commit_timer_v1", wayland_protocols::wp::commit_timing::v1::client::wp_commit_timer_v1::Error),
        ("wp_color_manager_v1", wayland_protocols::wp::color_management::v1::client::wp_color_manager_v1::Error),
        ("wp_color_management_surface_v1", wayland_protocols::wp::color_management::v1::client::wp_color_management_surface_v1::Error),
        ("wp_color_management_surface_feedback_v1", wayland_protocols::wp::color_management::v1::client::wp_color_management_surface_feedback_v1::Error),
        ("wp_image_description_creator_params_v1", wayland_protocols::wp::color_management::v1::client::wp_image_description_creator_params_v1::Error),
        ("wp_image_description_creator_icc_v1", wayland_protocols::wp::color_management::v1::client::wp_image_description_creator_icc_v1::Error),
        ("wp_image_description_v1", wayland_protocols::wp::color_management::v1::client::wp_image_description_v1::Error),
        ("wp_content_type_manager_v1", wayland_protocols::wp::content_type::v1::client::wp_content_type_manager_v1::Error),
        ("wp_cursor_shape_device_v1", wayland_protocols::wp::cursor_shape::v1::client::wp_cursor_shape_device_v1::Error),
        ("wp_fifo_manager_v1", wayland_protocols::wp::fifo::v1::client::wp_fifo_manager_v1::Error),
        ("wp_fifo_v1", wayland_protocols::wp::fifo::v1::client::wp_fifo_v1::Error),
        ("wp_fractional_scale_manager_v1", wayland_protocols::wp::fractional_scale::v1::client::wp_fractional_scale_manager_v1::Error),
        ("wp_security_context_manager_v1", wayland_protocols::wp::security_context::v1::client::wp_security_context_manager_v1::Error),
        ("wp_security_context_v1", wayland_protocols::wp::security_context::v1::client::wp_security_context_v1::Error),
        ("ext_background_effect_manager_v1", wayland_protocols::ext::background_effect::v1::client::ext_background_effect_manager_v1::Error),
        ("ext_background_effect_surface_v1", wayland_protocols::ext::background_effect::v1::client::ext_background_effect_surface_v1::Error),
        ("ext_data_control_device_v1", wayland_protocols::ext::data_control::v1::client::ext_data_control_device_v1::Error),
        ("ext_data_control_source_v1", wayland_protocols::ext::data_control::v1::client::ext_data_control_source_v1::Error),
        ("ext_session_lock_v1", wayland_protocols::ext::session_lock::v1::client::ext_session_lock_v1::Error),
        ("ext_session_lock_surface_v1", wayland_protocols::ext::session_lock::v1::client::ext_session_lock_surface_v1::Error),
        ("xdg_activation_token_v1", wayland_protocols::xdg::activation::v1::client::xdg_activation_token_v1::Error),
        ("xdg_wm_dialog_v1", wayland_protocols::xdg::dialog::v1::client::xdg_wm_dialog_v1::Error),
        ("xdg_toplevel_icon_v1", wayland_protocols::xdg::toplevel_icon::v1::client::xdg_toplevel_icon_v1::Error),
        ("zxdg_toplevel_decoration_v1", wayland_protocols::xdg::decoration::zv1::client::zxdg_toplevel_decoration_v1::Error),
        // get_toplevel_decoration on a toplevel that has one is refused on the manager with the
        // decoration's own codes (already_constructed, orphaned), as smithay and wlroots post it;
        // the manager has no error enum of its own.
        ("zxdg_decoration_manager_v1", wayland_protocols::xdg::decoration::zv1::client::zxdg_toplevel_decoration_v1::Error),
        ("zxdg_exporter_v2", wayland_protocols::xdg::foreign::zv2::client::zxdg_exporter_v2::Error),
        ("zxdg_imported_v2", wayland_protocols::xdg::foreign::zv2::client::zxdg_imported_v2::Error),
        ("zwp_keyboard_shortcuts_inhibit_manager_v1", wayland_protocols::wp::keyboard_shortcuts_inhibit::zv1::client::zwp_keyboard_shortcuts_inhibit_manager_v1::Error),
        ("zwp_pointer_constraints_v1", wayland_protocols::wp::pointer_constraints::zv1::client::zwp_pointer_constraints_v1::Error),
        ("zwp_text_input_v3", wayland_protocols::wp::text_input::zv3::client::zwp_text_input_v3::Error),
        ("zwlr_layer_shell_v1", wayland_protocols_wlr::layer_shell::v1::client::zwlr_layer_shell_v1::Error),
        ("zwlr_layer_surface_v1", wayland_protocols_wlr::layer_shell::v1::client::zwlr_layer_surface_v1::Error),
        ("zwlr_data_control_device_v1", wayland_protocols_wlr::data_control::v1::client::zwlr_data_control_device_v1::Error),
        ("zwlr_data_control_source_v1", wayland_protocols_wlr::data_control::v1::client::zwlr_data_control_source_v1::Error),
        ("zwlr_output_configuration_v1", wayland_protocols_wlr::output_management::v1::client::zwlr_output_configuration_v1::Error),
        ("zwlr_output_configuration_head_v1", wayland_protocols_wlr::output_management::v1::client::zwlr_output_configuration_head_v1::Error),
        ("zwlr_output_power_v1", wayland_protocols_wlr::output_power_management::v1::client::zwlr_output_power_v1::Error),
        ("zwlr_gamma_control_v1", wayland_protocols_wlr::gamma_control::v1::client::zwlr_gamma_control_v1::Error),
        ("zwp_virtual_keyboard_v1", wayland_protocols_misc::zwp_virtual_keyboard_v1::client::zwp_virtual_keyboard_v1::Error),
        ("zwp_virtual_keyboard_manager_v1", wayland_protocols_misc::zwp_virtual_keyboard_v1::client::zwp_virtual_keyboard_manager_v1::Error),
        ("quire_window_manager_v1", casement_protocols::client::quire_window_v1::quire_window_manager_v1::Error),
        ("quire_window_v1", casement_protocols::client::quire_window_v1::quire_window_v1::Error),
        ("quire_sensitive_manager_v1", casement_protocols::client::quire_sensitive_v1::quire_sensitive_manager_v1::Error),
        ("quire_sensitive_surface_v1", casement_protocols::client::quire_sensitive_v1::quire_sensitive_surface_v1::Error),
        ("quire_agent_manager_v1", casement_protocols::client::quire_agent_v1::quire_agent_manager_v1::Error),
        ("quire_trusted_manager_v1", casement_protocols::client::quire_trusted_surface_v1::quire_trusted_manager_v1::Error),
        ("quire_desktop_manager_v1", casement_protocols::client::quire_desktop_v1::quire_desktop_manager_v1::Error),
        ("quire_shell_v1", casement_protocols::client::quire_shell_v1::quire_shell_v1::Error),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_code_is_documented_when_its_interface_lists_it() {
        assert_eq!(is_documented("wl_display", 0), Some(true));
        assert_eq!(is_documented("wl_display", 3), Some(true));
        assert_eq!(is_documented("wl_display", 4), Some(false));
        assert_eq!(is_documented("xdg_wm_base", 4), Some(true));
        assert_eq!(is_documented("xdg_wm_base", 99), Some(false));
    }

    #[test]
    fn a_refused_bind_is_a_core_error_on_the_registry() {
        assert_eq!(is_documented("wl_registry", 0), Some(true));
        assert_eq!(is_documented("wl_registry", 4), Some(false));
    }

    #[test]
    fn an_interface_without_an_error_enum_has_no_documented_codes() {
        assert_eq!(is_documented("wl_callback", 0), None);
        assert_eq!(is_documented("no_such_interface", 0), None);
    }
}
