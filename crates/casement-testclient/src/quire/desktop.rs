//! quire_desktop_v1 as a client (P2-Q1): the apps, windows and Spaces the compositor announced,
//! and the requests a shell makes of them.
//!
//! Self-contained like `manage::workspace`: it reads its own events on a side queue of the
//! client's connection (the fixture serves the compositor on the same thread, so a blocking
//! round trip would never be answered). Objects are named by their order of announcement.

mod roster;

use self::roster::{Origin, Roster};
use crate::misc::idle::{pump_queue, side_queue};
use crate::{ClientError, GlobalList, Pumped};
use casement_protocols::client::quire_desktop_v1::quire_desktop_manager_v1::QuireDesktopManagerV1;
use std::ops::RangeInclusive;
use wayland_client::Connection;
use wayland_client::EventQueue;
use wayland_client::protocol::wl_output::WlOutput;
use wayland_client::protocol::wl_seat::WlSeat;
use wayland_protocols::ext::foreign_toplevel_list::v1::client::ext_foreign_toplevel_list_v1::ExtForeignToplevelListV1;

pub use self::roster::{ActiveView, AppView, GeometryView, Presence, SpaceView, WindowView};
pub use casement_protocols::client::quire_desktop_v1::quire_desktop_app_v1::{
    AppLifeKind, AppVisibility, Attention, ProofKind,
};
pub use casement_protocols::client::quire_desktop_v1::quire_desktop_manager_v1::{
    ActorKind, Error as ManagerError,
};
pub use casement_protocols::client::quire_desktop_v1::quire_desktop_space_v1::{
    SpaceActivity, SpaceKind,
};
pub use casement_protocols::client::quire_desktop_v1::quire_desktop_window_v1::{
    Assign, LeaseState, Modality, PlaceKind, Responsiveness, Toggle, WindowStateKind,
};
pub use casement_protocols::client::quire_window_v1::quire_window_v1::{
    Emphasis, PlaceTarget, PlacementMark, WindowRole,
};

/// A request on an app.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum AppRequest {
    /// `activate`, with the serial 0 `desktop-client` sends.
    Activate,
    /// `hide`.
    Hide,
    /// `unhide`.
    Unhide,
    /// `hide_others`.
    HideOthers,
    /// `quit`.
    Quit,
    /// `force_quit`.
    ForceQuit,
    /// `reopen`.
    Reopen,
}

/// A request on a window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum WindowRequest {
    /// `activate`, with serial 0.
    Activate,
    /// `close`.
    Close,
    /// `minimise`.
    Minimise,
    /// `unminimise`.
    Unminimise,
    /// `place`.
    Place(PlaceTarget),
    /// `full_screen`.
    FullScreen(Toggle),
    /// `to_space`, naming the Space announced at this index.
    ToSpace(usize),
    /// `assign`.
    Assign(Assign),
}

/// A request on a Space.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum SpaceRequest {
    /// `activate`.
    Activate,
    /// `remove`.
    Remove,
    /// `move_to`.
    MoveTo(u32),
}

/// A bound `quire_desktop_manager_v1`.
#[derive(Debug)]
pub struct DesktopWatch {
    connection: Connection,
    queue: EventQueue<Roster>,
    roster: Roster,
    manager: QuireDesktopManagerV1,
    output: WlOutput,
    seat: WlSeat,
    _list: Option<ExtForeignToplevelListV1>,
}

impl DesktopWatch {
    /// Binds the first `wl_output`, a `wl_seat`, the window list when offered, and the manager;
    /// the compositor announces the desktop at the fixture's next pump.
    ///
    /// The manager is bound at the newest version the compositor serves (2); a client that
    /// wants version 1 behaviour uses [`DesktopWatch::start_at`].
    pub fn start(globals: &GlobalList) -> Result<DesktopWatch, ClientError> {
        DesktopWatch::start_at(globals, 1..=2)
    }

    /// As [`DesktopWatch::start`], binding the manager at a version in `versions`.
    pub fn start_at(
        globals: &GlobalList,
        versions: RangeInclusive<u32>,
    ) -> Result<DesktopWatch, ClientError> {
        let (connection, queue) = side_queue::<Roster>(globals)?;
        let qh = queue.handle();
        let missing = |name: &str| ClientError::MissingGlobal(name.to_owned());
        let output = globals
            .bind::<WlOutput, _, _>(&qh, 1..=4, ())
            .map_err(|_| missing("wl_output"))?;
        let seat = globals
            .bind::<WlSeat, _, _>(&qh, 1..=1, ())
            .map_err(|_| missing("wl_seat"))?;
        let list = globals
            .bind::<ExtForeignToplevelListV1, _, _>(&qh, 1..=1, ())
            .ok();
        let manager = globals
            .bind::<QuireDesktopManagerV1, _, _>(&qh, versions, ())
            .map_err(|_| missing("quire_desktop_manager_v1"))?;
        connection
            .flush()
            .map_err(|error| ClientError::Dispatch(error.to_string()))?;
        Ok(DesktopWatch {
            connection,
            queue,
            roster: Roster::default(),
            manager,
            output,
            seat,
            _list: list,
        })
    }

