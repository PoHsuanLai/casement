//! `zwlr_gamma_control_manager_v1`: a client (night light) drives an output's colour curve.
//! The ramp becomes `Event::GammaSet`, the model answers with `Effect::SetGamma`, and
//! `casement::exec` hands it to the backend; destroying the control restores the identity
//! curve.
//!
//! One control per output: a second `get_gamma_control` on a controlled output, or on one
//! that is gone or cannot take a ramp, gets an inert control that says `failed` at once. The
//! table size is the backend's ([`GammaHandler::gamma_table_size`]); a ramp of any other
//! length is `invalid_gamma`, and a ramp is raised to at least an eighth of the identity curve
//! (`GammaTable::floored`, so a client cannot black the screen out). Hidden from sandboxed clients (I7).
//!
//! Changing the displays is a power (`Power::Displays`): a control asked for while the person has
//! not answered is made but inert (no `gamma_size`, and its ramps change nothing) until they
//! allow; refused, it says `failed`. A power taken back fails the controls in use and restores
//! the identity curve.

use crate::client_file;
use crate::consent::{self, Admission, ConsentHandler, Held, Mortal, Pushed};
use crate::states::{GlobalFilter, WaylandFault};
use casement_core::output::{GammaRamp, GammaTable};
use casement_core::policy::GlobalKind;
use casement_core::privilege::PowerOutcome;
use casement_core::{ClientKey, Event, OutputBook, OutputKey};
use casement_protocols::vocab::Power;
use smithay::reexports::wayland_protocols_wlr::gamma_control::v1::server::{
    zwlr_gamma_control_manager_v1::{self, ZwlrGammaControlManagerV1},
    zwlr_gamma_control_v1::{self, ZwlrGammaControlV1},
};
use smithay::reexports::wayland_server::backend::{ClientId, GlobalId};
use smithay::reexports::wayland_server::protocol::wl_output::WlOutput;
use smithay::reexports::wayland_server::{
    Client, DataInit, Dispatch, DisplayHandle, GlobalDispatch, New, Resource,
};
use smithay::wayland::{Dispatch2, GlobalDispatch2};
use std::collections::BTreeMap;
use std::fs::File;
use std::os::fd::OwnedFd;
use std::os::unix::fs::FileExt;

/// The version casement serves.
pub const VERSION: u32 = 1;

/// How many steps per channel an output's gamma table has (`gamma_size`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct GammaSize(pub u32);

/// `zwlr_gamma_control_manager_v1` and the outputs under a client's control.
#[derive(Debug, Default)]
pub struct GammaPart {
    /// Each controlled output and the table size it announced.
    pub controlled: BTreeMap<OutputKey, GammaSize>,
    /// Our global (none until [`GammaPart::new`]).
    pub global: Option<GlobalId>,
    objects: BTreeMap<OutputKey, ZwlrGammaControlV1>,
    /// The controls made while the person had not answered, inert until they do.
    waiting: Held<WaitingControl>,
}

/// A control that waits for the person's answer about the displays.
#[derive(Debug)]
pub struct WaitingControl {
    control: ZwlrGammaControlV1,
    output: OutputKey,
}

impl Mortal for WaitingControl {
    fn alive(&self) -> bool {
        self.control.is_alive()
    }
}

impl GammaPart {
    /// Creates the global (version 1), filtered by the I7 table.
    pub fn new<D>(display: &DisplayHandle, role_filter: GlobalFilter) -> GammaPart
    where
        D: GlobalDispatch<ZwlrGammaControlManagerV1, GammaGlobal> + 'static,
    {
        GammaPart {
            global: Some(display.create_global::<D, ZwlrGammaControlManagerV1, _>(
                VERSION,
                GammaGlobal {
                    filter: role_filter,
                },
            )),
            ..GammaPart::default()
        }
    }
}

/// What the composition root provides to this protocol's dispatch.
pub trait GammaHandler: ConsentHandler + Sized {
    /// The part.
    fn gamma_part(&mut self) -> &mut GammaPart;
    /// The output a client's `wl_output` names.
    fn gamma_output(&self, output: &WlOutput) -> Option<OutputKey>;
    /// The table size the backend drives `output` with; none when it cannot take a ramp.
    fn gamma_table_size(&self, output: &OutputKey) -> Option<GammaSize>;
    /// Feeds the events a request produced into the model (`core_loop::feed`).
    fn gamma_events(&mut self, events: Vec<Event>);
}

/// The global's data: the filter its `can_view` asks.
#[derive(Debug, Clone, Copy)]
pub struct GammaGlobal {
    filter: GlobalFilter,
}

