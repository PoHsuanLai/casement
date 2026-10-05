//! xdg toplevels (P0-H2): `xdg_wm_base` (it answers pings), `xdg_surface` and `xdg_toplevel`
//! with a log of every configure, plus the press serials a pointer delivers (what a move,
//! resize or popup grab must answer).
//!
//! The helper follows the connection core's pattern: `Dispatch<object, log>` for the client's
//! [`Host`] type and one [`Shared`] log per object that the test reads. The transcript is
//! recorded by the core's wire tap, so nothing here logs.

pub mod extras;
mod press;

use crate::conn::{ClientState, TestClient};
use crate::shm::ShmBuffer;
use std::sync::{Arc, Mutex};
use wayland_client::protocol::wl_surface::WlSurface;
use wayland_client::{Connection, Dispatch, QueueHandle};
use wayland_protocols::xdg::shell::client::xdg_surface::{self, XdgSurface};
use wayland_protocols::xdg::shell::client::xdg_toplevel::{self, XdgToplevel};
use wayland_protocols::xdg::shell::client::xdg_wm_base::{self, XdgWmBase};

pub use press::{BoundPointer, PressEvents, PressLog, PressSeen, bind_pointer};
pub use wayland_protocols::xdg::shell::client as shell;
pub use wayland_protocols::xdg::shell::client::xdg_toplevel::{ResizeEdge, State};

/// The type every helper's `Dispatch` impl is for.
pub type Host = ClientState;

/// A log a helper's `Dispatch` impl writes and its test reads (user data must be shareable).
#[derive(Debug, Default)]
pub struct Shared<T>(Arc<Mutex<T>>);

impl<T> Clone for Shared<T> {
    fn clone(&self) -> Self {
        Shared(Arc::clone(&self.0))
    }
}

impl<T> Shared<T> {
    /// Runs `f` on the log.
    pub fn with<R>(&self, f: impl FnOnce(&mut T) -> R) -> R {
        f(&mut self.0.lock().unwrap_or_else(|poison| poison.into_inner()))
    }
}

/// One configure pair a toplevel received: `xdg_toplevel.configure` and the
/// `xdg_surface.configure` that closed it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Configure {
    /// The serial to ack.
    pub serial: u32,
    /// The suggested size (0 on a side: the client chooses).
    pub size: (i32, i32),
    /// The states, in wire order.
    pub states: Vec<State>,
}

/// Everything a toplevel and its `xdg_surface` were told.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ToplevelEvents {
    /// Every completed configure, oldest first.
    pub configures: Vec<Configure>,
    /// `configure_bounds`, as last sent.
    pub bounds: Option<(i32, i32)>,
    /// `wm_capabilities`, as last sent (wire codes).
    pub capabilities: Vec<u32>,
    /// How many `close` events arrived.
    pub closes: u32,
    pending: Option<((i32, i32), Vec<State>)>,
}

/// A toplevel's log.
pub type ToplevelLog = Shared<ToplevelEvents>;

/// How many pings an `xdg_wm_base` was sent (it answers each at once).
pub type Pings = Shared<Vec<u32>>;

/// A bound `xdg_wm_base`.
#[derive(Debug)]
pub struct WmBase {
    /// The global.
    pub base: XdgWmBase,
    /// The ping serials it received (each answered with a pong).
    pub pings: Pings,
}

impl WmBase {
    /// Binds `xdg_wm_base` at `version`.
    pub fn bind(client: &mut TestClient, version: u32) -> WmBase {
        let pings = Pings::default();
        let base = client
            .bind(version..=version, pings.clone())
            .expect("the compositor advertises xdg_wm_base");
        WmBase { base, pings }
    }

    /// A new surface with the toplevel role (through a new `xdg_surface`). Nothing is committed.
    pub fn toplevel(&self, client: &mut TestClient) -> Toplevel {
        let surface = client
            .create_surface()
            .expect("the compositor advertises wl_compositor");
        self.toplevel_on(client, surface)
    }

    /// Gives `surface` the toplevel role (through a new `xdg_surface`). Nothing is committed.
    pub fn toplevel_on(&self, client: &mut TestClient, surface: WlSurface) -> Toplevel {
        let log = ToplevelLog::default();
        let xdg_surface = self
            .base
            .get_xdg_surface(&surface, &client.qh(), log.clone());
        let toplevel = xdg_surface.get_toplevel(&client.qh(), log.clone());
        Toplevel {
            surface,
            xdg_surface,
            toplevel,
            log,
        }
    }
}

/// A toplevel: its surface, its `xdg_surface`, its `xdg_toplevel` and what they were told.
#[derive(Debug)]
pub struct Toplevel {
    /// The `wl_surface`.
    pub surface: WlSurface,
    /// The `xdg_surface`.
    pub xdg_surface: XdgSurface,
    /// The `xdg_toplevel`.
    pub toplevel: XdgToplevel,
    /// Its configures, closes and bounds.
    pub log: ToplevelLog,
}

impl Toplevel {
    /// Commits with nothing attached: the request for the initial configure.
    pub fn commit_bare(&self) {
        self.surface.commit();
    }

    /// Acks the newest configure received, if any; says which serial.
    pub fn ack_latest(&self) -> Option<u32> {
        let serial = self
            .log
            .with(|seen| seen.configures.last().map(|last| last.serial))?;
        self.xdg_surface.ack_configure(serial);
        Some(serial)
    }

