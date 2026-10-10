//! Hotplug reconciliation (P1-Z; filled by P1-D2): from the connectors a device reports now to
//! the plug, change and unplug events the model hears, with no repeated events and none lost
//! across a hundred plug cycles.

use super::connector::output_facts;
use super::kms::{ConnectorId, ConnectorStatus, KmsResources};
use crate::headless::ConnectorEvent;
use casement_core::output::OutputFacts;
use std::collections::BTreeMap;

/// The connectors that were plugged when we last looked.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct KnownConnectors(pub BTreeMap<ConnectorId, OutputFacts>);

/// The events that take `known` to what `now` shows, and the connectors known after them.
/// Unplugs come first, then plugs and changes in connector order. A connector whose display was
/// swapped for another between two looks (a different key) is an unplug and a plug, never a
/// change: the model files settings by key.
///
/// ```
/// use casement_backend::tty::{KnownConnectors, KmsResources, reconcile};
///
/// let (known, events) = reconcile(&KnownConnectors::default(), &KmsResources::default());
/// assert!(known.0.is_empty() && events.is_empty());
/// ```
pub fn reconcile(
    known: &KnownConnectors,
    now: &KmsResources,
) -> (KnownConnectors, Vec<ConnectorEvent>) {
    let attached: BTreeMap<ConnectorId, OutputFacts> = now
        .connectors
        .iter()
        .filter(|c| c.status == ConnectorStatus::Connected)
        .map(|c| (c.id, output_facts(c)))
        .collect();
    let gone = known
        .0
        .iter()
        .filter(|(id, old)| attached.get(id).is_none_or(|new| new.key != old.key));
    let mut events: Vec<ConnectorEvent> = gone
        .map(|(_, old)| ConnectorEvent::Unplugged(old.key.clone()))
        .collect();
    for (id, new) in &attached {
        match known.0.get(id) {
            Some(old) if old.key == new.key && old == new => {}
            Some(old) if old.key == new.key => events.push(ConnectorEvent::Changed(new.clone())),
            _ => events.push(ConnectorEvent::Plugged(new.clone())),
        }
    }
    (KnownConnectors(attached), events)
}
