//! The real udev monitor (feature `tty`): `DeviceSource` over the device crates, a thin
//! adapter (B7).
//!
//! The monitor is an event source in the loop; each event is queued as a [`DeviceEvent`] with
//! the node's path (udev reports a removed device by number only, so the adapter keeps the
//! numbers it has seen) until `next_event` hands the queue out.

use super::udev::{DeviceEvent, DevicePath, DeviceSource};
use smithay::backend::udev::{UdevBackend, UdevEvent};
use smithay::reexports::calloop::LoopHandle;
use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::rc::Rc;

/// The udev monitor on the session's seat.
#[derive(Debug)]
pub struct RealUdev {
    present: Vec<DevicePath>,
    queue: Rc<RefCell<VecDeque<DeviceEvent>>>,
}

impl RealUdev {
    /// Starts watching the DRM devices of `seat`, with `handle`'s loop reading the monitor.
    pub fn watch<D: 'static>(
        seat: &str,
        handle: &LoopHandle<'static, D>,
    ) -> Result<RealUdev, std::io::Error> {
        let backend = UdevBackend::new(seat)?;
        let mut known: HashMap<_, PathBuf> = backend
            .device_list()
            .map(|(id, path)| (id, path.to_path_buf()))
            .collect();
        let mut present: Vec<DevicePath> = known.values().cloned().map(DevicePath).collect();
        present.sort();
        let queue = Rc::new(RefCell::new(VecDeque::new()));
        let events = Rc::clone(&queue);
        handle
            .insert_source(backend, move |event, _, _| {
                let seen = match event {
                    UdevEvent::Added { device_id, path } => {
                        known.insert(device_id, path.clone());
                        Some(DeviceEvent::Added(DevicePath(path)))
                    }
                    UdevEvent::Changed { device_id } => known
                        .get(&device_id)
                        .map(|path| DeviceEvent::Changed(DevicePath(path.clone()))),
                    UdevEvent::Removed { device_id } => known
                        .remove(&device_id)
                        .map(|path| DeviceEvent::Removed(DevicePath(path))),
                };
                if let Some(seen) = seen {
                    events.borrow_mut().push_back(seen);
                }
            })
            .map_err(|error| std::io::Error::other(error.error.to_string()))?;
        Ok(RealUdev { present, queue })
    }
}

impl DeviceSource for RealUdev {
    fn present(&self) -> Vec<DevicePath> {
        self.present.clone()
    }

    fn next_event(&mut self) -> Option<DeviceEvent> {
        self.queue.borrow_mut().pop_front()
    }
}
