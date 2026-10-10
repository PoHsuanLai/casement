//! `zwlr_output_manager_v1`: lists the heads (outputs) and their modes and applies or tests a
//! configuration a client built (wlr-randr, sill's displays page).
//!
//! A configuration is a list of per-head changes ([`HeadRequest`]) at a serial the client
//! read; a stale serial is answered `cancelled`. `test` checks every head with
//! `casement_core::output::resolve_change` and changes nothing; `apply` also feeds one
//! `Event::OutputRequested` per head, which the model applies (`Effect::ApplyOutput`). The heads
//! follow the model: [`apply_heads`] runs after every batch of events, announces the new, changed
//! and gone outputs, and sends `done` with a new serial.
//!
//! Disabling a head is `OutputPower::Off`, enabling it `OutputPower::On` (the output keeps its
//! place in the arrangement). Hidden from sandboxed clients (I7).

mod configuration;
mod heads;
mod wire;

pub use configuration::{ConfigData, ConfigHeadData};
pub use heads::{HeadData, ModeData};

use crate::states::{GlobalFilter, WaylandFault};
use casement_core::output::{OutputChangeSet, OutputConfig, OutputFacts, resolve_change};
use casement_core::policy::GlobalKind;
use casement_core::{Event, OutputBook, OutputKey};
use heads::{HeadObjects, create_head};
use smithay::reexports::wayland_protocols_wlr::output_management::v1::server::{
    zwlr_output_configuration_v1::ZwlrOutputConfigurationV1,
    zwlr_output_head_v1::ZwlrOutputHeadV1,
    zwlr_output_manager_v1::{self, ZwlrOutputManagerV1},
    zwlr_output_mode_v1::ZwlrOutputModeV1,
};
use smithay::reexports::wayland_server::backend::{ClientId, GlobalId};
use smithay::reexports::wayland_server::{
    Client, DataInit, Dispatch, DisplayHandle, GlobalDispatch, New, Resource,
};
use smithay::wayland::{Dispatch2, GlobalDispatch2};
use std::collections::{BTreeMap, BTreeSet};

/// The version casement serves.
pub const VERSION: u32 = 4;

/// The manager's serial: it moves whenever the heads change, and a configuration made at an
/// older one is `cancelled`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct ConfigSerial(pub u32);

/// What was announced of one output: compared with the model to see what changed.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Announced {
    facts: OutputFacts,
    config: OutputConfig,
}

/// One bound manager and the heads its client was given.
#[derive(Debug)]
struct ManagerEntry {
    manager: ZwlrOutputManagerV1,
    heads: BTreeMap<OutputKey, HeadObjects>,
}

/// `zwlr_output_manager_v1` and the heads it has announced.
#[derive(Debug, Default)]
pub struct OutputManagementPart {
    /// The outputs announced as heads.
    pub heads: BTreeSet<OutputKey>,
    /// The serial of the last `done`.
    pub serial: ConfigSerial,
    /// Our global (none until [`OutputManagementPart::new`]).
    pub global: Option<GlobalId>,
    managers: Vec<ManagerEntry>,
    known: BTreeMap<OutputKey, Announced>,
}

impl OutputManagementPart {
    /// Creates the global (version 4), filtered by the I7 table.
    pub fn new<D>(display: &DisplayHandle, role_filter: GlobalFilter) -> OutputManagementPart
    where
        D: GlobalDispatch<ZwlrOutputManagerV1, OutputManagementGlobal> + 'static,
    {
        OutputManagementPart {
            global: Some(display.create_global::<D, ZwlrOutputManagerV1, _>(
                VERSION,
                OutputManagementGlobal {
                    filter: role_filter,
                },
            )),
            ..OutputManagementPart::default()
        }
    }

    /// Whether the heads announced are the model's outputs as they are.
    fn is_current(&self, outputs: &OutputBook) -> bool {
        self.known.len() == outputs.0.len()
            && outputs.0.iter().all(|(key, plugged)| {
                self.known.get(key).is_some_and(|announced| {
                    announced.facts == plugged.facts && announced.config == plugged.config
                })
            })
    }
}

/// What the composition root provides to this protocol's dispatch.
pub trait OutputManagementHandler: Sized + 'static {
    /// The part.
    fn output_management_part(&mut self) -> &mut OutputManagementPart;
    /// The model's outputs.
    fn output_management_outputs(&self) -> &OutputBook;
    /// Feeds the events a configuration produced into the model (`core_loop::feed`).
    fn output_management_events(&mut self, events: Vec<Event>);
}

/// The global's data: the filter its `can_view` asks.
#[derive(Debug, Clone, Copy)]
pub struct OutputManagementGlobal {
    filter: GlobalFilter,
}

