//! `xdg_toplevel_icon_manager_v1`: a toplevel's own icon (the dock and switcher prefer the
//! app's installed icon; this is the fallback).
//!
//! The protocol is served here, not by smithay's module. smithay's `xdg_toplevel_icon_v1`
//! handler posts `immutable` and then goes on to change the icon, which trips its own debug
//! assertion (a client's mistake would abort a debug build, and the contract is that client
//! misbehaviour is a protocol error and nothing else), and it announces the preferred sizes
//! from a `HashSet`. Here an icon is a small state machine ([`icon`]): while it is being built
//! its requests change it, once assigned they are `immutable` errors and change nothing.
//!
//! The assigned icon is double-buffered in the toplevel's surface state ([`ToplevelIcon`]),
//! where the Dock and the switcher read it from (P3); the model holds no icon, so no request
//! reaches it.

mod icon;

pub use icon::{IconBuffer, IconData, IconParts};

use crate::states::GlobalFilter;
use icon::Registry;
use smithay::reexports::wayland_protocols::xdg::toplevel_icon::v1::server::xdg_toplevel_icon_manager_v1::{
    self, XdgToplevelIconManagerV1,
};
use smithay::reexports::wayland_protocols::xdg::toplevel_icon::v1::server::xdg_toplevel_icon_v1::XdgToplevelIconV1;
use smithay::reexports::wayland_server::backend::GlobalId;
use smithay::reexports::wayland_server::protocol::wl_buffer::WlBuffer;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::reexports::wayland_server::{
    Client, DataInit, Dispatch, DisplayHandle, GlobalDispatch, New, Resource,
};
use smithay::wayland::compositor::{Cacheable, with_states};
use smithay::wayland::shell::xdg::XdgShellHandler;
use smithay::wayland::{Dispatch2, GlobalDispatch2};

/// The version casement serves.
pub const VERSION: u32 = 1;

/// The icon edges (in surface-local pixels) the compositor prefers, announced at every bind
/// in this order. None at P0: the Dock sets them when it lands.
pub const PREFERRED_SIZES: [i32; 0] = [];

/// `xdg_toplevel_icon_manager_v1`.
#[derive(Debug)]
pub struct XdgToplevelIconPart {
    /// The global.
    pub global: GlobalId,
    /// The icons that hold buffers, for the `no_buffer` check.
    registry: Registry,
}

impl XdgToplevelIconPart {
    /// Creates the global (version 1). Every connection sees it.
    pub fn new<D>(display: &DisplayHandle, role_filter: GlobalFilter) -> XdgToplevelIconPart
    where
        D: GlobalDispatch<XdgToplevelIconManagerV1, IconGlobal> + 'static,
    {
        let _ = role_filter;
        let registry = Registry::default();
        XdgToplevelIconPart {
            global: display.create_global::<D, XdgToplevelIconManagerV1, _>(
                VERSION,
                IconGlobal {
                    registry: registry.clone(),
                },
            ),
            registry,
        }
    }
}

/// A `wl_buffer` was destroyed: every live icon that holds it is a `no_buffer` error (the
/// buffer must outlive the icon it was added to).
pub fn on_buffer_destroyed(part: &XdgToplevelIconPart, buffer: &WlBuffer) {
    part.registry.buffer_destroyed(buffer);
}

/// The icon `surface`'s toplevel has applied (committed), if it has one.
pub fn icon_of(surface: &WlSurface) -> Option<IconParts> {
    with_states(surface, |states| {
        states
            .cached_state
            .get::<ToplevelIcon>()
            .current()
            .parts
            .clone()
    })
}

/// The icon a toplevel has: assigned by `set_icon`, applied at the toplevel's next commit.
#[derive(Debug, Clone, Default)]
pub struct ToplevelIcon {
    parts: Option<IconParts>,
}

impl Cacheable for ToplevelIcon {
    fn commit(&mut self, _display: &DisplayHandle) -> Self {
        self.clone()
    }

    fn merge_into(self, into: &mut Self, _display: &DisplayHandle) {
        *into = self;
    }
}

/// The user data of the manager global: a bind sends the preferred sizes and `done`.
#[derive(Debug, Clone)]
pub struct IconGlobal {
    registry: Registry,
}

/// The user data of a bound manager.
#[derive(Debug, Clone)]
pub struct IconManager {
    registry: Registry,
}

impl<D> GlobalDispatch2<XdgToplevelIconManagerV1, D> for IconGlobal
where
    D: Dispatch<XdgToplevelIconManagerV1, IconManager> + 'static,
{
    fn bind(
        &self,
        _state: &mut D,
        _display: &DisplayHandle,
        _client: &Client,
        resource: New<XdgToplevelIconManagerV1>,
        data_init: &mut DataInit<'_, D>,
    ) {
        let manager = data_init.init(
            resource,
            IconManager {
                registry: self.registry.clone(),
            },
        );
        for size in PREFERRED_SIZES {
            manager.icon_size(size);
        }
        manager.done();
    }
}

impl<D> Dispatch2<XdgToplevelIconManagerV1, D> for IconManager
where
    D: Dispatch<XdgToplevelIconV1, IconData> + XdgShellHandler + 'static,
{
    fn request(
        &self,
        state: &mut D,
        _client: &Client,
        _manager: &XdgToplevelIconManagerV1,
        request: xdg_toplevel_icon_manager_v1::Request,
        _display: &DisplayHandle,
        data_init: &mut DataInit<'_, D>,
    ) {
        use xdg_toplevel_icon_manager_v1::Request;
        match request {
            Request::CreateIcon { id } => {
                data_init.init(id, IconData::new(self.registry.clone()));
            }
            Request::SetIcon { toplevel, icon } => {
                let Some(surface) = state.xdg_shell_state().get_toplevel(&toplevel) else {
                    return;
                };
                let assigned = icon.and_then(|icon| icon.data::<IconData>()?.assign());
                with_states(surface.wl_surface(), |states| {
                    states.cached_state.get::<ToplevelIcon>().pending().parts = assigned;
                });
            }
            Request::Destroy => {}
            _ => {}
        }
    }
}
