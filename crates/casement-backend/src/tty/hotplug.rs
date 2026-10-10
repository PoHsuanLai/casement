//! Hotplug reconciliation (P1-Z; filled by P1-D2): from the connectors a device reports now to
//! the plug, change and unplug events the model hears, with no repeated events and none lost
//! across a hundred plug cycles.

use super::kms::{ConnectorId, KmsResources};
use crate::headless::ConnectorEvent;
use casement_core::output::OutputFacts;
use std::collections::BTreeMap;

/// The connectors that were plugged when we last looked.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct KnownConnectors(pub BTreeMap<ConnectorId, OutputFacts>);

/// The events that take `known` to what `now` shows, and the connectors known after them.
pub fn reconcile(
    known: &KnownConnectors,
    now: &KmsResources,
) -> (KnownConnectors, Vec<ConnectorEvent>) {
    let _ = (known, now);
    todo!("P1-D2: reconcile")
}
