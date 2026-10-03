//! Plugging and unplugging virtual outputs: the same `ConnectorEvent` path KMS hotplug takes.

use crate::fixture::Fixture;
use casement_backend::VirtualOutputSpec;
use casement_core::OutputName;

impl Fixture {
    /// Plugs an output and pumps; returns its connector name.
    pub fn plug(&mut self, spec: VirtualOutputSpec) -> OutputName {
        let _ = spec;
        todo!("P0-B-headless: Fixture::plug")
    }

    /// Unplugs `name` and pumps.
    pub fn unplug(&mut self, name: &OutputName) {
        let _ = name;
        todo!("P0-B-headless: Fixture::unplug")
    }
}
