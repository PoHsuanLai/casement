//! `wp_fifo_v1` (P0-H1).

use crate::conn::TestClient;
use crate::surface::subsurface::{H1Data, Surface, quiet};
use wayland_protocols::wp::fifo::v1::client::{
    wp_fifo_manager_v1::WpFifoManagerV1, wp_fifo_v1::WpFifoV1,
};

/// A bound `wp_fifo_manager_v1`.
#[derive(Debug)]
pub struct Fifo {
    /// The manager.
    pub manager: WpFifoManagerV1,
}

impl Fifo {
    /// Binds the global (version 1).
    pub fn bind(client: &TestClient) -> Fifo {
        Fifo {
            manager: client
                .bind(1..=1, H1Data)
                .expect("the compositor advertises wp_fifo_manager_v1"),
        }
    }

    /// The fifo object of `surface`.
    pub fn for_surface(&self, client: &TestClient, surface: &Surface) -> WpFifoV1 {
        self.manager
            .get_fifo(&surface.surface, &client.qh(), H1Data)
    }
}

quiet!(WpFifoManagerV1, WpFifoV1);
