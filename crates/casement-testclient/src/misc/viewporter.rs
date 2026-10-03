//! `wp_viewporter`, plus the two other surface adjuncts of this lane that have no module of
//! their own in the frozen tree: `wp_single_pixel_buffer_manager_v1` and `wp_alpha_modifier_v1`
//! (P0-H1).

use crate::conn::TestClient;
use crate::surface::subsurface::{H1Data, Surface, quiet};
use wayland_client::protocol::wl_buffer::WlBuffer;
use wayland_protocols::wp::alpha_modifier::v1::client::{
    wp_alpha_modifier_surface_v1::WpAlphaModifierSurfaceV1, wp_alpha_modifier_v1::WpAlphaModifierV1,
};
use wayland_protocols::wp::single_pixel_buffer::v1::client::wp_single_pixel_buffer_manager_v1::WpSinglePixelBufferManagerV1;
use wayland_protocols::wp::viewporter::client::{
    wp_viewport::WpViewport, wp_viewporter::WpViewporter,
};

/// A bound `wp_viewporter`.
#[derive(Debug)]
pub struct Viewporter {
    manager: WpViewporter,
}

impl Viewporter {
    /// Binds the global (version 1).
    pub fn bind(client: &TestClient) -> Viewporter {
        Viewporter {
            manager: client
                .bind(1..=1, H1Data)
                .expect("the compositor advertises wp_viewporter"),
        }
    }

    /// A viewport on `surface`.
    pub fn viewport(&self, client: &TestClient, surface: &Surface) -> Viewport {
        Viewport {
            viewport: self
                .manager
                .get_viewport(&surface.surface, &client.qh(), H1Data),
        }
    }
}

/// A `wp_viewport`.
#[derive(Debug)]
pub struct Viewport {
    /// The object.
    pub viewport: WpViewport,
}

impl Viewport {
    /// Crops to the buffer rectangle (`x`, `y`, `w`, `h`), in buffer pixels.
    pub fn source(&self, x: f64, y: f64, w: f64, h: f64) {
        self.viewport.set_source(x, y, w, h);
    }

    /// Scales to `w` by `h` surface-local pixels.
    pub fn destination(&self, w: i32, h: i32) {
        self.viewport.set_destination(w, h);
    }

    /// Destroys the viewport object.
    pub fn destroy(&self) {
        self.viewport.destroy();
    }
}

/// A bound `wp_single_pixel_buffer_manager_v1`.
#[derive(Debug)]
pub struct SinglePixel {
    manager: WpSinglePixelBufferManagerV1,
}

impl SinglePixel {
    /// Binds the global (version 1).
    pub fn bind(client: &TestClient) -> SinglePixel {
        SinglePixel {
            manager: client
                .bind(1..=1, H1Data)
                .expect("the compositor advertises wp_single_pixel_buffer_manager_v1"),
        }
    }

    /// A one-pixel buffer of this colour (premultiplied 32-bit components, 0 to `u32::MAX`).
    pub fn buffer(&self, client: &TestClient, rgba: [u32; 4]) -> WlBuffer {
        self.manager.create_u32_rgba_buffer(
            rgba[0],
            rgba[1],
            rgba[2],
            rgba[3],
            &client.qh(),
            H1Data,
        )
    }
}

/// A bound `wp_alpha_modifier_v1`.
#[derive(Debug)]
pub struct AlphaModifier {
    manager: WpAlphaModifierV1,
}

impl AlphaModifier {
    /// Binds the global (version 1).
    pub fn bind(client: &TestClient) -> AlphaModifier {
        AlphaModifier {
            manager: client
                .bind(1..=1, H1Data)
                .expect("the compositor advertises wp_alpha_modifier_v1"),
        }
    }

    /// The alpha object of `surface`.
    pub fn for_surface(&self, client: &TestClient, surface: &Surface) -> WpAlphaModifierSurfaceV1 {
        self.manager
            .get_surface(&surface.surface, &client.qh(), H1Data)
    }
}

quiet!(
    WpViewporter,
    WpViewport,
    WpSinglePixelBufferManagerV1,
    WpAlphaModifierV1,
    WpAlphaModifierSurfaceV1
);
