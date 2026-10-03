//! The fake udev: a scripted stream of device events (hotplug reconciliation, P1).

use std::collections::VecDeque;
use std::path::PathBuf;

/// One scripted device event (P1-Z's `DeviceEvent` in the backend replaces it).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UdevEvent {
    /// A device appeared.
    Added(PathBuf),
    /// A device changed (a connector's hotplug).
    Changed(PathBuf),
    /// A device went away.
    Removed(PathBuf),
}

/// A scripted udev.
#[derive(Debug, Default)]
pub struct FakeUdev {
    #[expect(dead_code, reason = "P0-B-headless: the scripted events")]
    script: VecDeque<UdevEvent>,
}

impl FakeUdev {
    /// A udev that will report `script` in order.
    pub fn scripted(script: Vec<UdevEvent>) -> FakeUdev {
        FakeUdev {
            script: script.into(),
        }
    }

    /// The next scripted event, if any.
    pub fn next_event(&mut self) -> Option<UdevEvent> {
        todo!("P0-B-headless: FakeUdev::next_event")
    }
}
