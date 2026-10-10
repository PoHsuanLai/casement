//! `zwlr_gamma_control_manager_v1` as a client (P1-M1): takes controls on outputs by connector
//! name, learns the table size, sends ramps and records `failed`.
//!
//! Self-contained like `output_power`, whose output binding it reuses.

use super::output_power::{bind_outputs, note_output, output_named};
use crate::misc::idle::{pump_queue, side_queue};
use crate::{ClientError, GlobalList, Pumped};
use rustix::fs::{MemfdFlags, memfd_create};
use std::os::fd::{AsFd, OwnedFd};
use wayland_client::protocol::wl_output::{self, WlOutput};
use wayland_client::{Connection, Dispatch, EventQueue, QueueHandle};
use wayland_protocols_wlr::gamma_control::v1::client::{
    zwlr_gamma_control_manager_v1::{self, ZwlrGammaControlManagerV1},
    zwlr_gamma_control_v1::{self, ZwlrGammaControlV1},
};

/// Which control a request or an answer is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ControlId(pub u32);

/// What a control has said so far.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ControlState {
    /// Nothing yet.
    Silent,
    /// The control announced this table size (steps per channel).
    Sized(u32),
    /// The compositor sent `failed`.
    Failed,
}

/// One control and what it said.
#[derive(Debug)]
struct Control {
    id: ControlId,
    proxy: ZwlrGammaControlV1,
    state: ControlState,
}

/// What the side queue dispatches into.
#[derive(Debug, Default)]
struct Seen {
    outputs: Vec<(WlOutput, Option<String>)>,
    controls: Vec<Control>,
}

/// A bound `zwlr_gamma_control_manager_v1`.
#[derive(Debug)]
pub struct GammaControls {
    connection: Connection,
    queue: EventQueue<Seen>,
    seen: Seen,
    manager: ZwlrGammaControlManagerV1,
    next: u32,
}

impl GammaControls {
    /// Binds the manager and every output; the compositor answers at the fixture's next pump.
    pub fn start(globals: &GlobalList) -> Result<GammaControls, ClientError> {
        let (connection, queue) = side_queue::<Seen>(globals)?;
        let qh = queue.handle();
        let manager = globals
            .bind::<ZwlrGammaControlManagerV1, _, _>(&qh, 1..=1, ())
            .map_err(|_| ClientError::MissingGlobal("zwlr_gamma_control_manager_v1".to_owned()))?;
        let outputs = bind_outputs(globals, &qh);
        connection
            .flush()
            .map_err(|error| ClientError::Dispatch(error.to_string()))?;
        Ok(GammaControls {
            connection,
            queue,
            seen: Seen {
                outputs,
                controls: Vec::new(),
            },
            manager,
            next: 0,
        })
    }

    /// Reads what arrived.
    pub fn pump(&mut self) -> Result<Pumped, ClientError> {
        pump_queue(&self.connection, &mut self.queue, &mut self.seen)
    }

    /// Asks for a control on the output called `name`; an error when no output said that name.
    pub fn control(&mut self, name: &str) -> Result<ControlId, ClientError> {
        let output = output_named(&self.seen.outputs, name)
            .ok_or_else(|| ClientError::Dispatch(format!("no output named {name}")))?;
        let id = ControlId(self.next);
        self.next += 1;
        let proxy = self
            .manager
            .get_gamma_control(output, &self.queue.handle(), id);
        self.seen.controls.push(Control {
            id,
            proxy,
            state: ControlState::Silent,
        });
        self.flush()
    }

    /// What a control has said last.
    pub fn state(&self, id: ControlId) -> Option<ControlState> {
        self.seen
            .controls
            .iter()
            .find(|control| control.id == id)
            .map(|control| control.state)
    }

    /// Sends a ramp: `red`, `green` and `blue` in turn, 16-bit native endian, in a memfd.
    pub fn set_ramp(
        &mut self,
        id: ControlId,
        red: &[u16],
        green: &[u16],
        blue: &[u16],
    ) -> Result<(), ClientError> {
        let bytes: Vec<u8> = red
            .iter()
            .chain(green)
            .chain(blue)
            .flat_map(|step| step.to_ne_bytes())
            .collect();
        let fd: OwnedFd = memfd_create("casement-gamma", MemfdFlags::CLOEXEC)
            .map_err(|error| ClientError::Dispatch(format!("memfd: {error}")))?;
        rustix::io::write(&fd, &bytes)
            .map_err(|error| ClientError::Dispatch(format!("ramp write: {error}")))?;
        let control = self.find(id)?;
        control.proxy.set_gamma(fd.as_fd());
        self.flush()
    }

    /// Destroys a control (the compositor restores the identity curve).
    pub fn destroy(&mut self, id: ControlId) -> Result<(), ClientError> {
        let control = self.find(id)?;
        control.proxy.destroy();
        self.seen.controls.retain(|control| control.id != id);
        self.flush()
    }

    fn find(&self, id: ControlId) -> Result<&Control, ClientError> {
        self.seen
            .controls
            .iter()
            .find(|control| control.id == id)
            .ok_or_else(|| ClientError::Dispatch(format!("no control {id:?}")))
    }

    fn flush(&self) -> Result<(), ClientError> {
        self.connection
            .flush()
            .map_err(|error| ClientError::Dispatch(error.to_string()))
    }
}

impl Dispatch<WlOutput, ()> for Seen {
    fn event(
        state: &mut Seen,
        proxy: &WlOutput,
        event: wl_output::Event,
        _data: &(),
        _connection: &Connection,
        _qh: &QueueHandle<Seen>,
    ) {
        note_output(&mut state.outputs, proxy, event);
    }
}

impl Dispatch<ZwlrGammaControlManagerV1, ()> for Seen {
    fn event(
        _state: &mut Seen,
        _proxy: &ZwlrGammaControlManagerV1,
        _event: zwlr_gamma_control_manager_v1::Event,
        _data: &(),
        _connection: &Connection,
        _qh: &QueueHandle<Seen>,
    ) {
    }
}

impl Dispatch<ZwlrGammaControlV1, ControlId> for Seen {
    fn event(
        state: &mut Seen,
        _proxy: &ZwlrGammaControlV1,
        event: zwlr_gamma_control_v1::Event,
        data: &ControlId,
        _connection: &Connection,
        _qh: &QueueHandle<Seen>,
    ) {
        let Some(control) = state
            .controls
            .iter_mut()
            .find(|control| control.id == *data)
        else {
            return;
        };
        match event {
            zwlr_gamma_control_v1::Event::GammaSize { size } => {
                control.state = ControlState::Sized(size);
            }
            zwlr_gamma_control_v1::Event::Failed => control.state = ControlState::Failed,
            _ => {}
        }
    }
}