/// A bound manager's data.
#[derive(Debug, Clone, Copy, Default)]
pub struct ManagerData;

/// Whether a configuration is applied or only checked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ConfigurationIntent {
    /// `apply`.
    Apply,
    /// `test`.
    Test,
}

/// One head's part of a configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct HeadRequest {
    /// The output.
    pub output: OutputKey,
    /// What the client set on it.
    pub change: OutputChangeSet,
}

impl HeadRequest {
    /// A head's request.
    pub fn new(output: OutputKey, change: OutputChangeSet) -> HeadRequest {
        HeadRequest { output, change }
    }
}

/// A configuration a client built.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Configuration {
    /// The manager serial the client built it at.
    pub serial: ConfigSerial,
    /// The heads it enables or changes. (The protocol makes a client configure every head; a
    /// head left out is refused before a configuration is made.)
    pub heads: Vec<HeadRequest>,
}

impl Configuration {
    /// A configuration.
    pub fn new(serial: ConfigSerial, heads: Vec<HeadRequest>) -> Configuration {
        Configuration { serial, heads }
    }
}

/// How the protocol answers a configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ConfigurationAnswer {
    /// `succeeded`, with the events to feed (none for a test).
    Succeeded(Vec<Event>),
    /// `failed`: a head was refused.
    Failed,
    /// `cancelled`: the heads changed since the client read them.
    Cancelled,
}

/// A client's `apply` or `test` of `configuration`, checked against `outputs`.
///
/// ```no_run
/// use casement_core::OutputBook;
/// use casement_wayland::output_management::{
///     ConfigSerial, Configuration, ConfigurationIntent, OutputManagementPart, on_configuration,
/// };
///
/// let mut part = OutputManagementPart::default();
/// let nothing = Configuration::new(ConfigSerial(0), Vec::new());
/// let answer = on_configuration(&mut part, &OutputBook::default(), ConfigurationIntent::Test, &nothing);
/// let _ = answer;
/// ```
pub fn on_configuration(
    part: &mut OutputManagementPart,
    outputs: &OutputBook,
    intent: ConfigurationIntent,
    configuration: &Configuration,
) -> ConfigurationAnswer {
    if configuration.serial != part.serial {
        return ConfigurationAnswer::Cancelled;
    }
    let accepted = configuration.heads.iter().all(|head| {
        outputs.0.get(&head.output).is_some_and(|plugged| {
            resolve_change(&plugged.facts, &plugged.config, &head.change).is_ok()
        })
    });
    match (accepted, intent) {
        (false, _) => ConfigurationAnswer::Failed,
        (true, ConfigurationIntent::Test) => ConfigurationAnswer::Succeeded(Vec::new()),
        (true, ConfigurationIntent::Apply) => ConfigurationAnswer::Succeeded(
            configuration
                .heads
                .iter()
                .map(|head| Event::OutputRequested {
                    output: head.output.clone(),
                    change: head.change,
                })
                .collect(),
        ),
    }
}

/// Announces `outputs` as heads (or their changes) and sends `done` with a new serial. Nothing
/// is sent, and the serial stays, while the heads are already what `outputs` says.
///
/// ```no_run
/// use casement_core::OutputBook;
/// use casement_wayland::output_management::{OutputManagementPart, apply_heads};
/// use smithay::reexports::wayland_server::DisplayHandle;
///
/// # fn demo<D>(display: &DisplayHandle)
/// # where
/// #     D: smithay::reexports::wayland_server::Dispatch<
/// #         smithay::reexports::wayland_protocols_wlr::output_management::v1::server::zwlr_output_head_v1::ZwlrOutputHeadV1,
/// #         casement_wayland::output_management::HeadData,
/// #     > + smithay::reexports::wayland_server::Dispatch<
/// #         smithay::reexports::wayland_protocols_wlr::output_management::v1::server::zwlr_output_mode_v1::ZwlrOutputModeV1,
/// #         casement_wayland::output_management::ModeData,
/// #     > + 'static,
/// # {
/// let mut part = OutputManagementPart::default();
/// apply_heads::<D>(&mut part, display, &OutputBook::default()).expect("nothing to announce");
/// # }
/// ```
pub fn apply_heads<D>(
    part: &mut OutputManagementPart,
    display: &DisplayHandle,
    outputs: &OutputBook,
) -> Result<(), WaylandFault>
where
    D: Dispatch<ZwlrOutputHeadV1, HeadData> + Dispatch<ZwlrOutputModeV1, ModeData> + 'static,
{
    if part.is_current(outputs) {
        return Ok(());
    }
    part.serial = ConfigSerial(part.serial.0.wrapping_add(1));
    part.managers.retain(|entry| entry.manager.is_alive());
    let serial = part.serial;
    let mut outcome = Ok(());
    for entry in &mut part.managers {
        let result = match display.get_client(entry.manager.id()) {
            Ok(client) => sync_manager::<D>(display, &client, entry, outputs, serial),
            Err(_) => Err(WaylandFault::SurfaceGone),
        };
        outcome = outcome.and(result);
    }
    part.known = outputs
        .0
        .iter()
        .map(|(key, plugged)| {
            let announced = Announced {
                facts: plugged.facts.clone(),
                config: plugged.config,
            };
            (key.clone(), announced)
        })
        .collect();
    part.heads = part.known.keys().cloned().collect();
    outcome
}

