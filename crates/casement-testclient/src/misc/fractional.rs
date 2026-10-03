//! `wp_fractional_scale_v1` (P0-H1). The `preferred_scale` events are in the transcript and
//! [`crate::surface::subsurface::seen`] reads them off the scale object.

use crate::conn::TestClient;
use crate::surface::subsurface::{H1Data, Surface, quiet};
use wayland_protocols::wp::fractional_scale::v1::client::{
    wp_fractional_scale_manager_v1::WpFractionalScaleManagerV1,
    wp_fractional_scale_v1::WpFractionalScaleV1,
};

/// A bound `wp_fractional_scale_manager_v1`.
#[derive(Debug)]
pub struct FractionalScale {
    manager: WpFractionalScaleManagerV1,
}

impl FractionalScale {
    /// Binds the global (version 1).
    pub fn bind(client: &TestClient) -> FractionalScale {
        FractionalScale {
            manager: client
                .bind(1..=1, H1Data)
                .expect("the compositor advertises wp_fractional_scale_manager_v1"),
        }
    }

    /// The scale object of `surface`.
    pub fn for_surface(&self, client: &TestClient, surface: &Surface) -> WpFractionalScaleV1 {
        self.manager
            .get_fractional_scale(&surface.surface, &client.qh(), H1Data)
    }
}

/// The `preferred_scale` values `object` received (120ths), in order.
pub fn preferred(client: &TestClient, object: &WpFractionalScaleV1) -> Vec<u32> {
    crate::surface::subsurface::seen(client, object)
        .into_iter()
        .filter(|(event, _)| event == "preferred_scale")
        .filter_map(|(_, args)| args.first()?.parse().ok())
        .collect()
}

quiet!(WpFractionalScaleManagerV1, WpFractionalScaleV1);
