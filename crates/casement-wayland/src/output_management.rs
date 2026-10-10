//! `zwlr_output_manager_v1`: lists the heads (outputs) and their modes and applies or tests a
//! configuration a client built (wlr-randr, sill's displays page). P1-Z declares it; P1-M1
//! creates the global in [`OutputManagementPart::new`] and fills the rest. Until then no
//! global is advertised.
//!
//! A configuration is a list of per-head changes ([`HeadRequest`]) at a serial the client
//! read; a stale serial is answered `cancelled`. `test` checks every head with
//! `casement_core::output::resolve_change` and changes nothing; `apply` also feeds one
//! `Event::OutputRequested` per head.

use crate::states::{GlobalFilter, WaylandFault};
use casement_core::output::OutputChangeSet;
use casement_core::{Event, OutputBook, OutputKey};
use smithay::reexports::wayland_server::DisplayHandle;
use std::collections::BTreeSet;

/// The manager's serial: it moves whenever the heads change, and a configuration made at an
/// older one is `cancelled`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct ConfigSerial(pub u32);

/// `zwlr_output_manager_v1` and the heads it has announced.
#[derive(Debug, Default)]
pub struct OutputManagementPart {
    /// The outputs announced as heads.
    pub heads: BTreeSet<OutputKey>,
    /// The serial of the last `done`.
    pub serial: ConfigSerial,
}

impl OutputManagementPart {
    /// Creates the global (version 4), filtered by the I7 table. P1-Z: creates nothing; the
    /// part is declared and no global is advertised.
    pub fn new<D: 'static>(
        display: &DisplayHandle,
        role_filter: GlobalFilter,
    ) -> OutputManagementPart {
        let _ = (display, role_filter, std::marker::PhantomData::<D>);
        OutputManagementPart::default()
    }
}

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
    /// The heads it enables or changes (heads it leaves out are disabled).
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
    let _ = (part, outputs, intent, configuration);
    todo!("P1-M1: output_management::on_configuration")
}

/// Announces `outputs` as heads (or their changes) and sends `done` with a new serial.
pub fn apply_heads(
    part: &mut OutputManagementPart,
    outputs: &OutputBook,
) -> Result<(), WaylandFault> {
    let _ = (part, outputs);
    todo!("P1-M1: output_management::apply_heads")
}
