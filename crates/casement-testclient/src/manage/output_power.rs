//! `zwlr_output_power_manager_v1` as a client (P1-M1): holds controls on outputs by connector
//! name, sets their mode and records the `mode` and `failed` events.
//!
//! Self-contained like `foreign_toplevel_list`: it binds the manager and every `wl_output` on a
//! side queue and reads their events there. Start it, let the fixture pump, pump the helper (the
//! outputs say their names), then ask for a control.

use crate::misc::idle::{pump_queue, side_queue};
use crate::{ClientError, GlobalList, Pumped};
use wayland_client::protocol::wl_output::{self, WlOutput};
use wayland_client::{Connection, Dispatch, EventQueue, QueueHandle};
use wayland_protocols_wlr::output_power_management::v1::client::{
    zwlr_output_power_manager_v1::{self, ZwlrOutputPowerManagerV1},
    zwlr_output_power_v1::{self, ZwlrOutputPowerV1},
};

/// Whether an output is lit, as the protocol says it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PowerMode {
    /// Lit.
    On,
    /// Dark.
    Off,
}

/// Which control a request or an answer is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ControlId(pub u32);

/// What a control has said so far.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ControlState {
    /// Nothing yet.
    Silent,
    /// The control reported this mode last.
    Reported(PowerMode),
    /// The compositor sent `failed`.
    Failed,
}

/// One control and what it said.
#[derive(Debug)]
struct Control {
    id: ControlId,
    proxy: ZwlrOutputPowerV1,
    state: ControlState,
    modes: Vec<PowerMode>,
}

/// What the side queue dispatches into.
#[derive(Debug, Default)]
struct Seen {
    outputs: Vec<(WlOutput, Option<String>)>,
    controls: Vec<Control>,
}

/// Records a `wl_output` event (shared with the gamma helper).
pub(super) fn note_output(
    outputs: &mut [(WlOutput, Option<String>)],
    proxy: &WlOutput,
    event: wl_output::Event,
) {
    if let (wl_output::Event::Name { name }, Some(slot)) = (
        event,
        outputs.iter_mut().find(|(output, _)| *output == *proxy),
    ) {
        slot.1 = Some(name);
    }
}

/// The `wl_output` called `name`, once it has said so.
pub(super) fn output_named<'a>(
    outputs: &'a [(WlOutput, Option<String>)],
    name: &str,
) -> Option<&'a WlOutput> {
    outputs
        .iter()
        .find(|(_, known)| known.as_deref() == Some(name))
        .map(|(output, _)| output)
}

/// Binds every `wl_output` the registry advertises (up to version 4, which has `name`).
pub(super) fn bind_outputs<S>(
    globals: &GlobalList,
    qh: &QueueHandle<S>,
) -> Vec<(WlOutput, Option<String>)>
where
    S: Dispatch<WlOutput, ()> + 'static,
{
    let advertised: Vec<(u32, u32)> = globals.contents().with_list(|list| {
        list.iter()
            .filter(|global| global.interface == "wl_output")
            .map(|global| (global.name, global.version))
            .collect()
    });
    advertised
        .into_iter()
        .map(|(name, version)| {
            let output = globals
                .registry()
                .bind::<WlOutput, _, _>(name, version.min(4), qh, ());
            (output, None)
        })
        .collect()
}

/// A bound `zwlr_output_power_manager_v1`.
#[derive(Debug)]
pub struct OutputPowers {
    connection: Connection,
    queue: EventQueue<Seen>,
    seen: Seen,
    manager: ZwlrOutputPowerManagerV1,
    next: u32,
}

impl OutputPowers {
    /// Binds the manager and every output; the compositor answers at the fixture's next pump.
    pub fn start(globals: &GlobalList) -> Result<OutputPowers, ClientError> {
        let (connection, queue) = side_queue::<Seen>(globals)?;
        let qh = queue.handle();
        let manager = globals
            .bind::<ZwlrOutputPowerManagerV1, _, _>(&qh, 1..=1, ())
            .map_err(|_| ClientError::MissingGlobal("zwlr_output_power_manager_v1".to_owned()))?;
        let outputs = bind_outputs(globals, &qh);
        connection
            .flush()
            .map_err(|error| ClientError::Dispatch(error.to_string()))?;
        Ok(OutputPowers {
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
            .get_output_power(output, &self.queue.handle(), id);
        self.seen.controls.push(Control {
            id,
            proxy,
            state: ControlState::Silent,
            modes: Vec::new(),
        });
        self.flush()
    }

    /// Asks for `mode` on a control.
    pub fn set_mode(&mut self, id: ControlId, mode: PowerMode) -> Result<(), ClientError> {
        let control = self.find(id)?;
        control.proxy.set_mode(match mode {
            PowerMode::On => zwlr_output_power_v1::Mode::On,
            PowerMode::Off => zwlr_output_power_v1::Mode::Off,
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

    /// Every mode a control reported, in order.
    pub fn reported(&self, id: ControlId) -> Vec<PowerMode> {
        self.seen
            .controls
            .iter()
            .find(|control| control.id == id)
            .map(|control| control.modes.clone())
            .unwrap_or_default()
    }

    /// Destroys a control.
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

impl Dispatch<ZwlrOutputPowerManagerV1, ()> for Seen {
    fn event(
        _state: &mut Seen,
        _proxy: &ZwlrOutputPowerManagerV1,
        _event: zwlr_output_power_manager_v1::Event,
        _data: &(),
        _connection: &Connection,
        _qh: &QueueHandle<Seen>,
    ) {
    }
}

impl Dispatch<ZwlrOutputPowerV1, ControlId> for Seen {
    fn event(
        state: &mut Seen,
        _proxy: &ZwlrOutputPowerV1,
        event: zwlr_output_power_v1::Event,
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
            zwlr_output_power_v1::Event::Mode { mode } => {
                let reported = match mode.into_result() {
                    Ok(zwlr_output_power_v1::Mode::On) => PowerMode::On,
                    _ => PowerMode::Off,
                };
                control.modes.push(reported);
                control.state = ControlState::Reported(reported);
            }
            zwlr_output_power_v1::Event::Failed => control.state = ControlState::Failed,
            _ => {}
        }
    }
}
