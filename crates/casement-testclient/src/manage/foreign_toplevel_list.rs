//! `ext_foreign_toplevel_list_v1` as a client (P0-H8): every window the compositor lists, with
//! the identifier, title and app id it announced and whether it was closed.
//!
//! Self-contained like `misc::idle`, whose side queue it reuses.

use crate::misc::idle::{pump_queue, side_queue};
use crate::{ClientError, GlobalList, Pumped};
use wayland_client::{Connection, Dispatch, EventQueue, QueueHandle, event_created_child};
use wayland_protocols::ext::foreign_toplevel_list::v1::client::{
    ext_foreign_toplevel_handle_v1::{self, ExtForeignToplevelHandleV1},
    ext_foreign_toplevel_list_v1::{self, ExtForeignToplevelListV1},
};

/// Whether a listed window is still open.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Listing {
    /// Open.
    #[default]
    Open,
    /// The compositor sent `closed`.
    Closed,
}

/// One window as the list announced it (updates apply at `done`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Listed {
    /// The stable identifier.
    pub identifier: String,
    /// The title after the last `done`.
    pub title: String,
    /// The app id after the last `done`.
    pub app_id: String,
    /// Whether it is still open.
    pub listing: Listing,
}

/// A handle and what it said so far (pending until `done`).
#[derive(Debug)]
struct Entry {
    handle: ExtForeignToplevelHandleV1,
    shown: Listed,
    pending: Listed,
}

/// What the list announced.
#[derive(Debug, Default)]
struct Roster {
    entries: Vec<Entry>,
    finished: bool,
}

/// A bound `ext_foreign_toplevel_list_v1`.
#[derive(Debug)]
pub struct ToplevelList {
    connection: Connection,
    queue: EventQueue<Roster>,
    roster: Roster,
    list: ExtForeignToplevelListV1,
}

impl ToplevelList {
    /// Binds the list; the compositor announces its windows at the fixture's next pump.
    pub fn start(globals: &GlobalList) -> Result<ToplevelList, ClientError> {
        let (connection, queue) = side_queue::<Roster>(globals)?;
        let qh = queue.handle();
        let list = globals
            .bind::<ExtForeignToplevelListV1, _, _>(&qh, 1..=1, ())
            .map_err(|_| ClientError::MissingGlobal("ext_foreign_toplevel_list_v1".to_owned()))?;
        connection
            .flush()
            .map_err(|error| ClientError::Dispatch(error.to_string()))?;
        Ok(ToplevelList {
            connection,
            queue,
            roster: Roster::default(),
            list,
        })
    }

    /// Reads what arrived.
    pub fn pump(&mut self) -> Result<Pumped, ClientError> {
        pump_queue(&self.connection, &mut self.queue, &mut self.roster)
    }

    /// Every window announced so far, in order, as of its last `done`.
    pub fn listed(&self) -> Vec<Listed> {
        self.roster
            .entries
            .iter()
            .map(|entry| entry.shown.clone())
            .collect()
    }

    /// Asks the compositor to stop (`stop`); it answers `finished`.
    pub fn stop(&mut self) -> Result<(), ClientError> {
        self.list.stop();
        self.connection
            .flush()
            .map_err(|error| ClientError::Dispatch(error.to_string()))
    }

    /// Whether the compositor sent `finished`.
    pub fn finished(&self) -> bool {
        self.roster.finished
    }
}

impl Dispatch<ExtForeignToplevelListV1, ()> for Roster {
    fn event(
        state: &mut Roster,
        _proxy: &ExtForeignToplevelListV1,
        event: ext_foreign_toplevel_list_v1::Event,
        _data: &(),
        _connection: &Connection,
        _qh: &QueueHandle<Roster>,
    ) {
        match event {
            ext_foreign_toplevel_list_v1::Event::Toplevel { toplevel } => {
                let blank = Listed {
                    identifier: String::new(),
                    title: String::new(),
                    app_id: String::new(),
                    listing: Listing::Open,
                };
                state.entries.push(Entry {
                    handle: toplevel,
                    shown: blank.clone(),
                    pending: blank,
                });
            }
            ext_foreign_toplevel_list_v1::Event::Finished => state.finished = true,
            _ => {}
        }
    }

    event_created_child!(Roster, ExtForeignToplevelListV1, [
        ext_foreign_toplevel_list_v1::EVT_TOPLEVEL_OPCODE => (ExtForeignToplevelHandleV1, ()),
    ]);
}

impl Dispatch<ExtForeignToplevelHandleV1, ()> for Roster {
    fn event(
        state: &mut Roster,
        proxy: &ExtForeignToplevelHandleV1,
        event: ext_foreign_toplevel_handle_v1::Event,
        _data: &(),
        _connection: &Connection,
        _qh: &QueueHandle<Roster>,
    ) {
        let Some(entry) = state
            .entries
            .iter_mut()
            .find(|entry| &entry.handle == proxy)
        else {
            return;
        };
        match event {
            ext_foreign_toplevel_handle_v1::Event::Identifier { identifier } => {
                entry.pending.identifier = identifier;
            }
            ext_foreign_toplevel_handle_v1::Event::Title { title } => entry.pending.title = title,
            ext_foreign_toplevel_handle_v1::Event::AppId { app_id } => {
                entry.pending.app_id = app_id
            }
            ext_foreign_toplevel_handle_v1::Event::Done => entry.shown = entry.pending.clone(),
            ext_foreign_toplevel_handle_v1::Event::Closed => {
                entry.pending.listing = Listing::Closed;
                entry.shown.listing = Listing::Closed;
            }
            _ => {}
        }
    }
}
