//! `zwlr_output_power_manager_v1`: a client (an idle daemon, sill's lock) lights an output or
//! turns it dark. Works on virtual outputs too: power is part of `OutputConfig`, so a request
//! is one `Event::OutputRequested` that changes only the power, and the `mode` event goes to
//! the control once the model's configuration says so ([`follow`]).
//!
//! One control per output: a second control on an output, or one on an output that is gone,
//! says `failed` at once. A control whose output is unplugged says `failed` and lets go. Hidden
//! from sandboxed clients (I7).
//!
//! Changing the displays is a power (`Power::Displays`): a control asked for while the person has
//! not answered is made but inert (it hears nothing and its requests change nothing) until they
//! allow; refused, it says `failed`. A power taken back fails the controls in use and lights an
//! output they left dark.

use crate::consent::{self, Admission, ConsentHandler, Held, Mortal, Pushed};
use crate::states::{GlobalFilter, WaylandFault};
use casement_core::output::{OutputChangeSet, OutputPower};
use casement_core::policy::GlobalKind;
use casement_core::privilege::PowerOutcome;
use casement_core::{ClientKey, Event, OutputBook, OutputKey};
use casement_protocols::vocab::Power;
use smithay::reexports::wayland_protocols_wlr::output_power_management::v1::server::{
    zwlr_output_power_manager_v1::{self, ZwlrOutputPowerManagerV1},
    zwlr_output_power_v1::{self, ZwlrOutputPowerV1},
};
use smithay::reexports::wayland_server::backend::{ClientId, GlobalId};
use smithay::reexports::wayland_server::protocol::wl_output::WlOutput;
use smithay::reexports::wayland_server::{
    Client, DataInit, Dispatch, DisplayHandle, GlobalDispatch, New, Resource,
};
use smithay::wayland::{Dispatch2, GlobalDispatch2};
use std::collections::{BTreeMap, BTreeSet};

/// The version casement serves.
pub const VERSION: u32 = 1;

/// `zwlr_output_power_manager_v1` and the outputs a client holds a control on.
#[derive(Debug, Default)]
pub struct OutputPowerPart {
    /// The outputs with a live `zwlr_output_power_v1`.
    pub controls: BTreeSet<OutputKey>,
    /// Our global (none until [`OutputPowerPart::new`]).
    pub global: Option<GlobalId>,
    objects: BTreeMap<OutputKey, ZwlrOutputPowerV1>,
    shown: BTreeMap<OutputKey, OutputPower>,
    /// The outputs a control asked to darken and has not lit again: lit when it goes (CASC-09).
    darkened: BTreeSet<OutputKey>,
    /// The controls made while the person had not answered, inert until they do.
    waiting: Held<WaitingControl>,
}

/// A control that waits for the person's answer about the displays.
#[derive(Debug)]
pub struct WaitingControl {
    control: ZwlrOutputPowerV1,
    output: OutputKey,
}

impl Mortal for WaitingControl {
    fn alive(&self) -> bool {
        self.control.is_alive()
    }
}

impl OutputPowerPart {
    /// Creates the global (version 1), filtered by the I7 table.
    pub fn new<D>(display: &DisplayHandle, role_filter: GlobalFilter) -> OutputPowerPart
    where
        D: GlobalDispatch<ZwlrOutputPowerManagerV1, OutputPowerGlobal> + 'static,
    {
        OutputPowerPart {
            global: Some(display.create_global::<D, ZwlrOutputPowerManagerV1, _>(
                VERSION,
                OutputPowerGlobal {
                    filter: role_filter,
                },
            )),
            ..OutputPowerPart::default()
        }
    }
}

/// What the composition root provides to this protocol's dispatch.
pub trait OutputPowerHandler: ConsentHandler + Sized {
    /// The part.
    fn output_power_part(&mut self) -> &mut OutputPowerPart;
    /// The output a client's `wl_output` names.
    fn output_power_output(&self, output: &WlOutput) -> Option<OutputKey>;
    /// Whether the model has `output` lit; none when it is not plugged.
    fn output_power_of(&self, output: &OutputKey) -> Option<OutputPower>;
    /// Feeds the events a request produced into the model (`core_loop::feed`).
    fn output_power_events(&mut self, events: Vec<Event>);
}

/// The global's data: the filter its `can_view` asks.
#[derive(Debug, Clone, Copy)]
pub struct OutputPowerGlobal {
    filter: GlobalFilter,
}

/// A bound manager's data.
#[derive(Debug, Clone, Copy, Default)]
pub struct OutputPowerManagerData;

/// One control's data: the output it holds, none for an inert (failed) control.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PowerData {
    output: Option<OutputKey>,
}

