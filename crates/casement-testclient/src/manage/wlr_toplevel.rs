//! `zwlr_foreign_toplevel_manager_v1` as a client (P1-M2): every window the compositor lists,
//! with the title, app id, states and number of outputs it announced (as of the last `done`),
//! and the requests a dock or switcher makes on a handle.
//!
//! Self-contained like `foreign_toplevel_list`: it reads its own events on a side queue of the
//! client's connection. It binds the seat and every `wl_output` first, so the compositor has
//! `wl_output` objects to name in `output_enter`.

use crate::misc::idle::{pump_queue, side_queue};
use crate::{ClientError, GlobalList, Pumped};
use wayland_client::protocol::wl_output::{self, WlOutput};
use wayland_client::protocol::wl_seat::{self, WlSeat};
use wayland_client::protocol::wl_surface::WlSurface;
use wayland_client::{Connection, Dispatch, EventQueue, QueueHandle, event_created_child};
use wayland_protocols_wlr::foreign_toplevel::v1::client::{
    zwlr_foreign_toplevel_handle_v1::{self, ZwlrForeignToplevelHandleV1},
    zwlr_foreign_toplevel_manager_v1::{self, ZwlrForeignToplevelManagerV1},
};

/// A state a listed window can be in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum WlrState {
    /// `maximized`.
    Maximized,
    /// `minimized`.
    Minimized,
    /// `activated`.
    Activated,
    /// `fullscreen`.
    Fullscreen,
}

/// One window as the list announced it (updates apply at `done`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WlrListed {
    /// The title after the last `done`.
    pub title: String,
    /// The app id after the last `done`.
    pub app_id: String,
    /// Its states after the last `done`, in the order sent.
    pub states: Vec<WlrState>,
    /// How many `wl_output`s it is on after the last `done`.
    pub outputs: usize,
    /// Whether the compositor sent `closed`.
    pub closed: Closing,
}

/// Whether a listed window was withdrawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Closing {
    /// Still listed.
    #[default]
    Open,
    /// The compositor sent `closed`.
    Closed,
}

/// A handle and what it said so far.
#[derive(Debug)]
struct Entry {
    handle: ZwlrForeignToplevelHandleV1,
    shown: WlrListed,
    pending: WlrListed,
    on: Vec<WlOutput>,
}

/// What the manager announced.
#[derive(Debug, Default)]
struct Roster {
    entries: Vec<Entry>,
    finished: bool,
}

/// A bound `zwlr_foreign_toplevel_manager_v1`.
#[derive(Debug)]
pub struct ToplevelWatch {
    connection: Connection,
    queue: EventQueue<Roster>,
    roster: Roster,
    manager: ZwlrForeignToplevelManagerV1,
    seat: WlSeat,
    _outputs: Vec<WlOutput>,
}

impl ToplevelWatch {
    /// Binds the seat, the first `wl_output` and the manager; the compositor announces its
    /// windows at the fixture's next pump.
    pub fn start(globals: &GlobalList) -> Result<ToplevelWatch, ClientError> {
        let (connection, queue) = side_queue::<Roster>(globals)?;
        let qh = queue.handle();
        let missing = |name: &str| ClientError::MissingGlobal(name.to_owned());
        let seat = globals
            .bind::<WlSeat, _, _>(&qh, 1..=1, ())
            .map_err(|_| missing("wl_seat"))?;
        let output = globals
            .bind::<WlOutput, _, _>(&qh, 1..=4, ())
            .map_err(|_| missing("wl_output"))?;
        let manager = globals
            .bind::<ZwlrForeignToplevelManagerV1, _, _>(&qh, 1..=3, ())
            .map_err(|_| missing("zwlr_foreign_toplevel_manager_v1"))?;
        connection
            .flush()
            .map_err(|error| ClientError::Dispatch(error.to_string()))?;
        Ok(ToplevelWatch {
            connection,
            queue,
            roster: Roster::default(),
            manager,
            seat,
            _outputs: vec![output],
        })
    }

    /// Reads what arrived.
    pub fn pump(&mut self) -> Result<Pumped, ClientError> {
        pump_queue(&self.connection, &mut self.queue, &mut self.roster)
    }

    /// Every window announced so far, in order, as of its last `done`.
    pub fn listed(&self) -> Vec<WlrListed> {
        self.roster
            .entries
            .iter()
            .map(|entry| entry.shown.clone())
            .collect()
    }

    /// Asks the compositor to stop (`stop`); it answers `finished`.
    pub fn stop(&mut self) -> Result<(), ClientError> {
        self.manager.stop();
        self.flush()
    }

    /// Whether the compositor sent `finished`.
    pub fn finished(&self) -> bool {
        self.roster.finished
    }

    /// `set_maximized` on window `index`.
    pub fn maximize(&mut self, index: usize) -> Result<(), ClientError> {
        self.on(index, |handle, _| handle.set_maximized())
    }

    /// `unset_maximized` on window `index`.
    pub fn unmaximize(&mut self, index: usize) -> Result<(), ClientError> {
        self.on(index, |handle, _| handle.unset_maximized())
    }

    /// `set_minimized` on window `index`.
    pub fn minimize(&mut self, index: usize) -> Result<(), ClientError> {
        self.on(index, |handle, _| handle.set_minimized())
    }

    /// `unset_minimized` on window `index`.
    pub fn unminimize(&mut self, index: usize) -> Result<(), ClientError> {
        self.on(index, |handle, _| handle.unset_minimized())
    }

    /// `set_fullscreen` on window `index`, on no output in particular.
    pub fn fullscreen(&mut self, index: usize) -> Result<(), ClientError> {
        self.on(index, |handle, _| handle.set_fullscreen(None))
    }

