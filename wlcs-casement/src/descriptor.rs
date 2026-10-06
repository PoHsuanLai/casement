//! The extensions casement serves that wlcs has tests for, in wlcs' descriptor format: wlcs
//! skips the tests of an interface that is neither listed nor served. Versions are the ones in
//! `casement-testkit/tests/golden/globals.txt`. Left out, because casement does not serve them:
//! `wl_shell`, `zxdg_shell_v6`, `zwlr_virtual_pointer_manager_v1`,
//! `zwlr_foreign_toplevel_manager_v1`, the GTK primary selection and text-input v2.

use wlcs::extension_list;
use wlcs::ffi_display_server_api::{WlcsExtensionDescriptor, WlcsIntegrationDescriptor};

static EXTENSIONS: &[WlcsExtensionDescriptor] = extension_list!(
    ("wl_compositor", 5),
    ("wl_subcompositor", 1),
    ("wl_data_device_manager", 3),
    ("wl_seat", 9),
    ("wl_output", 4),
    ("xdg_wm_base", 7),
    ("zxdg_output_manager_v1", 3),
    ("zxdg_decoration_manager_v1", 1),
    ("zwp_relative_pointer_manager_v1", 1),
    ("zwp_pointer_constraints_v1", 1),
    ("zwp_primary_selection_device_manager_v1", 1),
    ("zwlr_layer_shell_v1", 5),
    ("wp_viewporter", 1),
    ("wp_fractional_scale_manager_v1", 1),
    ("zwp_text_input_manager_v3", 1),
    ("zwp_input_method_manager_v2", 1),
);

/// The descriptor `get_descriptor` hands wlcs.
pub static DESCRIPTOR: WlcsIntegrationDescriptor = WlcsIntegrationDescriptor {
    version: 1,
    num_extensions: EXTENSIONS.len(),
    supported_extensions: EXTENSIONS.as_ptr(),
};