    /// Reads what arrived.
    pub fn pump(&mut self) -> Result<Pumped, ClientError> {
        pump_queue(&self.connection, &mut self.queue, &mut self.roster)
    }

    /// Every app announced, in order, as of the events read.
    pub fn apps(&self) -> Vec<AppView> {
        self.roster
            .apps
            .iter()
            .map(|seen| seen.view.clone())
            .collect()
    }

    /// Every window the manager announced, in order, names resolved.
    pub fn windows(&self) -> Vec<WindowView> {
        self.roster.window_views(Origin::Announced)
    }

    /// Every window object made with [`get_window_for`](Self::get_window_for), in order.
    pub fn asked_windows(&self) -> Vec<WindowView> {
        self.roster.window_views(Origin::Asked)
    }

    /// Every Space announced, in order.
    pub fn spaces(&self) -> Vec<SpaceView> {
        self.roster
            .spaces
            .iter()
            .map(|seen| seen.view.clone())
            .collect()
    }

    /// The active app and key window as last announced.
    pub fn active(&self) -> ActiveView {
        self.roster.active_view()
    }

    /// How many `done` events the manager sent.
    pub fn dones(&self) -> usize {
        self.roster.dones
    }

    /// Whether the compositor sent `finished`.
    pub fn finished(&self) -> bool {
        self.roster.finished
    }

    /// The identifiers of the windows the list announced, in order.
    pub fn listed(&self) -> Vec<String> {
        self.roster
            .handles
            .iter()
            .map(|(_, identifier)| identifier.clone())
            .collect()
    }

    /// `get_window_for` the window the list announced at `index`.
    pub fn get_window_for(&mut self, index: usize) -> Result<(), ClientError> {
        let (handle, _) = self
            .roster
            .handles
            .get(index)
            .ok_or_else(|| ClientError::Dispatch(format!("no listed window {index}")))?;
        let object = self
            .manager
            .get_window_for(handle, &self.queue.handle(), ());
        self.roster.asked(object);
        self.flush()
    }

    /// `create_space` on the first output.
    pub fn create_space(&self) -> Result<(), ClientError> {
        self.manager.create_space(&self.output);
        self.flush()
    }

    /// `stop`: no more new objects; the compositor answers `finished`.
    pub fn stop(&self) -> Result<(), ClientError> {
        self.manager.stop();
        self.flush()
    }

    /// A request on the app announced at `index`.
    pub fn app(&self, index: usize, request: AppRequest) -> Result<(), ClientError> {
        let object = &self
            .roster
            .apps
            .get(index)
            .ok_or_else(|| ClientError::Dispatch(format!("no app {index}")))?
            .object;
        match request {
            AppRequest::Activate => object.activate(&self.seat, 0),
            AppRequest::Hide => object.hide(),
            AppRequest::Unhide => object.unhide(),
            AppRequest::HideOthers => object.hide_others(),
            AppRequest::Quit => object.quit(),
            AppRequest::ForceQuit => object.force_quit(),
            AppRequest::Reopen => object.reopen(),
        }
        self.flush()
    }

    /// A request on the window announced at `index`.
    pub fn window(&self, index: usize, request: WindowRequest) -> Result<(), ClientError> {
        let object = &self
            .roster
            .windows
            .iter()
            .filter(|seen| seen.origin == Origin::Announced)
            .nth(index)
            .ok_or_else(|| ClientError::Dispatch(format!("no window {index}")))?
            .object;
        match request {
            WindowRequest::Activate => object.activate(&self.seat, 0),
            WindowRequest::Close => object.close(),
            WindowRequest::Minimise => object.minimise(),
            WindowRequest::Unminimise => object.unminimise(),
            WindowRequest::Place(target) => object.place(target),
            WindowRequest::FullScreen(toggle) => object.full_screen(toggle),
            WindowRequest::ToSpace(space) => {
                let space = self
                    .roster
                    .spaces
                    .get(space)
                    .ok_or_else(|| ClientError::Dispatch(format!("no Space {space}")))?;
                object.to_space(&space.object);
            }
            WindowRequest::Assign(assign) => object.assign(assign),
        }
        self.flush()
    }

    /// A request on the Space announced at `index`.
    pub fn space(&self, index: usize, request: SpaceRequest) -> Result<(), ClientError> {
        let object = &self
            .roster
            .spaces
            .get(index)
            .ok_or_else(|| ClientError::Dispatch(format!("no Space {index}")))?
            .object;
        match request {
            SpaceRequest::Activate => object.activate(),
            SpaceRequest::Remove => object.remove(),
            SpaceRequest::MoveTo(position) => object.move_to(position),
        }
        self.flush()
    }

    fn flush(&self) -> Result<(), ClientError> {
        self.connection
            .flush()
            .map_err(|error| ClientError::Dispatch(error.to_string()))
    }
}