/// Brings one manager's client up to date with `outputs`: heads that are gone or whose facts
/// changed are finished (a changed one is announced again), new ones announced, the others told
/// what moved; then `done`.
fn sync_manager<D>(
    display: &DisplayHandle,
    client: &Client,
    entry: &mut ManagerEntry,
    outputs: &OutputBook,
    serial: ConfigSerial,
) -> Result<(), WaylandFault>
where
    D: Dispatch<ZwlrOutputHeadV1, HeadData> + Dispatch<ZwlrOutputModeV1, ModeData> + 'static,
{
    let stale: Vec<OutputKey> = entry
        .heads
        .iter()
        .filter(|(key, objects)| {
            outputs
                .0
                .get(*key)
                .is_none_or(|plugged| plugged.facts != objects.facts)
        })
        .map(|(key, _)| key.clone())
        .collect();
    for key in stale {
        if let Some(objects) = entry.heads.remove(&key) {
            objects.finish();
        }
    }
    for (key, plugged) in &outputs.0 {
        match entry.heads.get_mut(key) {
            Some(objects) if objects.config != plugged.config => objects.update(plugged.config),
            Some(_) => {}
            None => {
                let objects = create_head::<D>(display, client, &entry.manager, plugged)?;
                entry.heads.insert(key.clone(), objects);
            }
        }
    }
    entry.manager.done(serial.0);
    Ok(())
}

/// Forgets `manager` (it stopped, or its client went).
fn forget(part: &mut OutputManagementPart, manager: &ZwlrOutputManagerV1) {
    part.managers.retain(|entry| entry.manager != *manager);
}

impl<D> GlobalDispatch2<ZwlrOutputManagerV1, D> for OutputManagementGlobal
where
    D: OutputManagementHandler
        + Dispatch<ZwlrOutputManagerV1, ManagerData>
        + Dispatch<ZwlrOutputHeadV1, HeadData>
        + Dispatch<ZwlrOutputModeV1, ModeData>
        + 'static,
{
    fn bind(
        &self,
        state: &mut D,
        handle: &DisplayHandle,
        client: &Client,
        resource: New<ZwlrOutputManagerV1>,
        data_init: &mut DataInit<'_, D>,
    ) {
        let manager = data_init.init(resource, ManagerData);
        let mut entry = ManagerEntry {
            manager,
            heads: BTreeMap::new(),
        };
        let serial = state.output_management_part().serial;
        let outcome = sync_manager::<D>(
            handle,
            client,
            &mut entry,
            state.output_management_outputs(),
            serial,
        );
        if let Err(fault) = outcome {
            tracing::warn!(%fault, "the heads were not announced to a new manager");
        }
        state.output_management_part().managers.push(entry);
    }

    fn can_view(&self, client: &Client) -> bool {
        self.filter.sees(GlobalKind::OutputManagement, client)
    }
}

impl<D> Dispatch2<ZwlrOutputManagerV1, D> for ManagerData
where
    D: OutputManagementHandler + Dispatch<ZwlrOutputConfigurationV1, ConfigData>,
{
    fn request(
        &self,
        state: &mut D,
        _client: &Client,
        resource: &ZwlrOutputManagerV1,
        request: zwlr_output_manager_v1::Request,
        _handle: &DisplayHandle,
        data_init: &mut DataInit<'_, D>,
    ) {
        match request {
            zwlr_output_manager_v1::Request::CreateConfiguration { id, serial } => {
                data_init.init(id, ConfigData::new(ConfigSerial(serial)));
            }
            zwlr_output_manager_v1::Request::Stop => {
                forget(state.output_management_part(), resource);
                resource.finished();
            }
            _ => {}
        }
    }

    fn destroyed(&self, state: &mut D, _client: ClientId, resource: &ZwlrOutputManagerV1) {
        forget(state.output_management_part(), resource);
    }
}
