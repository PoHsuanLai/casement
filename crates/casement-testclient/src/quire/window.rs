//! `quire_window_v1` (C50, P0-H2): hints in, emphasis and placement out.

use crate::conn::TestClient;
use crate::surface::toplevel::Shared;
use crate::surface::toplevel::{Host, Toplevel};
use casement_protocols::client::quire_window_v1::quire_window_manager_v1::QuireWindowManagerV1;
use casement_protocols::client::quire_window_v1::quire_window_v1::{self, QuireWindowV1};
use wayland_client::protocol::wl_region::WlRegion;
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle, WEnum};

pub use casement_protocols::client::quire_window_v1::quire_window_manager_v1::Error as ManagerError;
pub use casement_protocols::client::quire_window_v1::quire_window_v1::{
    Edited, Emphasis, Error as WindowError, FrameMode, PlaceTarget, PlacementMark, WindowRole,
};

/// What a window extension was told.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct WindowEvents {
    /// Every `emphasis`, in order.
    pub emphasis: Vec<Emphasis>,
    /// Every `placement`, in order.
    pub placement: Vec<PlacementMark>,
}

/// A window extension's log.
pub type WindowLog = Shared<WindowEvents>;

/// A bound `quire_window_manager_v1`.
#[derive(Debug)]
pub struct WindowManager {
    /// The global.
    pub manager: QuireWindowManagerV1,
}

impl WindowManager {
    /// Binds the global at `version` (1 is the only one).
    pub fn bind(client: &mut TestClient, version: u32) -> WindowManager {
        WindowManager {
            manager: client
                .bind(version..=version, ())
                .expect("the compositor advertises quire_window_manager_v1"),
        }
    }

    /// Extends `toplevel`.
    pub fn window(&self, client: &mut TestClient, toplevel: &Toplevel) -> Window {
        let log = WindowLog::default();
        let object = self
            .manager
            .get_window(&toplevel.toplevel, &client.qh(), log.clone());
        Window { object, log }
    }
}

/// A toplevel's extension object.
#[derive(Debug)]
pub struct Window {
    /// The object.
    pub object: QuireWindowV1,
    /// What it was told.
    pub log: WindowLog,
}

impl Window {
    /// The newest emphasis.
    pub fn emphasis(&self) -> Option<Emphasis> {
        self.log.with(|seen| seen.emphasis.last().copied())
    }

    /// The newest placement mark.
    pub fn placement(&self) -> Option<PlacementMark> {
        self.log.with(|seen| seen.placement.last().copied())
    }

    /// Sets the first-mouse region.
    pub fn set_first_mouse(&self, region: Option<&WlRegion>) {
        self.object.set_first_mouse(region);
    }

    /// Sets the frame.
    pub fn set_frame(&self, mode: FrameMode, corner_radius: u32) {
        self.object.set_frame(mode, corner_radius);
    }

    /// Asks for a placement, answering `serial`.
    pub fn request_placement(&self, target: PlaceTarget, serial: u32) {
        self.object.request_placement(target, serial);
    }

    /// Declares the role.
    pub fn set_role(&self, role: WindowRole) {
        self.object.set_role(role);
    }

    /// Says whether the document has unsaved changes.
    pub fn set_edited(&self, edited: Edited) {
        self.object.set_edited(edited);
    }

    /// Names the accessibility root.
    pub fn set_a11y_root(&self, bus_name: &str, object_path: &str) {
        self.object
            .set_a11y_root(bus_name.to_owned(), object_path.to_owned());
    }
}

impl Dispatch<QuireWindowManagerV1, ()> for Host {
    fn event(
        _: &mut Self,
        _: &QuireWindowManagerV1,
        _: <QuireWindowManagerV1 as Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<QuireWindowV1, WindowLog> for Host {
    fn event(
        _: &mut Self,
        _: &QuireWindowV1,
        event: quire_window_v1::Event,
        log: &WindowLog,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            quire_window_v1::Event::Emphasis {
                emphasis: WEnum::Value(emphasis),
            } => {
                log.with(|seen| seen.emphasis.push(emphasis));
            }
            quire_window_v1::Event::Placement {
                mark: WEnum::Value(mark),
            } => {
                log.with(|seen| seen.placement.push(mark));
            }
            _ => {}
        }
    }
}
