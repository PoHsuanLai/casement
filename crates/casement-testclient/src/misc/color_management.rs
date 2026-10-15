//! `wp_color_manager_v1` and its objects. The events (`supported_*`, `ready2`, `failed`, the
//! information) are in the transcript; [`crate::surface::subsurface::seen`] reads them off an
//! object.

use crate::conn::TestClient;
use crate::surface::subsurface::{H1Data, Surface, quiet};
use wayland_client::protocol::wl_output::WlOutput;
pub use wayland_protocols::wp::color_management::v1::client::wp_color_manager_v1::{
    Primaries, RenderIntent, TransferFunction,
};
use wayland_protocols::wp::color_management::v1::client::{
    wp_color_management_output_v1::WpColorManagementOutputV1,
    wp_color_management_surface_feedback_v1::WpColorManagementSurfaceFeedbackV1,
    wp_color_management_surface_v1::WpColorManagementSurfaceV1,
    wp_color_manager_v1::WpColorManagerV1,
    wp_image_description_creator_icc_v1::WpImageDescriptionCreatorIccV1,
    wp_image_description_creator_params_v1::WpImageDescriptionCreatorParamsV1,
    wp_image_description_info_v1::WpImageDescriptionInfoV1,
    wp_image_description_v1::WpImageDescriptionV1,
};

/// A bound `wp_color_manager_v1`.
#[derive(Debug)]
pub struct ColorManagement {
    /// The manager.
    pub manager: WpColorManagerV1,
}

impl ColorManagement {
    /// Binds the global at exactly `version` (1 or 2).
    pub fn bind(client: &TestClient, version: u32) -> ColorManagement {
        ColorManagement {
            manager: client
                .bind(version..=version, H1Data)
                .expect("the compositor advertises wp_color_manager_v1"),
        }
    }

    /// A parametric creator.
    pub fn creator(&self, client: &TestClient) -> WpImageDescriptionCreatorParamsV1 {
        self.manager.create_parametric_creator(&client.qh(), H1Data)
    }

    /// An ICC creator (not served: the compositor answers with a protocol error).
    pub fn icc_creator(&self, client: &TestClient) -> WpImageDescriptionCreatorIccV1 {
        self.manager.create_icc_creator(&client.qh(), H1Data)
    }

    /// The colour object of `output`.
    pub fn output(&self, client: &TestClient, output: &WlOutput) -> WpColorManagementOutputV1 {
        self.manager.get_output(output, &client.qh(), H1Data)
    }

    /// The colour object of `surface`.
    pub fn surface(&self, client: &TestClient, surface: &Surface) -> WpColorManagementSurfaceV1 {
        self.manager
            .get_surface(&surface.surface, &client.qh(), H1Data)
    }

    /// The feedback object of `surface`.
    pub fn feedback(
        &self,
        client: &TestClient,
        surface: &Surface,
    ) -> WpColorManagementSurfaceFeedbackV1 {
        self.manager
            .get_surface_feedback(&surface.surface, &client.qh(), H1Data)
    }
}

/// A description made from sRGB primaries and `transfer`, made ready (or not) by the compositor.
pub fn describe(
    client: &TestClient,
    creator: WpImageDescriptionCreatorParamsV1,
    primaries: Primaries,
    transfer: TransferFunction,
) -> WpImageDescriptionV1 {
    creator.set_primaries_named(primaries);
    creator.set_tf_named(transfer);
    creator.create(&client.qh(), H1Data)
}

/// The information object of `description`.
pub fn information(
    client: &TestClient,
    description: &WpImageDescriptionV1,
) -> WpImageDescriptionInfoV1 {
    description.get_information(&client.qh(), H1Data)
}

quiet!(
    WpColorManagerV1,
    WpColorManagementOutputV1,
    WpColorManagementSurfaceV1,
    WpColorManagementSurfaceFeedbackV1,
    WpImageDescriptionCreatorIccV1,
    WpImageDescriptionCreatorParamsV1,
    WpImageDescriptionInfoV1,
    WpImageDescriptionV1,
);
