//! `ext_background_effect_manager_v1` as a client (P0-H8): a surface of its own asks for the
//! blur behind a region, commits, and reads the capabilities the manager announced.
//!
//! Self-contained like `misc::idle`, whose side queue it reuses.

use super::idle::{pump_queue, side_queue};
use crate::{ClientError, GlobalList, Pumped};
use casement_core::geometry::LogicalRect;
use wayland_client::protocol::wl_compositor::{self, WlCompositor};
use wayland_client::protocol::wl_region::{self, WlRegion};
use wayland_client::protocol::wl_surface::{self, WlSurface};
use wayland_client::{Connection, Dispatch, EventQueue, QueueHandle, WEnum};
use wayland_protocols::ext::background_effect::v1::client::{
    ext_background_effect_manager_v1::{self, Capability, ExtBackgroundEffectManagerV1},
    ext_background_effect_surface_v1::{self, ExtBackgroundEffectSurfaceV1},
};

/// What the manager announced.
#[derive(Debug, Default)]
struct Announced {
    capabilities: Option<WEnum<Capability>>,
}

/// One surface with a background-effect object.
#[derive(Debug)]
pub struct BlurAsk {
    connection: Connection,
    queue: EventQueue<Announced>,
    announced: Announced,
    compositor: WlCompositor,
    surface: WlSurface,
    effect: ExtBackgroundEffectSurfaceV1,
}

impl BlurAsk {
    /// Binds the manager, creates a surface and its background-effect object. The requests are
    /// flushed.
    pub fn start(globals: &GlobalList) -> Result<BlurAsk, ClientError> {
        let (connection, queue) = side_queue::<Announced>(globals)?;
        let qh = queue.handle();
        let missing = |name: &str| ClientError::MissingGlobal(name.to_owned());
        let manager = globals
            .bind::<ExtBackgroundEffectManagerV1, _, _>(&qh, 1..=1, ())
            .map_err(|_| missing("ext_background_effect_manager_v1"))?;
        let compositor = globals
            .bind::<WlCompositor, _, _>(&qh, 1..=6, ())
            .map_err(|_| missing("wl_compositor"))?;
        let surface = compositor.create_surface(&qh, ());
        let effect = manager.get_background_effect(&surface, &qh, ());
        connection
            .flush()
            .map_err(|error| ClientError::Dispatch(error.to_string()))?;
        Ok(BlurAsk {
            connection,
            queue,
            announced: Announced::default(),
            compositor,
            surface,
            effect,
        })
    }

    /// The surface the effect belongs to (to give it a role, an xdg toplevel say, so the blur
    /// is behind a window).
    pub fn surface(&self) -> &WlSurface {
        &self.surface
    }

    /// Reads what arrived.
    pub fn pump(&mut self) -> Result<Pumped, ClientError> {
        pump_queue(&self.connection, &mut self.queue, &mut self.announced)
    }

    /// Whether the manager announced that it blurs.
    pub fn blur_announced(&self) -> bool {
        matches!(
            self.announced.capabilities,
            Some(WEnum::Value(capabilities)) if capabilities.contains(Capability::Blur)
        )
    }

    /// Asks for blur behind `rects` (surface-local logical rectangles); takes effect at commit.
    pub fn set_blur(&mut self, rects: &[LogicalRect]) -> Result<(), ClientError> {
        let qh = self.queue.handle();
        let region = self.compositor.create_region(&qh, ());
        for rect in rects {
            region.add(
                rect.origin.x,
                rect.origin.y,
                i32::try_from(rect.size.w).unwrap_or(i32::MAX),
                i32::try_from(rect.size.h).unwrap_or(i32::MAX),
            );
        }
        self.effect.set_blur_region(Some(&region));
        region.destroy();
        self.flush()
    }

    /// Asks for no blur (`set_blur_region(null)`); takes effect at commit.
    pub fn unset_blur(&mut self) -> Result<(), ClientError> {
        self.effect.set_blur_region(None);
        self.flush()
    }

    /// Commits the surface.
    pub fn commit(&mut self) -> Result<(), ClientError> {
        self.surface.commit();
        self.flush()
    }

    /// Destroys the background-effect object (the effect goes with it).
    pub fn destroy_effect(&mut self) -> Result<(), ClientError> {
        self.effect.destroy();
        self.flush()
    }

    fn flush(&self) -> Result<(), ClientError> {
        self.connection
            .flush()
            .map_err(|error| ClientError::Dispatch(error.to_string()))
    }
}

impl Dispatch<ExtBackgroundEffectManagerV1, ()> for Announced {
    fn event(
        state: &mut Announced,
        _proxy: &ExtBackgroundEffectManagerV1,
        event: ext_background_effect_manager_v1::Event,
        _data: &(),
        _connection: &Connection,
        _qh: &QueueHandle<Announced>,
    ) {
        let ext_background_effect_manager_v1::Event::Capabilities { flags } = event else {
            return;
        };
        state.capabilities = Some(flags);
    }
}

impl Dispatch<ExtBackgroundEffectSurfaceV1, ()> for Announced {
    fn event(
        _state: &mut Announced,
        _proxy: &ExtBackgroundEffectSurfaceV1,
        _event: ext_background_effect_surface_v1::Event,
        _data: &(),
        _connection: &Connection,
        _qh: &QueueHandle<Announced>,
    ) {
    }
}

impl Dispatch<WlCompositor, ()> for Announced {
    fn event(
        _state: &mut Announced,
        _proxy: &WlCompositor,
        _event: wl_compositor::Event,
        _data: &(),
        _connection: &Connection,
        _qh: &QueueHandle<Announced>,
    ) {
    }
}

impl Dispatch<WlSurface, ()> for Announced {
    fn event(
        _state: &mut Announced,
        _proxy: &WlSurface,
        _event: wl_surface::Event,
        _data: &(),
        _connection: &Connection,
        _qh: &QueueHandle<Announced>,
    ) {
    }
}

impl Dispatch<WlRegion, ()> for Announced {
    fn event(
        _state: &mut Announced,
        _proxy: &WlRegion,
        _event: wl_region::Event,
        _data: &(),
        _connection: &Connection,
        _qh: &QueueHandle<Announced>,
    ) {
    }
}
