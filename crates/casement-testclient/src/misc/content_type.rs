//! `wp_content_type_v1` (P0-H1).

use crate::conn::TestClient;
use crate::surface::subsurface::{H1Data, Surface, quiet};
pub use wayland_protocols::wp::content_type::v1::client::wp_content_type_v1::Type;
use wayland_protocols::wp::content_type::v1::client::{
    wp_content_type_manager_v1::WpContentTypeManagerV1, wp_content_type_v1::WpContentTypeV1,
};

/// A bound `wp_content_type_manager_v1`.
#[derive(Debug)]
pub struct ContentTypes {
    /// The manager.
    pub manager: WpContentTypeManagerV1,
}

impl ContentTypes {
    /// Binds the global (version 1).
    pub fn bind(client: &TestClient) -> ContentTypes {
        ContentTypes {
            manager: client
                .bind(1..=1, H1Data)
                .expect("the compositor advertises wp_content_type_manager_v1"),
        }
    }

    /// The content-type object of `surface`.
    pub fn for_surface(&self, client: &TestClient, surface: &Surface) -> WpContentTypeV1 {
        self.manager
            .get_surface_content_type(&surface.surface, &client.qh(), H1Data)
    }
}

/// Sets what `object`'s surface shows, for the next commit.
pub fn set(object: &WpContentTypeV1, kind: Type) {
    object.set_content_type(kind);
}

quiet!(WpContentTypeManagerV1, WpContentTypeV1);
