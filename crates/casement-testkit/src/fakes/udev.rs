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
        self.script.pop_front()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_script_comes_out_in_order_then_ends() {
        let card = PathBuf::from("/dev/dri/card0");
        let mut udev = FakeUdev::scripted(vec![
            UdevEvent::Added(card.clone()),
            UdevEvent::Changed(card.clone()),
            UdevEvent::Removed(card.clone()),
        ]);
        assert_eq!(udev.next_event(), Some(UdevEvent::Added(card.clone())));
        assert_eq!(udev.next_event(), Some(UdevEvent::Changed(card.clone())));
        assert_eq!(udev.next_event(), Some(UdevEvent::Removed(card)));
        assert_eq!(udev.next_event(), None);
    }
}