/// A bound manager's data.
#[derive(Debug, Clone, Copy, Default)]
pub struct GammaManagerData;

/// One control's data: the output it holds, none for an inert (failed) control.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlData {
    output: Option<OutputKey>,
}

/// A client's `set_gamma` on `output`: one `Event::GammaSet`.
///
/// ```no_run
/// use casement_core::output::GammaRamp;
/// use casement_wayland::gamma::{GammaPart, on_set_gamma};
///
/// # fn demo(key: casement_core::OutputKey) {
/// let mut part = GammaPart::default();
/// let events = on_set_gamma(&mut part, key, GammaRamp::Identity);
/// assert_eq!(events.len(), 1);
/// # }
/// ```
pub fn on_set_gamma(part: &mut GammaPart, output: OutputKey, ramp: GammaRamp) -> Vec<Event> {
    let _ = part;
    vec![Event::GammaSet { output, ramp }]
}

/// The client destroyed its control (or went away): the identity curve is restored.
pub fn on_released(part: &mut GammaPart, output: &OutputKey) -> Vec<Event> {
    part.objects.remove(output);
    match part.controlled.remove(output) {
        Some(_) => vec![Event::GammaSet {
            output: output.clone(),
            ramp: GammaRamp::Identity,
        }],
        None => Vec::new(),
    }
}

/// Announces `output`'s table size to the control (`gamma_size`) and records the control as
/// holding it.
pub fn apply_size(
    part: &mut GammaPart,
    output: &OutputKey,
    size: GammaSize,
) -> Result<(), WaylandFault> {
    let control = part.objects.get(output).ok_or(WaylandFault::SurfaceGone)?;
    control.gamma_size(size.0);
    part.controlled.insert(output.clone(), size);
    Ok(())
}

/// Tells the controls of outputs that are no longer plugged `failed` and lets them go.
///
/// ```no_run
/// use casement_core::OutputBook;
/// use casement_wayland::gamma::{GammaPart, follow};
///
/// let mut part = GammaPart::default();
/// follow(&mut part, &OutputBook::default());
/// assert!(part.controlled.is_empty());
/// ```
pub fn follow(part: &mut GammaPart, outputs: &OutputBook) {
    let gone: Vec<OutputKey> = part
        .objects
        .keys()
        .filter(|key| !outputs.0.contains_key(*key))
        .cloned()
        .collect();
    for key in gone {
        part.controlled.remove(&key);
        if let Some(control) = part.objects.remove(&key) {
            control.failed();
        }
    }
}

/// The ramp in `fd` for a table of `size` steps: red, green and blue in turn, 16-bit native
/// endian, the whole file and nothing more. None when the file is another length.
fn read_ramp(fd: OwnedFd, size: GammaSize) -> Option<GammaRamp> {
    let steps = usize::try_from(size.0).ok()?;
    let length = steps.checked_mul(6)?;
    let file = File::from(fd);
    // Only a memory file is read, and it is asked its length only then: a file a hostile
    // process serves would block either on the loop (CASW-09).
    if !client_file::is_memory(&file)
        || file.metadata().ok()?.len() != u64::try_from(length).ok()?
    {
        return None;
    }
    let mut bytes = vec![0; length];
    file.read_exact_at(&mut bytes, 0).ok()?;
    let words: Vec<u16> = bytes
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u16::from_ne_bytes(*pair))
        .collect();
    let (red, rest) = words.split_at(steps);
    let (green, blue) = rest.split_at(steps);
    GammaTable::new(red.to_vec(), green.to_vec(), blue.to_vec())
        .ok()
        .map(|table| GammaRamp::Table(table.floored()))
}

impl<D> GlobalDispatch2<ZwlrGammaControlManagerV1, D> for GammaGlobal
where
    D: Dispatch<ZwlrGammaControlManagerV1, GammaManagerData> + 'static,
{
    fn bind(
        &self,
        _state: &mut D,
        _handle: &DisplayHandle,
        _client: &Client,
        resource: New<ZwlrGammaControlManagerV1>,
        data_init: &mut DataInit<'_, D>,
    ) {
        data_init.init(resource, GammaManagerData);
    }

    fn can_view(&self, client: &Client) -> bool {
        self.filter.sees(GlobalKind::Gamma, client)
    }
}

