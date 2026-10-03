//! `zwp_idle_inhibit_manager_v1` as a client (P0-H8): a surface of its own holds idleness off
//! until the inhibitor is destroyed. The surface has no role and no buffer, so it is alive but
//! not shown; casement counts it all the same until the shown-Space lookup exists (the limit
//! `IdleInhibitPart::inhibited` documents).
//!
//! Self-contained like `misc::idle`, whose side queue it reuses.

use super::idle::{pump_queue, side_queue};
use crate::{ClientError, GlobalList, Pumped};
use wayland_client::protocol::wl_compositor::{self, WlCompositor};
use wayland_client::protocol::wl_surface::{self, WlSurface};
use wayland_client::{Connection, Dispatch, EventQueue, QueueHandle};
use wayland_protocols::wp::idle_inhibit::zv1::client::{
    zwp_idle_inhibit_manager_v1::{self, ZwpIdleInhibitManagerV1},
    zwp_idle_inhibitor_v1::{self, ZwpIdleInhibitorV1},
};

/// The helper's dispatch target; the objects it holds send no events that matter.
#[derive(Debug)]
struct Quiet;

/// One inhibitor on a surface of its own.
#[derive(Debug)]
pub struct InhibitHold {
    connection: Connection,
    queue: EventQueue<Quiet>,
    surface: WlSurface,
    inhibitor: ZwpIdleInhibitorV1,
}

impl InhibitHold {
    /// Creates a surface and an inhibitor on it. The requests are flushed.
    pub fn start(globals: &GlobalList) -> Result<InhibitHold, ClientError> {
        let (connection, queue) = side_queue::<Quiet>(globals)?;
        let qh = queue.handle();
        let missing = |name: &str| ClientError::MissingGlobal(name.to_owned());
        let manager = globals
            .bind::<ZwpIdleInhibitManagerV1, _, _>(&qh, 1..=1, ())
            .map_err(|_| missing("zwp_idle_inhibit_manager_v1"))?;
        let compositor = globals
            .bind::<WlCompositor, _, _>(&qh, 1..=6, ())
            .map_err(|_| missing("wl_compositor"))?;
        let surface = compositor.create_surface(&qh, ());
        let inhibitor = manager.create_inhibitor(&surface, &qh, ());
        connection
            .flush()
            .map_err(|error| ClientError::Dispatch(error.to_string()))?;
        Ok(InhibitHold {
            connection,
            queue,
            surface,
            inhibitor,
        })
    }

    /// Reads what arrived (nothing the helper keeps).
    pub fn pump(&mut self) -> Result<Pumped, ClientError> {
        pump_queue(&self.connection, &mut self.queue, &mut Quiet)
    }

    /// Destroys the inhibitor (and its surface).
    pub fn release(self) -> Result<(), ClientError> {
        self.inhibitor.destroy();
        self.surface.destroy();
        self.connection
            .flush()
            .map_err(|error| ClientError::Dispatch(error.to_string()))
    }
}

impl Dispatch<ZwpIdleInhibitManagerV1, ()> for Quiet {
    fn event(
        _state: &mut Quiet,
        _proxy: &ZwpIdleInhibitManagerV1,
        _event: zwp_idle_inhibit_manager_v1::Event,
        _data: &(),
        _connection: &Connection,
        _qh: &QueueHandle<Quiet>,
    ) {
    }
}

impl Dispatch<ZwpIdleInhibitorV1, ()> for Quiet {
    fn event(
        _state: &mut Quiet,
        _proxy: &ZwpIdleInhibitorV1,
        _event: zwp_idle_inhibitor_v1::Event,
        _data: &(),
        _connection: &Connection,
        _qh: &QueueHandle<Quiet>,
    ) {
    }
}

impl Dispatch<WlCompositor, ()> for Quiet {
    fn event(
        _state: &mut Quiet,
        _proxy: &WlCompositor,
        _event: wl_compositor::Event,
        _data: &(),
        _connection: &Connection,
        _qh: &QueueHandle<Quiet>,
    ) {
    }
}

impl Dispatch<WlSurface, ()> for Quiet {
    fn event(
        _state: &mut Quiet,
        _proxy: &WlSurface,
        _event: wl_surface::Event,
        _data: &(),
        _connection: &Connection,
        _qh: &QueueHandle<Quiet>,
    ) {
    }
}