    /// Attaches `buffer`, damages it and commits: with the initial configure acked this maps.
    pub fn show(&self, buffer: &ShmBuffer) {
        self.surface.attach(Some(buffer.buffer()), 0, 0);
        self.surface.damage_buffer(0, 0, i32::MAX, i32::MAX);
        self.surface.commit();
    }

    /// Sets the title.
    pub fn set_title(&self, title: &str) {
        self.toplevel.set_title(title.to_owned());
    }

    /// Sets the app id.
    pub fn set_app_id(&self, app_id: &str) {
        self.toplevel.set_app_id(app_id.to_owned());
    }

    /// Sets the size limits (0 on a side: none).
    pub fn set_limits(&self, min: (i32, i32), max: (i32, i32)) {
        self.toplevel.set_min_size(min.0, min.1);
        self.toplevel.set_max_size(max.0, max.1);
    }

    /// Sets (or clears) the parent.
    pub fn set_parent(&self, parent: Option<&Toplevel>) {
        self.toplevel
            .set_parent(parent.map(|parent| &parent.toplevel));
    }

    /// Asks the compositor to start a move, answering `serial`.
    pub fn request_move(&self, seat: &wayland_client::protocol::wl_seat::WlSeat, serial: u32) {
        self.toplevel._move(seat, serial);
    }

    /// Asks the compositor to start a resize from `edge`, answering `serial`.
    pub fn request_resize(
        &self,
        seat: &wayland_client::protocol::wl_seat::WlSeat,
        serial: u32,
        edge: ResizeEdge,
    ) {
        self.toplevel.resize(seat, serial, edge);
    }

    /// Asks to be maximised.
    pub fn set_maximized(&self) {
        self.toplevel.set_maximized();
    }

    /// Asks to be full screen (on whichever output the compositor picks).
    pub fn set_fullscreen(&self) {
        self.toplevel.set_fullscreen(None);
    }

    /// Asks to be minimised.
    pub fn set_minimized(&self) {
        self.toplevel.set_minimized();
    }

    /// Destroys the `xdg_toplevel`, then the `xdg_surface`, then the `wl_surface` (the order
    /// the protocol asks for).
    pub fn destroy(&self) {
        self.toplevel.destroy();
        self.xdg_surface.destroy();
        self.surface.destroy();
    }

    /// The completed configures so far.
    pub fn configures(&self) -> Vec<Configure> {
        self.log.with(|seen| seen.configures.clone())
    }

    /// Whether the newest configure has `state`.
    pub fn newest_has(&self, state: State) -> bool {
        self.log.with(|seen| {
            seen.configures
                .last()
                .is_some_and(|last| last.states.contains(&state))
        })
    }

    /// The `wl_surface` object.
    pub fn wl_surface(&self) -> &WlSurface {
        &self.surface
    }
}

impl Dispatch<XdgWmBase, Pings> for Host {
    fn event(
        _: &mut Self,
        base: &XdgWmBase,
        event: xdg_wm_base::Event,
        pings: &Pings,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let xdg_wm_base::Event::Ping { serial } = event {
            pings.with(|seen| seen.push(serial));
            base.pong(serial);
        }
    }
}

impl Dispatch<XdgSurface, ToplevelLog> for Host {
    fn event(
        _: &mut Self,
        _: &XdgSurface,
        event: xdg_surface::Event,
        log: &ToplevelLog,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let xdg_surface::Event::Configure { serial } = event {
            log.with(|seen| {
                let (size, states) = seen.pending.take().unwrap_or_default();
                seen.configures.push(Configure {
                    serial,
                    size,
                    states,
                });
            });
        }
    }
}

/// The states a configure carried, from the wire's packed `u32`s.
fn states_of(raw: &[u8]) -> Vec<State> {
    raw.as_chunks::<4>()
        .0
        .iter()
        .filter_map(|bytes| State::try_from(u32::from_ne_bytes(*bytes)).ok())
        .collect()
}

impl Dispatch<XdgToplevel, ToplevelLog> for Host {
    fn event(
        _: &mut Self,
        _: &XdgToplevel,
        event: xdg_toplevel::Event,
        log: &ToplevelLog,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            xdg_toplevel::Event::Configure {
                width,
                height,
                states,
            } => {
                let states = states_of(&states);
                log.with(|seen| seen.pending = Some(((width, height), states)));
            }
            xdg_toplevel::Event::Close => {
                log.with(|seen| seen.closes += 1);
            }
            xdg_toplevel::Event::ConfigureBounds { width, height } => {
                log.with(|seen| seen.bounds = Some((width, height)));
            }
            xdg_toplevel::Event::WmCapabilities { capabilities } => {
                let codes: Vec<u32> = capabilities
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .map(|bytes| u32::from_ne_bytes(*bytes))
                    .collect();
                log.with(|seen| seen.capabilities = codes);
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn states_unpack_from_the_wire() {
        let raw: Vec<u8> = [4u32, 1, 99]
            .iter()
            .flat_map(|code| code.to_ne_bytes())
            .collect();
        // 4 = activated, 1 = maximized, 99 is not a state and is dropped.
        assert_eq!(states_of(&raw), vec![State::Activated, State::Maximized]);
        assert_eq!(states_of(&[1, 2, 3]), Vec::new());
    }
}
