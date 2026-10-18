//! The configuration a client builds: heads enabled or disabled, each with the properties it
//! sets, until `apply` or `test` concludes it. The draft lives in the objects' user data; the
//! answer is [`on_configuration`]'s.

use super::heads::{HeadData, ModeData};
use super::wire::transform_from;
use super::{
    ConfigSerial, Configuration, ConfigurationAnswer, ConfigurationIntent, HeadRequest,
    OutputManagementHandler, on_configuration,
};
use crate::consent::{self, Admission, Mortal, Pushed};
use casement_core::geometry::{LogicalPoint, PhysicalSize};
use casement_core::output::{Mode, ModePreference, OutputChangeSet, OutputFacts, OutputPower};
use casement_core::privilege::PowerOutcome;
use casement_core::time::Millihertz;
use casement_core::{ClientKey, OutputKey};
use casement_protocols::vocab::Power;
use ds_core::geometry::scale::Scale;
use smithay::reexports::wayland_protocols_wlr::output_management::v1::server::{
    zwlr_output_configuration_head_v1::{self, ZwlrOutputConfigurationHeadV1},
    zwlr_output_configuration_v1::{self, ZwlrOutputConfigurationV1},
    zwlr_output_head_v1::ZwlrOutputHeadV1,
};
use smithay::reexports::wayland_server::{Client, DataInit, Dispatch, DisplayHandle, Resource};
use smithay::wayland::Dispatch2;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

/// An `apply` or a `test` that waits for the person's answer about the displays
/// (`Power::Displays`).
#[derive(Debug)]
pub struct HeldConclusion {
    resource: ZwlrOutputConfigurationV1,
    intent: ConfigurationIntent,
    configuration: Configuration,
}

impl Mortal for HeldConclusion {
    fn alive(&self) -> bool {
        self.resource.is_alive()
    }
}

/// Whether `apply` or `test` has been sent: a configuration is used once.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum Concluded {
    #[default]
    Not,
    Yes,
}

/// One head in the draft and what the client set on it.
#[derive(Debug, Clone)]
struct DraftHead {
    output: OutputKey,
    change: OutputChangeSet,
}

/// What the client has built so far.
#[derive(Debug, Default)]
struct Draft {
    heads: Vec<DraftHead>,
    concluded: Concluded,
}

/// A configuration's data: the serial it was built at and the draft.
#[derive(Debug)]
pub struct ConfigData {
    serial: ConfigSerial,
    draft: Arc<Mutex<Draft>>,
}

impl ConfigData {
    pub(super) fn new(serial: ConfigSerial) -> ConfigData {
        ConfigData {
            serial,
            draft: Arc::new(Mutex::new(Draft::default())),
        }
    }
}

/// A configuration head's data: the draft it belongs to and the output it configures.
#[derive(Debug)]
pub struct ConfigHeadData {
    draft: Arc<Mutex<Draft>>,
    output: Option<OutputKey>,
}

/// A poisoned lock is still a lock: the draft is plain data.
fn locked(draft: &Mutex<Draft>) -> MutexGuard<'_, Draft> {
    draft.lock().unwrap_or_else(PoisonError::into_inner)
}

/// A property that was already set once.
struct AlreadySet;

/// Sets `slot` to `value` unless the client set it before.
fn claim<T>(slot: &mut Option<T>, value: T) -> Result<(), AlreadySet> {
    match slot {
        Some(_) => Err(AlreadySet),
        None => {
            *slot = Some(value);
            Ok(())
        }
    }
}

/// The mode a custom size and rate name: one the display offers when it has one (the fastest
/// when no rate is given), else the mode as asked, which `apply` and `test` then refuse.
fn custom_mode(facts: Option<&OutputFacts>, size: PhysicalSize, refresh: u32) -> Mode {
    let offered = facts
        .into_iter()
        .flat_map(|facts| facts.modes.iter())
        .filter(|mode| mode.size == size && (refresh == 0 || mode.refresh.0 == refresh))
        .max_by_key(|mode| mode.refresh.0);
    offered.copied().unwrap_or(Mode {
        size,
        refresh: Millihertz(refresh),
        preference: ModePreference::Other,
    })
}

