//! The extensions the stand-in compositor serves, in wlcs' descriptor format: wlcs skips the
//! tests of an interface that is not listed. smithay's `wlcs_anvil` lists six; the stand-in also
//! serves `zxdg_output_manager_v1` and `zwp_relative_pointer_manager_v1`, so the XdgOutput and
//! RelativePointer filters of smithay's CI run instead of skipping.

use wlcs::extension_list;
use wlcs::ffi_display_server_api::{WlcsExtensionDescriptor, WlcsIntegrationDescriptor};

static EXTENSIONS: &[WlcsExtensionDescriptor] = extension_list!(
    ("wl_compositor", 4),
    ("wl_subcompositor", 1),
    ("wl_data_device_manager", 3),
    ("wl_seat", 7),
    ("wl_output", 4),
    ("xdg_wm_base", 3),
    ("zxdg_output_manager_v1", 3),
    ("zwp_relative_pointer_manager_v1", 1),
);

/// The descriptor `get_descriptor` hands wlcs.
pub static DESCRIPTOR: WlcsIntegrationDescriptor = WlcsIntegrationDescriptor {
    version: 1,
    num_extensions: EXTENSIONS.len(),
    supported_extensions: EXTENSIONS.as_ptr(),
};
