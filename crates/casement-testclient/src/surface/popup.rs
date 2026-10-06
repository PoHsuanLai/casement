//! xdg popups (P0-H2): positioners, popups on a toplevel, grabs and repositioning.

use crate::conn::TestClient;
use crate::surface::toplevel::Shared;
use crate::surface::toplevel::{Host, Toplevel, WmBase};
use wayland_client::protocol::wl_seat::WlSeat;
use wayland_client::protocol::wl_surface::WlSurface;
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle};
use wayland_protocols::xdg::shell::client::xdg_popup::{self, XdgPopup};
use wayland_protocols::xdg::shell::client::xdg_positioner::{
    Anchor, ConstraintAdjustment, Gravity, XdgPositioner,
};
use wayland_protocols::xdg::shell::client::xdg_surface::{self, XdgSurface};

/// Where a popup wants to be, in its parent's coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PopupSpec {
    /// The popup's size.
    pub size: (i32, i32),
    /// The anchor rectangle: x, y, width, height.
    pub anchor_rect: (i32, i32, i32, i32),
    /// The anchor edges.
    pub anchor: Anchor,
    /// The gravity.
    pub gravity: Gravity,
    /// How it may be adjusted to fit.
    pub adjustment: ConstraintAdjustment,
}

impl PopupSpec {
    /// A menu 100 x 50 hanging down and right from the top-left of a 10 x 10 anchor at `at`,
    /// allowed to slide.
    pub fn menu_at(at: (i32, i32)) -> PopupSpec {
        PopupSpec {
            size: (100, 50),
            anchor_rect: (at.0, at.1, 10, 10),
            anchor: Anchor::TopLeft,
            gravity: Gravity::BottomRight,
            adjustment: ConstraintAdjustment::SlideX | ConstraintAdjustment::SlideY,
        }
    }
}

/// What a popup was told.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct PopupEvents {
    /// Every `xdg_popup.configure`: x, y, width, height.
    pub configures: Vec<(i32, i32, i32, i32)>,
    /// The serials of every `xdg_surface.configure`, in order.
    pub serials: Vec<u32>,
    /// The tokens of every `repositioned`.
    pub repositioned: Vec<u32>,
    /// How many `popup_done` events arrived.
    pub done: u32,
}

/// A popup's log.
pub type PopupLog = Shared<PopupEvents>;

/// A popup and its objects.
#[derive(Debug)]
pub struct Popup {
    /// The `wl_surface`.
    pub surface: WlSurface,
    /// The `xdg_surface`.
    pub xdg_surface: XdgSurface,
    /// The `xdg_popup`.
    pub popup: XdgPopup,
    /// The positioner it was made with.
    pub positioner: XdgPositioner,
    /// What it was told.
    pub log: PopupLog,
}

/// A new positioner from `spec`.
pub fn positioner(client: &mut TestClient, base: &WmBase, spec: PopupSpec) -> XdgPositioner {
    let positioner = base.base.create_positioner(&client.qh(), ());
    positioner.set_size(spec.size.0, spec.size.1);
    let (x, y, w, h) = spec.anchor_rect;
    positioner.set_anchor_rect(x, y, w, h);
    positioner.set_anchor(spec.anchor);
    positioner.set_gravity(spec.gravity);
    positioner.set_constraint_adjustment(spec.adjustment);
    positioner
}

impl Popup {
    /// A popup of `parent` placed by `spec`. Nothing is committed.
    pub fn of(
        client: &mut TestClient,
        base: &WmBase,
        parent: &Toplevel,
        surface: WlSurface,
        spec: PopupSpec,
    ) -> Popup {
        let positioner = positioner(client, base, spec);
        Popup::placed_by(client, base, parent, surface, positioner)
    }

    /// As [`Popup::of`], placed by `spec` and asking to follow its parent when that moves
    /// (`xdg_positioner.set_reactive`).
    pub fn reactive_of(
        client: &mut TestClient,
        base: &WmBase,
        parent: &Toplevel,
        surface: WlSurface,
        spec: PopupSpec,
    ) -> Popup {
        let positioner = positioner(client, base, spec);
        positioner.set_reactive();
        Popup::placed_by(client, base, parent, surface, positioner)
    }

    fn placed_by(
        client: &mut TestClient,
        base: &WmBase,
        parent: &Toplevel,
        surface: WlSurface,
        positioner: XdgPositioner,
    ) -> Popup {
        let log = PopupLog::default();
        let xdg_surface = base
            .base
            .get_xdg_surface(&surface, &client.qh(), log.clone());
        let popup = xdg_surface.get_popup(
            Some(&parent.xdg_surface),
            &positioner,
            &client.qh(),
            log.clone(),
        );
        Popup {
            surface,
            xdg_surface,
            popup,
            positioner,
            log,
        }
    }

