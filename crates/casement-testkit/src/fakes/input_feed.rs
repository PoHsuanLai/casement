//! The fake seat input (P1-T): `FakeInputFeed` stands in for libinput on a tty seat. A test
//! plugs devices and pushes events; the compositor polls them through the same `InputFeed` seam
//! the real adapter fills, and the settings it applies are recorded (a shared
//! [`FakeInputDevices`]). A clone shares everything.

use super::FakeInputDevices;
use casement_backend::input::{
    Applied, DeviceName, DeviceSettings, FedDevice, FedInput, InputFeed, PollContext,
};
use casement_core::DeviceId;
use casement_core::input::{DeviceClass, InputEvent, InputKind, Origin};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

/// Whether the seat's input is being read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum Reading {
    #[default]
    Live,
    Suspended,
}

#[derive(Debug, Default)]
struct FeedWorld {
    devices: BTreeMap<DeviceId, DeviceClass>,
    queue: Vec<Queued>,
    reading: Reading,
    suspends: u32,
    resumes: u32,
}

/// What a test pushed, until the next poll turns it into a [`FedInput`].
#[derive(Debug)]
enum Queued {
    Appeared(FedDevice),
    Gone(DeviceId),
    Event(DeviceId, InputKind),
}

/// A fake libinput.
///
/// ```
/// use casement_testkit::fakes::FakeInputFeed;
///
/// let feed = FakeInputFeed::new();
/// assert_eq!(feed.suspends(), 0);
/// ```
#[derive(Debug, Clone, Default)]
pub struct FakeInputFeed {
    configured: FakeInputDevices,
    world: Arc<Mutex<FeedWorld>>,
}

impl FakeInputFeed {
    /// A feed with no device.
    pub fn new() -> FakeInputFeed {
        FakeInputFeed::default()
    }

    fn world(&self) -> MutexGuard<'_, FeedWorld> {
        self.world.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// A device appears (reported at the next poll).
    pub fn plug(&self, id: DeviceId, class: DeviceClass, name: &str) {
        let mut world = self.world();
        world.devices.insert(id, class);
        world.queue.push(Queued::Appeared(FedDevice::new(
            id,
            class,
            DeviceName(name.to_owned()),
        )));
    }

    /// A device goes away (reported at the next poll).
    pub fn unplug(&self, id: DeviceId) {
        let mut world = self.world();
        world.devices.remove(&id);
        world.queue.push(Queued::Gone(id));
    }

    /// `device` produces `kind`. Dropped while the seat is lent, as a key pressed on another VT
    /// is never read here.
    pub fn push(&self, device: DeviceId, kind: InputKind) {
        let mut world = self.world();
        if world.reading == Reading::Live {
            world.queue.push(Queued::Event(device, kind));
        }
    }

    /// The settings applied to devices so far, in order.
    pub fn applied(&self) -> Vec<(DeviceId, DeviceSettings)> {
        self.configured.applied()
    }

    /// How many times reading was suspended.
    pub fn suspends(&self) -> u32 {
        self.world().suspends
    }

    /// How many times reading resumed.
    pub fn resumes(&self) -> u32 {
        self.world().resumes
    }
}

impl InputFeed for FakeInputFeed {
    fn apply(&mut self, device: DeviceId, settings: &DeviceSettings) -> Applied {
        let present = self.world().devices.contains_key(&device);
        match present {
            true => self.configured.apply(device, settings),
            false => Applied::Refused,
        }
    }

    fn poll(&mut self, context: &PollContext) -> Vec<FedInput> {
        let mut world = self.world();
        if world.reading == Reading::Suspended {
            return Vec::new();
        }
        let queued = std::mem::take(&mut world.queue);
        queued
            .into_iter()
            .filter_map(|item| match item {
                Queued::Appeared(device) => Some(FedInput::Appeared(device)),
                Queued::Gone(id) => Some(FedInput::Gone(id)),
                Queued::Event(device, kind) => {
                    let class = *world.devices.get(&device)?;
                    Some(FedInput::Event(InputEvent {
                        origin: Origin::Physical(class),
                        device,
                        at: context.now,
                        kind,
                    }))
                }
            })
            .collect()
    }

    fn suspend(&mut self) {
        let mut world = self.world();
        world.reading = Reading::Suspended;
        world.suspends += 1;
        world.queue.clear();
    }

    fn resume(&mut self) {
        let mut world = self.world();
        world.reading = Reading::Live;
        world.resumes += 1;
    }
}