/// A client's `set_mode` on `output`: one `Event::OutputRequested` that changes only the power
/// (an output it darkens is lit again when its control goes, [`on_released`]).
///
/// ```no_run
/// use casement_core::output::OutputPower;
/// use casement_wayland::output_power::{OutputPowerPart, on_set_mode};
///
/// # fn demo(key: casement_core::OutputKey) {
/// let mut part = OutputPowerPart::default();
/// let events = on_set_mode(&mut part, key, OutputPower::Off);
/// assert_eq!(events.len(), 1);
/// # }
/// ```
pub fn on_set_mode(
    part: &mut OutputPowerPart,
    output: OutputKey,
    power: OutputPower,
) -> Vec<Event> {
    match power {
        OutputPower::Off => part.darkened.insert(output.clone()),
        OutputPower::On => part.darkened.remove(&output),
    };
    vec![Event::OutputRequested {
        output,
        change: OutputChangeSet::none().with_power(power),
    }]
}

/// The control on `output` went away (destroyed, or its client died): an output it left dark
/// is lit again, so a client that darkens a display and disappears cannot leave it that way.
pub fn on_released(part: &mut OutputPowerPart, output: &OutputKey) -> Vec<Event> {
    part.objects.remove(output);
    part.controls.remove(output);
    part.shown.remove(output);
    match part.darkened.remove(output) {
        true => vec![Event::OutputRequested {
            output: output.clone(),
            change: OutputChangeSet::none().with_power(OutputPower::On),
        }],
        false => Vec::new(),
    }
}

/// Tells the holders of a control on `output` its power is now `power` (`mode` event).
pub fn apply_mode(
    part: &mut OutputPowerPart,
    output: &OutputKey,
    power: OutputPower,
) -> Result<(), WaylandFault> {
    let control = part.objects.get(output).ok_or(WaylandFault::SurfaceGone)?;
    control.mode(wire_mode(power));
    part.shown.insert(output.clone(), power);
    Ok(())
}

/// Brings the controls up to date with the model: a control whose output's power moved is
/// told, one whose output is unplugged says `failed` and lets go.
///
/// ```no_run
/// use casement_core::OutputBook;
/// use casement_wayland::output_power::{OutputPowerPart, follow};
///
/// let mut part = OutputPowerPart::default();
/// follow(&mut part, &OutputBook::default());
/// assert!(part.controls.is_empty());
/// ```
pub fn follow(part: &mut OutputPowerPart, outputs: &OutputBook) {
    let keys: Vec<OutputKey> = part.objects.keys().cloned().collect();
    for key in keys {
        match outputs.0.get(&key) {
            None => {
                part.controls.remove(&key);
                part.shown.remove(&key);
                part.darkened.remove(&key);
                if let Some(control) = part.objects.remove(&key) {
                    control.failed();
                }
            }
            Some(plugged) if part.shown.get(&key) != Some(&plugged.config.power) => {
                let _ = apply_mode(part, &key, plugged.config.power);
            }
            Some(_) => {}
        }
    }
}

fn wire_mode(power: OutputPower) -> zwlr_output_power_v1::Mode {
    match power {
        OutputPower::On => zwlr_output_power_v1::Mode::On,
        OutputPower::Off => zwlr_output_power_v1::Mode::Off,
    }
}

impl<D> GlobalDispatch2<ZwlrOutputPowerManagerV1, D> for OutputPowerGlobal
where
    D: Dispatch<ZwlrOutputPowerManagerV1, OutputPowerManagerData> + 'static,
{
    fn bind(
        &self,
        _state: &mut D,
        _handle: &DisplayHandle,
        _client: &Client,
        resource: New<ZwlrOutputPowerManagerV1>,
        data_init: &mut DataInit<'_, D>,
    ) {
        data_init.init(resource, OutputPowerManagerData);
    }

    fn can_view(&self, client: &Client) -> bool {
        self.filter.sees(GlobalKind::OutputPower, client)
    }
}

impl<D> Dispatch2<ZwlrOutputPowerManagerV1, D> for OutputPowerManagerData
where
    D: OutputPowerHandler + Dispatch<ZwlrOutputPowerV1, PowerData>,
{
    fn request(
        &self,
        state: &mut D,
        client: &Client,
        _resource: &ZwlrOutputPowerManagerV1,
        request: zwlr_output_power_manager_v1::Request,
        _handle: &DisplayHandle,
        data_init: &mut DataInit<'_, D>,
    ) {
        let zwlr_output_power_manager_v1::Request::GetOutputPower { id, output } = request else {
            return;
        };
        let key = state.output_power_output(&output);
        let power = key.as_ref().and_then(|key| state.output_power_of(key));
        let held = key.as_ref().is_some_and(|key| {
            state
                .output_power_part()
                .objects
                .get(key)
                .is_some_and(|control| control.is_alive())
        });
        let Some((key, power)) = key.zip(power).filter(|_| !held) else {
            data_init.init(id, PowerData { output: None }).failed();
            return;
        };
        let control = data_init.init(
            id,
            PowerData {
                output: Some(key.clone()),
            },
        );
        match consent::admit(state, client, Power::Displays) {
            Admission::Go => register(state.output_power_part(), control, &key, power),
            Admission::Hold => wait(state, client, control, key),
            Admission::Refuse => control.failed(),
        }
    }
}

