//! The fake udev: a scripted stream of device events (hotplug reconciliation, P1-D1), as
//! `casement_backend::tty::DeviceSource`.

use casement_backend::tty::{DeviceEvent, DevicePath, DeviceSource};
use std::collections::VecDeque;

/// A scripted udev.
#[derive(Debug, Default)]
pub struct FakeUdev {
    present: Vec<DevicePath>,
    script: VecDeque<DeviceEvent>,
}

impl FakeUdev {
    /// A udev that will report `script` in order.
    pub fn scripted(script: Vec<DeviceEvent>) -> FakeUdev {
        FakeUdev {
            present: Vec::new(),
            script: script.into(),
        }
    }

    /// The same, with `present` devices there from the start.
    #[must_use]
    pub fn with_present(self, present: Vec<DevicePath>) -> FakeUdev {
        FakeUdev { present, ..self }
    }

    /// Queues one more event.
    pub fn push(&mut self, event: DeviceEvent) {
        self.script.push_back(event);
    }
}

impl DeviceSource for FakeUdev {
    fn present(&self) -> Vec<DevicePath> {
        self.present.clone()
    }

    fn next_event(&mut self) -> Option<DeviceEvent> {
        self.script.pop_front()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn the_script_comes_out_in_order_then_ends() {
        let card = DevicePath(PathBuf::from("/dev/dri/card0"));
        let mut udev = FakeUdev::scripted(vec![
            DeviceEvent::Added(card.clone()),
            DeviceEvent::Changed(card.clone()),
            DeviceEvent::Removed(card.clone()),
        ]);
        assert_eq!(udev.next_event(), Some(DeviceEvent::Added(card.clone())));
        assert_eq!(udev.next_event(), Some(DeviceEvent::Changed(card.clone())));
        assert_eq!(udev.next_event(), Some(DeviceEvent::Removed(card)));
        assert_eq!(udev.next_event(), None);
    }
}
