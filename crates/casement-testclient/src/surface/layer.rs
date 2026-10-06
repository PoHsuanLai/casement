//! layer-shell surfaces: anchors, exclusive zones, keyboard interactivity. Filled by P0-H3.
//!
//! The wire events reach the client's transcript through the connection's tap; the typed
//! handle here sends requests and remembers the one thing a request needs back, the latest
//! configure serial.

use crate::conn::{ClientError, ClientState, TestClient};
use crate::shm::ShmBuffer;
use std::sync::{Arc, Mutex, PoisonError};
use wayland_client::protocol::wl_compositor::WlCompositor;
use wayland_client::protocol::wl_surface::WlSurface;
use wayland_client::{Connection, Dispatch, QueueHandle};
use wayland_protocols::xdg::shell::client::xdg_popup::XdgPopup;
use wayland_protocols_wlr::layer_shell::v1::client::zwlr_layer_shell_v1::{self, ZwlrLayerShellV1};
use wayland_protocols_wlr::layer_shell::v1::client::zwlr_layer_surface_v1::{
    self, ZwlrLayerSurfaceV1,
};

pub use wayland_protocols_wlr::layer_shell::v1::client::zwlr_layer_shell_v1::Layer;
pub use wayland_protocols_wlr::layer_shell::v1::client::zwlr_layer_surface_v1::{
    Anchor, KeyboardInteractivity,
};

/// What a layer surface asks for.
#[derive(Debug, Clone)]
pub struct LayerAsk {
    /// Which layer.
    pub level: Layer,
    /// Its namespace.
    pub namespace: &'static str,
    /// The edges it is held to.
    pub anchor: Anchor,
    /// Its exclusive zone (`-1`, `0` or the depth in logical pixels).
    pub exclusive_zone: i32,
    /// Whether it takes the keyboard.
    pub keyboard: KeyboardInteractivity,
    /// The size it asks for (`0` along an axis it is held to both ends of).
    pub size: (u32, u32),
}

/// A configure the compositor sent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LayerConfigure {
    /// Its serial.
    pub serial: u32,
    /// The width it suggests.
    pub width: u32,
    /// The height it suggests.
    pub height: u32,
}

/// What a layer surface's events left behind: the latest configure and whether it closed.
#[derive(Debug, Clone, Default)]
pub struct LayerLog(Arc<Mutex<LayerState>>);

#[derive(Debug, Default)]
struct LayerState {
    configure: Option<LayerConfigure>,
    closed: bool,
}

impl LayerLog {
    fn with<T>(&self, f: impl FnOnce(&mut LayerState) -> T) -> T {
        f(&mut self.0.lock().unwrap_or_else(PoisonError::into_inner))
    }
}

/// The user data of the objects a layer surface needs that carry no state.
#[derive(Debug, Clone, Copy, Default)]
pub struct LayerPlain;

/// One layer surface of a test client.
#[derive(Debug)]
pub struct LayerSurface {
    surface: WlSurface,
    layer: ZwlrLayerSurfaceV1,
    log: LayerLog,
}

impl LayerSurface {
    /// Creates a surface and a layer surface for it on the compositor's choice of output, sets
    /// what `ask` says and makes the first commit (which asks for the initial configure).
    pub fn create(client: &TestClient, ask: &LayerAsk) -> Result<LayerSurface, ClientError> {
        let qh = client.qh();
        let compositor: WlCompositor = client.bind(1..=5, LayerPlain)?;
        let shell: ZwlrLayerShellV1 = client.bind(1..=5, LayerPlain)?;
        let surface = compositor.create_surface(&qh, LayerPlain);
        let log = LayerLog::default();
        let layer = shell.get_layer_surface(
            &surface,
            None,
            ask.level,
            ask.namespace.to_owned(),
            &qh,
            log.clone(),
        );
        layer.set_size(ask.size.0, ask.size.1);
        layer.set_anchor(ask.anchor);
        layer.set_exclusive_zone(ask.exclusive_zone);
        layer.set_keyboard_interactivity(ask.keyboard);
        surface.commit();
        let _ = client.connection().flush();
        Ok(LayerSurface {
            surface,
            layer,
            log,
        })
    }

    /// The latest configure the compositor sent, if any.
    pub fn configure(&self) -> Option<LayerConfigure> {
        self.log.with(|state| state.configure)
    }

    /// Whether the compositor closed the surface.
    pub fn is_closed(&self) -> bool {
        self.log.with(|state| state.closed)
    }

    /// Acknowledges the latest configure; false when there is none yet.
    pub fn ack(&self, client: &TestClient) -> bool {
        let Some(configure) = self.configure() else {
            return false;
        };
        self.layer.ack_configure(configure.serial);
        let _ = client.connection().flush();
        true
    }

    /// Attaches `buffer` over the whole surface and commits.
    pub fn show(&self, client: &TestClient, buffer: &ShmBuffer) {
        let size = buffer.size();
        self.surface.attach(Some(buffer.buffer()), 0, 0);
        self.surface.damage_buffer(
            0,
            0,
            i32::try_from(size.w).unwrap_or(i32::MAX),
            i32::try_from(size.h).unwrap_or(i32::MAX),
        );
        self.surface.commit();
        let _ = client.connection().flush();
    }

    /// Removes the buffer and commits (the surface unmaps).
    pub fn hide(&self, client: &TestClient) {
        self.surface.attach(None, 0, 0);
        self.surface.commit();
        let _ = client.connection().flush();
    }

    /// Makes `popup` (an xdg popup made without a parent) a popup of this layer surface.
    pub fn adopt(&self, popup: &XdgPopup) {
        self.layer.get_popup(popup);
    }

    /// The `wl_surface`, for requests of other protocols (a drag from it, a cursor).
    pub fn surface(&self) -> &WlSurface {
        &self.surface
    }

    /// Destroys the `wl_surface` and then the layer surface, which outlives it for a moment.
    pub fn destroy_surface_first(self, client: &TestClient) {
        self.surface.destroy();
        let _ = client.connection().flush();
        self.layer.destroy();
        let _ = client.connection().flush();
    }

    /// Destroys the layer surface and its `wl_surface`.
    pub fn destroy(self, client: &TestClient) {
        self.layer.destroy();
        self.surface.destroy();
        let _ = client.connection().flush();
    }
}

impl Dispatch<WlCompositor, LayerPlain> for ClientState {
    fn event(
        _: &mut Self,
        _: &WlCompositor,
        _: <WlCompositor as wayland_client::Proxy>::Event,
        _: &LayerPlain,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ZwlrLayerShellV1, LayerPlain> for ClientState {
    fn event(
        _: &mut Self,
        _: &ZwlrLayerShellV1,
        _: zwlr_layer_shell_v1::Event,
        _: &LayerPlain,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<WlSurface, LayerPlain> for ClientState {
    fn event(
        _: &mut Self,
        _: &WlSurface,
        _: <WlSurface as wayland_client::Proxy>::Event,
        _: &LayerPlain,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ZwlrLayerSurfaceV1, LayerLog> for ClientState {
    fn event(
        _: &mut Self,
        _: &ZwlrLayerSurfaceV1,
        event: zwlr_layer_surface_v1::Event,
        log: &LayerLog,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            zwlr_layer_surface_v1::Event::Configure {
                serial,
                width,
                height,
            } => log.with(|state| {
                state.configure = Some(LayerConfigure {
                    serial,
                    width,
                    height,
                })
            }),
            zwlr_layer_surface_v1::Event::Closed => log.with(|state| state.closed = true),
            _ => {}
        }
    }
}