impl ConfigData {
    /// Adds `head` to the draft, enabled or disabled; a head twice is an error.
    fn add(
        &self,
        resource: &ZwlrOutputConfigurationV1,
        head: &ZwlrOutputHeadV1,
        power: OutputPower,
    ) {
        let Some(output) = head.data::<HeadData>().map(|data| data.output.clone()) else {
            return;
        };
        let mut draft = locked(&self.draft);
        if draft.concluded == Concluded::Yes {
            resource.post_error(
                zwlr_output_configuration_v1::Error::AlreadyUsed,
                "the configuration was already applied or tested",
            );
            return;
        }
        if draft.heads.iter().any(|known| known.output == output) {
            resource.post_error(
                zwlr_output_configuration_v1::Error::AlreadyConfiguredHead,
                "the head was already enabled or disabled",
            );
            return;
        }
        draft.heads.push(DraftHead {
            output,
            change: OutputChangeSet::none().with_power(power),
        });
    }

    /// `apply` or `test`: every head must be configured; the answer goes back on `resource`,
    /// once the person has allowed `client` to change the displays.
    fn conclude<D: OutputManagementHandler>(
        &self,
        state: &mut D,
        client: &Client,
        resource: &ZwlrOutputConfigurationV1,
        intent: ConfigurationIntent,
    ) {
        let heads = {
            let mut draft = locked(&self.draft);
            if draft.concluded == Concluded::Yes {
                resource.post_error(
                    zwlr_output_configuration_v1::Error::AlreadyUsed,
                    "the configuration was already applied or tested",
                );
                return;
            }
            draft.concluded = Concluded::Yes;
            draft.heads.clone()
        };
        let book = state.output_management_outputs().clone();
        let unconfigured = book
            .0
            .keys()
            .any(|key| heads.iter().all(|head| head.output != *key));
        if unconfigured {
            resource.post_error(
                zwlr_output_configuration_v1::Error::UnconfiguredHead,
                "every head must be enabled or disabled before apply or test",
            );
            return;
        }
        let requests = heads
            .into_iter()
            .map(|head| HeadRequest::new(head.output, head.change))
            .collect();
        let configuration =
            Configuration::new(self.serial, requests, consent::requester_of(client));
        match consent::admit(state, client, Power::Displays) {
            Admission::Go => settle(state, resource, intent, &configuration),
            Admission::Hold => hold(state, client, resource, intent, configuration),
            Admission::Refuse => resource.failed(),
        }
    }
}

/// Checks `configuration` against the heads as they are now and answers on `resource`.
fn settle<D: OutputManagementHandler>(
    state: &mut D,
    resource: &ZwlrOutputConfigurationV1,
    intent: ConfigurationIntent,
    configuration: &Configuration,
) {
    let book = state.output_management_outputs().clone();
    let answer = on_configuration(state.output_management_part(), &book, intent, configuration);
    match answer {
        ConfigurationAnswer::Succeeded(events) => {
            state.feed(events);
            resource.succeeded();
        }
        ConfigurationAnswer::Failed => resource.failed(),
        ConfigurationAnswer::Cancelled => resource.cancelled(),
    }
}

/// Keeps the conclusion until the answer comes; one past the cap of waiting requests, and one
/// from a client nobody vouches for, fails.
fn hold<D: OutputManagementHandler>(
    state: &mut D,
    client: &Client,
    resource: &ZwlrOutputConfigurationV1,
    intent: ConfigurationIntent,
    configuration: Configuration,
) {
    let kept = HeldConclusion {
        resource: resource.clone(),
        intent,
        configuration,
    };
    let refused = match consent::key_of(client) {
        Some(key) => matches!(
            state.output_management_part().gated.push(key, kept),
            Pushed::Full(_)
        ),
        None => true,
    };
    if refused {
        resource.failed();
    }
}

/// The answer about `client`'s power over the displays came out: the conclusions that waited
/// are checked and answered now (against the heads as they are then, so a stale serial is
/// `cancelled`), or `failed`.
pub fn on_resolved<D: OutputManagementHandler>(
    state: &mut D,
    client: ClientKey,
    outcome: PowerOutcome,
) {
    for held in state.output_management_part().gated.take(client) {
        match outcome {
            PowerOutcome::Allowed => {
                settle(state, &held.resource, held.intent, &held.configuration);
            }
            PowerOutcome::Denied | PowerOutcome::Revoked => held.resource.failed(),
        }
    }
}