/// Makes `control` the one that holds `output`, and tells it the output's power.
fn register(
    part: &mut OutputPowerPart,
    control: ZwlrOutputPowerV1,
    output: &OutputKey,
    power: OutputPower,
) {
    part.objects.insert(output.clone(), control);
    part.controls.insert(output.clone());
    if let Err(fault) = apply_mode(part, output, power) {
        tracing::warn!(%fault, "a power control lost its output at once");
    }
}

/// Keeps `control` inert until the answer comes; one past the cap of waiting requests, and one
/// from a client nobody vouches for, fails.
fn wait<D: OutputPowerHandler>(
    state: &mut D,
    client: &Client,
    control: ZwlrOutputPowerV1,
    output: OutputKey,
) {
    let kept = WaitingControl { control, output };
    let refused = match consent::key_of(client) {
        Some(key) => match state.output_power_part().waiting.push(key, kept) {
            Pushed::Kept => return,
            Pushed::Full(full) => full,
        },
        None => kept,
    };
    refused.control.failed();
}

/// The answer about `client`'s power over the displays came out. Allowed, the controls that
/// waited take their outputs (one whose output is gone, or held by another control by now,
/// fails); refused, they fail; taken back, the controls in use fail too and the outputs they
/// left dark are lit.
pub fn on_resolved<D: OutputPowerHandler>(state: &mut D, client: ClientKey, outcome: PowerOutcome) {
    let waiting = state.output_power_part().waiting.take(client);
    match outcome {
        PowerOutcome::Allowed => {
            for held in waiting {
                let power = state.output_power_of(&held.output);
                let part = state.output_power_part();
                let free = !part
                    .objects
                    .get(&held.output)
                    .is_some_and(|control| control.is_alive());
                match power.filter(|_| free) {
                    Some(power) => register(part, held.control, &held.output, power),
                    None => held.control.failed(),
                }
            }
        }
        PowerOutcome::Denied | PowerOutcome::Revoked => {
            waiting.iter().for_each(|held| held.control.failed());
            let part = state.output_power_part();
            let ended: Vec<OutputKey> = part
                .objects
                .iter()
                .filter(|(_, control)| {
                    control.client().as_ref().and_then(consent::key_of) == Some(client)
                })
                .map(|(output, _)| output.clone())
                .collect();
            let mut events = Vec::new();
            for output in ended {
                if let Some(control) = part.objects.get(&output) {
                    control.failed();
                }
                events.extend(on_released(part, &output));
            }
            if !events.is_empty() {
                state.output_power_events(events);
            }
        }
    }
}

impl<D> Dispatch2<ZwlrOutputPowerV1, D> for PowerData
where
    D: OutputPowerHandler,
{
    fn request(
        &self,
        state: &mut D,
        _client: &Client,
        resource: &ZwlrOutputPowerV1,
        request: zwlr_output_power_v1::Request,
        _handle: &DisplayHandle,
        _data_init: &mut DataInit<'_, D>,
    ) {
        let zwlr_output_power_v1::Request::SetMode { mode } = request else {
            return;
        };
        let Some(output) = &self.output else {
            return;
        };
        // Only the control that holds the output acts: one that waits for the person, or
        // was failed, changes nothing.
        if state.output_power_part().objects.get(output) != Some(resource) {
            return;
        }
        let power = match mode.into_result() {
            Ok(zwlr_output_power_v1::Mode::On) => OutputPower::On,
            Ok(zwlr_output_power_v1::Mode::Off) => OutputPower::Off,
            _ => {
                resource.post_error(
                    zwlr_output_power_v1::Error::InvalidMode,
                    "the power mode is neither on nor off",
                );
                return;
            }
        };
        let events = on_set_mode(state.output_power_part(), output.clone(), power);
        state.output_power_events(events);
    }

    fn destroyed(&self, state: &mut D, _client: ClientId, resource: &ZwlrOutputPowerV1) {
        let Some(output) = &self.output else {
            return;
        };
        let part = state.output_power_part();
        if part.objects.get(output) == Some(resource) {
            let events = on_released(part, output);
            state.output_power_events(events);
        }
    }
}