    /// A popup of `parent` (a submenu), placed by `spec` in the parent popup's coordinates.
    /// Nothing is committed.
    pub fn of_popup(
        client: &mut TestClient,
        base: &WmBase,
        parent: &Popup,
        surface: WlSurface,
        spec: PopupSpec,
    ) -> Popup {
        let log = PopupLog::default();
        let positioner = positioner(client, base, spec);
        let xdg_surface = base
            .base
            .get_xdg_surface(&surface, &client.qh(), log.clone());
        let popup = xdg_surface.get_popup(
            Some(&parent.xdg_surface),
            &positioner,
            &client.qh(),
            log.clone(),
        );
        Popup {
            surface,
            xdg_surface,
            popup,
            positioner,
            log,
        }
    }

    /// A popup of a layer surface, placed by `spec` in the layer surface's coordinates: it has
    /// no xdg parent, and the layer surface adopts it ([`crate::surface::layer::LayerSurface::adopt`]).
    /// Nothing is committed.
    pub fn of_layer(
        client: &mut TestClient,
        base: &WmBase,
        surface: WlSurface,
        spec: PopupSpec,
    ) -> Popup {
        let log = PopupLog::default();
        let positioner = positioner(client, base, spec);
        let xdg_surface = base
            .base
            .get_xdg_surface(&surface, &client.qh(), log.clone());
        let popup = xdg_surface.get_popup(None, &positioner, &client.qh(), log.clone());
        Popup {
            surface,
            xdg_surface,
            popup,
            positioner,
            log,
        }
    }

    /// Acks the newest `xdg_surface.configure`; says which serial, if any.
    pub fn ack_latest(&self) -> Option<u32> {
        let serial = self.log.with(|seen| seen.serials.last().copied())?;
        self.xdg_surface.ack_configure(serial);
        Some(serial)
    }

    /// Attaches `buffer` over the whole popup and commits (the popup maps).
    pub fn show(&self, buffer: &crate::shm::ShmBuffer) {
        let size = buffer.size();
        self.surface.attach(Some(buffer.buffer()), 0, 0);
        self.surface.damage_buffer(
            0,
            0,
            i32::try_from(size.w).unwrap_or(i32::MAX),
            i32::try_from(size.h).unwrap_or(i32::MAX),
        );
        self.surface.commit();
    }

    /// Asks for a frame callback and commits: the returned handle says when it came back.
    pub fn request_frame(&self, client: &TestClient) -> crate::SyncDone {
        let done = crate::SyncDone::default();
        self.surface.frame(&client.qh(), done.clone());
        self.surface.commit();
        done
    }

    /// Takes an explicit grab answering `serial`.
    pub fn grab(&self, seat: &WlSeat, serial: u32) {
        self.popup.grab(seat, serial);
    }

    /// Asks for `spec`'s placement, tagged `token`.
    pub fn reposition(&self, client: &mut TestClient, base: &WmBase, spec: PopupSpec, token: u32) {
        let next = positioner(client, base, spec);
        self.popup.reposition(&next, token);
    }

    /// Destroys the popup objects, newest first.
    pub fn destroy(&self) {
        self.popup.destroy();
        self.xdg_surface.destroy();
        self.surface.destroy();
    }

    /// The `xdg_popup.configure`s so far.
    pub fn geometry(&self) -> Option<(i32, i32, i32, i32)> {
        self.log.with(|seen| seen.configures.last().copied())
    }
}

impl Dispatch<XdgPositioner, ()> for Host {
    fn event(
        _: &mut Self,
        _: &XdgPositioner,
        _: <XdgPositioner as Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<XdgSurface, PopupLog> for Host {
    fn event(
        _: &mut Self,
        _: &XdgSurface,
        event: xdg_surface::Event,
        log: &PopupLog,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let xdg_surface::Event::Configure { serial } = event {
            log.with(|seen| seen.serials.push(serial));
        }
    }
}

impl Dispatch<XdgPopup, PopupLog> for Host {
    fn event(
        _: &mut Self,
        _: &XdgPopup,
        event: xdg_popup::Event,
        log: &PopupLog,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            xdg_popup::Event::Configure {
                x,
                y,
                width,
                height,
            } => {
                log.with(|seen| seen.configures.push((x, y, width, height)));
            }
            xdg_popup::Event::PopupDone => {
                log.with(|seen| seen.done += 1);
            }
            xdg_popup::Event::Repositioned { token } => {
                log.with(|seen| seen.repositioned.push(token));
            }
            _ => {}
        }
    }
}