    /// `unset_fullscreen` on window `index`.
    pub fn unfullscreen(&mut self, index: usize) -> Result<(), ClientError> {
        self.on(index, |handle, _| handle.unset_fullscreen())
    }

    /// `activate` window `index` on the client's seat.
    pub fn activate(&mut self, index: usize) -> Result<(), ClientError> {
        self.on(index, |handle, seat| handle.activate(seat))
    }

    /// `close` window `index`.
    pub fn close(&mut self, index: usize) -> Result<(), ClientError> {
        self.on(index, |handle, _| handle.close())
    }

    /// `set_rectangle` on window `index`: the tile `(x, y, w, h)` of `surface` stands for it.
    pub fn set_rectangle(
        &mut self,
        index: usize,
        surface: &WlSurface,
        rect: (i32, i32, i32, i32),
    ) -> Result<(), ClientError> {
        self.on(index, |handle, _| {
            handle.set_rectangle(surface, rect.0, rect.1, rect.2, rect.3)
        })
    }

    /// `destroy` window `index`'s handle.
    pub fn destroy(&mut self, index: usize) -> Result<(), ClientError> {
        self.on(index, |handle, _| handle.destroy())
    }

    /// Runs `request` on window `index`'s handle and flushes.
    fn on(
        &mut self,
        index: usize,
        request: impl FnOnce(&ZwlrForeignToplevelHandleV1, &WlSeat),
    ) -> Result<(), ClientError> {
        let entry = self
            .roster
            .entries
            .get(index)
            .ok_or_else(|| ClientError::Dispatch(format!("no listed window {index}")))?;
        request(&entry.handle, &self.seat);
        self.flush()
    }

    fn flush(&self) -> Result<(), ClientError> {
        self.connection
            .flush()
            .map_err(|error| ClientError::Dispatch(error.to_string()))
    }
}

impl Dispatch<ZwlrForeignToplevelManagerV1, ()> for Roster {
    fn event(
        state: &mut Roster,
        _proxy: &ZwlrForeignToplevelManagerV1,
        event: zwlr_foreign_toplevel_manager_v1::Event,
        _data: &(),
        _connection: &Connection,
        _qh: &QueueHandle<Roster>,
    ) {
        match event {
            zwlr_foreign_toplevel_manager_v1::Event::Toplevel { toplevel } => {
                state.entries.push(Entry {
                    handle: toplevel,
                    shown: WlrListed::default(),
                    pending: WlrListed::default(),
                    on: Vec::new(),
                });
            }
            zwlr_foreign_toplevel_manager_v1::Event::Finished => state.finished = true,
            _ => {}
        }
    }

    event_created_child!(Roster, ZwlrForeignToplevelManagerV1, [
        zwlr_foreign_toplevel_manager_v1::EVT_TOPLEVEL_OPCODE => (ZwlrForeignToplevelHandleV1, ()),
    ]);
}

impl Dispatch<ZwlrForeignToplevelHandleV1, ()> for Roster {
    fn event(
        state: &mut Roster,
        proxy: &ZwlrForeignToplevelHandleV1,
        event: zwlr_foreign_toplevel_handle_v1::Event,
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
            zwlr_foreign_toplevel_handle_v1::Event::Title { title } => entry.pending.title = title,
            zwlr_foreign_toplevel_handle_v1::Event::AppId { app_id } => {
                entry.pending.app_id = app_id;
            }
            zwlr_foreign_toplevel_handle_v1::Event::OutputEnter { output } => {
                if !entry.on.contains(&output) {
                    entry.on.push(output);
                }
            }
            zwlr_foreign_toplevel_handle_v1::Event::OutputLeave { output } => {
                entry.on.retain(|held| held != &output);
            }
            zwlr_foreign_toplevel_handle_v1::Event::State { state: bytes } => {
                entry.pending.states = states_of(&bytes);
            }
            zwlr_foreign_toplevel_handle_v1::Event::Done => {
                entry.pending.outputs = entry.on.len();
                entry.shown = entry.pending.clone();
            }
            zwlr_foreign_toplevel_handle_v1::Event::Closed => {
                entry.pending.closed = Closing::Closed;
                entry.shown.closed = Closing::Closed;
            }
            _ => {}
        }
    }
}

/// The states a `state` array names (native-endian words; a code this client does not know is
/// skipped).
fn states_of(bytes: &[u8]) -> Vec<WlrState> {
    bytes
        .chunks_exact(4)
        .filter_map(
            |word| match u32::from_ne_bytes([word[0], word[1], word[2], word[3]]) {
                0 => Some(WlrState::Maximized),
                1 => Some(WlrState::Minimized),
                2 => Some(WlrState::Activated),
                3 => Some(WlrState::Fullscreen),
                _ => None,
            },
        )
        .collect()
}

impl Dispatch<WlSeat, ()> for Roster {
    fn event(
        _state: &mut Roster,
        _proxy: &WlSeat,
        _event: wl_seat::Event,
        _data: &(),
        _connection: &Connection,
        _qh: &QueueHandle<Roster>,
    ) {
    }
}

impl Dispatch<WlOutput, ()> for Roster {
    fn event(
        _state: &mut Roster,
        _proxy: &WlOutput,
        _event: wl_output::Event,
        _data: &(),
        _connection: &Connection,
        _qh: &QueueHandle<Roster>,
    ) {
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_state_array_is_read_word_by_word() {
        let bytes: Vec<u8> = [2u32, 0, 9]
            .iter()
            .flat_map(|code| code.to_ne_bytes())
            .collect();
        assert_eq!(
            states_of(&bytes),
            [WlrState::Activated, WlrState::Maximized],
            "an unknown code is skipped"
        );
        assert!(states_of(&[]).is_empty());
    }
}
