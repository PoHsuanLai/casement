//! `zxdg_decoration_manager_v1` (P0-H2): ask for a mode on a toplevel and read what the
//! compositor decided.

use crate::conn::TestClient;
use crate::surface::toplevel::Shared;
use crate::surface::toplevel::{Host, Toplevel};
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle, WEnum};
use wayland_protocols::xdg::decoration::zv1::client::zxdg_decoration_manager_v1::ZxdgDecorationManagerV1;
use wayland_protocols::xdg::decoration::zv1::client::zxdg_toplevel_decoration_v1::{
    self, ZxdgToplevelDecorationV1,
};

pub use wayland_protocols::xdg::decoration::zv1::client::zxdg_toplevel_decoration_v1::{
    Error as DecorationError, Mode,
};

/// The modes a decoration object was told, in order.
pub type ModeLog = Shared<Vec<Mode>>;

/// A decoration object on a toplevel.
#[derive(Debug)]
pub struct Decoration {
    /// The object.
    pub decoration: ZxdgToplevelDecorationV1,
    /// The `configure(mode)` events it received.
    pub modes: ModeLog,
}

impl Decoration {
    /// The newest mode the compositor chose.
    pub fn newest(&self) -> Option<Mode> {
        self.modes.with(|seen| seen.last().copied())
    }

    /// Asks for `mode`.
    pub fn set_mode(&self, mode: Mode) {
        self.decoration.set_mode(mode);
    }
}

/// A bound `zxdg_decoration_manager_v1`.
#[derive(Debug)]
pub struct DecorationManager {
    /// The global.
    pub manager: ZxdgDecorationManagerV1,
}

impl DecorationManager {
    /// Binds the global (version 1); `None` when this connection is not shown it (I7).
    pub fn bind(client: &mut TestClient) -> Option<DecorationManager> {
        client
            .bind(1..=1, ())
            .ok()
            .map(|manager| DecorationManager { manager })
    }

    /// A decoration object for `toplevel`.
    pub fn decorate(&self, client: &mut TestClient, toplevel: &Toplevel) -> Decoration {
        let modes = ModeLog::default();
        let decoration =
            self.manager
                .get_toplevel_decoration(&toplevel.toplevel, &client.qh(), modes.clone());
        Decoration { decoration, modes }
    }
}

impl Dispatch<ZxdgDecorationManagerV1, ()> for Host {
    fn event(
        _: &mut Self,
        _: &ZxdgDecorationManagerV1,
        _: <ZxdgDecorationManagerV1 as Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ZxdgToplevelDecorationV1, ModeLog> for Host {
    fn event(
        _: &mut Self,
        _: &ZxdgToplevelDecorationV1,
        event: zxdg_toplevel_decoration_v1::Event,
        modes: &ModeLog,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let zxdg_toplevel_decoration_v1::Event::Configure {
            mode: WEnum::Value(mode),
        } = event
        {
            modes.with(|seen| seen.push(mode));
        }
    }
}