impl<D> Dispatch2<ZwlrGammaControlManagerV1, D> for GammaManagerData
where
    D: GammaHandler + Dispatch<ZwlrGammaControlV1, ControlData>,
{
    fn request(
        &self,
        state: &mut D,
        client: &Client,
        _resource: &ZwlrGammaControlManagerV1,
        request: zwlr_gamma_control_manager_v1::Request,
        _handle: &DisplayHandle,
        data_init: &mut DataInit<'_, D>,
    ) {
        let zwlr_gamma_control_manager_v1::Request::GetGammaControl { id, output } = request else {
            return;
        };
        let granted = state.gamma_output(&output).and_then(|key| {
            let free = !state
                .gamma_part()
                .objects
                .get(&key)
                .is_some_and(|control| control.is_alive());
            let size = state.gamma_table_size(&key);
            size.filter(|_| free).map(|size| (key, size))
        });
        let Some((key, size)) = granted else {
            data_init.init(id, ControlData { output: None }).failed();
            return;
        };
        let control = data_init.init(
            id,
            ControlData {
                output: Some(key.clone()),
            },
        );
        match consent::admit(state, client, Power::Displays) {
            Admission::Go => register(state.gamma_part(), control, &key, size),
            Admission::Hold => wait(state, client, control, key),
            Admission::Refuse => control.failed(),
        }
    }
}

/// Makes `control` the one that holds `output`, and tells it the table size.
fn register(
    part: &mut GammaPart,
    control: ZwlrGammaControlV1,
    output: &OutputKey,
    size: GammaSize,
) {
    part.objects.insert(output.clone(), control);
    if let Err(fault) = apply_size(part, output, size) {
        tracing::warn!(%fault, "a gamma control lost its output at once");
    }
}

/// Keeps `control` inert until the answer comes; one past the cap of waiting requests, and one
/// from a client nobody vouches for, fails.
fn wait<D: GammaHandler>(
    state: &mut D,
    client: &Client,
    control: ZwlrGammaControlV1,
    output: OutputKey,
) {
    let kept = WaitingControl { control, output };
    let refused = match consent::key_of(client) {
        Some(key) => match state.gamma_part().waiting.push(key, kept) {
            Pushed::Kept => return,
            Pushed::Full(full) => full,
        },
        None => kept,
    };
    refused.control.failed();
}

/// The answer about `client`'s power over the displays came out. Allowed, the controls that
/// waited take their outputs (one whose output cannot take a ramp any more, or is held by
/// another control by now, fails); refused, they fail; taken back, the controls in use fail
/// too and the identity curve is restored.
pub fn on_resolved<D: GammaHandler>(state: &mut D, client: ClientKey, outcome: PowerOutcome) {
    let waiting = state.gamma_part().waiting.take(client);
    match outcome {
        PowerOutcome::Allowed => {
            for held in waiting {
                let size = state.gamma_table_size(&held.output);
                let part = state.gamma_part();
                let free = !part
                    .objects
                    .get(&held.output)
                    .is_some_and(|control| control.is_alive());
                match size.filter(|_| free) {
                    Some(size) => register(part, held.control, &held.output, size),
                    None => held.control.failed(),
                }
            }
        }
        PowerOutcome::Denied | PowerOutcome::Revoked => {
            waiting.iter().for_each(|held| held.control.failed());
            let part = state.gamma_part();
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
                state.gamma_events(events);
            }
        }
    }
}

impl<D> Dispatch2<ZwlrGammaControlV1, D> for ControlData
where
    D: GammaHandler,
{
    fn request(
        &self,
        state: &mut D,
        _client: &Client,
        resource: &ZwlrGammaControlV1,
        request: zwlr_gamma_control_v1::Request,
        _handle: &DisplayHandle,
        _data_init: &mut DataInit<'_, D>,
    ) {
        let zwlr_gamma_control_v1::Request::SetGamma { fd } = request else {
            return;
        };
        let Some(output) = &self.output else {
            return;
        };
        // Only the control that holds the output acts: one that waits for the person, or was
        // failed, changes nothing.
        if state.gamma_part().objects.get(output) != Some(resource) {
            return;
        }
        let Some(size) = state.gamma_part().controlled.get(output).copied() else {
            return;
        };
        match read_ramp(fd, size) {
            Some(ramp) => {
                let events = on_set_gamma(state.gamma_part(), output.clone(), ramp);
                state.gamma_events(events);
            }
            None => resource.post_error(
                zwlr_gamma_control_v1::Error::InvalidGamma,
                "the ramp must hold red, green and blue of gamma_size 16-bit steps each",
            ),
        }
    }

    fn destroyed(&self, state: &mut D, _client: ClientId, resource: &ZwlrGammaControlV1) {
        let Some(output) = &self.output else {
            return;
        };
        let part = state.gamma_part();
        if part.objects.get(output) != Some(resource) {
            return;
        }
        let events = on_released(part, output);
        state.gamma_events(events);
    }
}