impl<D> Dispatch2<ZwlrOutputConfigurationV1, D> for ConfigData
where
    D: OutputManagementHandler + Dispatch<ZwlrOutputConfigurationHeadV1, ConfigHeadData>,
{
    fn request(
        &self,
        state: &mut D,
        client: &Client,
        resource: &ZwlrOutputConfigurationV1,
        request: zwlr_output_configuration_v1::Request,
        _handle: &DisplayHandle,
        data_init: &mut DataInit<'_, D>,
    ) {
        match request {
            zwlr_output_configuration_v1::Request::EnableHead { id, head } => {
                let output = head.data::<HeadData>().map(|data| data.output.clone());
                data_init.init(
                    id,
                    ConfigHeadData {
                        draft: Arc::clone(&self.draft),
                        output,
                    },
                );
                self.add(resource, &head, OutputPower::On);
            }
            zwlr_output_configuration_v1::Request::DisableHead { head } => {
                self.add(resource, &head, OutputPower::Off);
            }
            zwlr_output_configuration_v1::Request::Apply => {
                self.conclude(state, client, resource, ConfigurationIntent::Apply);
            }
            zwlr_output_configuration_v1::Request::Test => {
                self.conclude(state, client, resource, ConfigurationIntent::Test);
            }
            _ => {}
        }
    }
}

impl<D> Dispatch2<ZwlrOutputConfigurationHeadV1, D> for ConfigHeadData
where
    D: OutputManagementHandler,
{
    fn request(
        &self,
        state: &mut D,
        _client: &Client,
        resource: &ZwlrOutputConfigurationHeadV1,
        request: zwlr_output_configuration_head_v1::Request,
        _handle: &DisplayHandle,
        _data_init: &mut DataInit<'_, D>,
    ) {
        use zwlr_output_configuration_head_v1::{Error, Request};
        let Some(output) = &self.output else {
            return;
        };
        let mut draft = locked(&self.draft);
        let Some(head) = draft.heads.iter_mut().find(|head| head.output == *output) else {
            return;
        };
        let set = match request {
            Request::SetMode { mode } => match mode.data::<ModeData>() {
                Some(data) if data.output == *output => claim(&mut head.change.mode, data.mode),
                _ => {
                    resource.post_error(Error::InvalidMode, "the mode belongs to another head");
                    return;
                }
            },
            Request::SetCustomMode {
                width,
                height,
                refresh,
            } => {
                let (Ok(w), Ok(h), Ok(refresh)) = (
                    u32::try_from(width),
                    u32::try_from(height),
                    u32::try_from(refresh),
                ) else {
                    resource.post_error(Error::InvalidCustomMode, "the custom mode is negative");
                    return;
                };
                if w == 0 || h == 0 {
                    resource.post_error(Error::InvalidCustomMode, "the custom mode is empty");
                    return;
                }
                let facts = state
                    .output_management_outputs()
                    .0
                    .get(output)
                    .map(|plugged| &plugged.facts);
                let mode = custom_mode(facts, PhysicalSize { w, h }, refresh);
                claim(&mut head.change.mode, mode)
            }
            Request::SetPosition { x, y } => {
                claim(&mut head.change.position, LogicalPoint { x, y })
            }
            Request::SetTransform { transform } => {
                match transform.into_result().ok().and_then(transform_from) {
                    Some(transform) => claim(&mut head.change.transform, transform),
                    None => {
                        resource.post_error(Error::InvalidTransform, "no such transform");
                        return;
                    }
                }
            }
            Request::SetScale { scale } => match Scale::from_factor(scale) {
                Some(scale) => claim(&mut head.change.scale, scale),
                None => {
                    resource.post_error(Error::InvalidScale, "the scale must be positive");
                    return;
                }
            },
            // Adaptive sync is not offered: a request to change it changes nothing.
            _ => return,
        };
        if set.is_err() {
            resource.post_error(Error::AlreadySet, "the property was already set");
        }
    }
}
