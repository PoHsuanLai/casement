//! The input a seat's devices produce (P1-T): the tty backend's counterpart of the headless
//! backend's queue of virtual events. [`InputFeed`] is the one seam for a real seat: it reads
//! what libinput reports, converts it once into our events (device, instant and origin stamped
//! here and never changed downstream, I8) and configures the devices
//! ([`InputFeed::apply`]). Real: `libinput_real.rs` (feature `libinput`), whose
//! devices are opened through the seat's session; fake: `casement_testkit::fakes::FakeInputFeed`.
//!
//! ```no_run
//! use casement_backend::input::{FedInput, InputFeed, PollContext};
//! use casement_core::geometry::{LogicalPoint, LogicalRect, LogicalSize};
//! use casement_core::time::Mono;
//! use std::time::Duration;
//!
//! fn events(feed: &mut dyn InputFeed) -> usize {
//!     let extent = LogicalRect::new(LogicalPoint { x: 0, y: 0 }, LogicalSize { w: 1920, h: 1080 });
//!     let now = PollContext::new(Mono(Duration::ZERO), extent);
//!     feed.poll(&now)
//!         .iter()
//!         .filter(|fed| matches!(fed, FedInput::Event(_)))
//!         .count()
//! }
//! ```

use super::{Applied, DeviceName, DeviceSettings};
use casement_core::DeviceId;
use casement_core::geometry::LogicalRect;
use casement_core::input::{DeviceClass, InputEvent};
use casement_core::time::Mono;

/// A device that appeared.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct FedDevice {
    /// Its id for the rest of the run.
    pub id: DeviceId,
    /// What kind of device.
    pub class: DeviceClass,
    /// Its name, as a device list shows it.
    pub name: DeviceName,
}

impl FedDevice {
    /// A device.
    pub fn new(id: DeviceId, class: DeviceClass, name: DeviceName) -> FedDevice {
        FedDevice { id, class, name }
    }
}

/// One thing the feed reports, in order.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum FedInput {
    /// A device appeared (the loop configures it with the stored settings).
    Appeared(FedDevice),
    /// A device went away.
    Gone(DeviceId),
    /// An input event, already in our words.
    Event(InputEvent),
}

/// What a poll is told: the instant on the session clock, and the desktop's extent (an absolute
/// device, a touchscreen or a tablet, covers it).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct PollContext {
    /// Now, on the session clock. The events of one poll are stamped with it: libinput's own
    /// timestamps are on the kernel's clock, which the session clock is not.
    pub now: Mono,
    /// The union of the outputs, in global logical coordinates.
    pub extent: LogicalRect,
}

impl PollContext {
    /// A poll at `now` over `extent`.
    pub fn new(now: Mono, extent: LogicalRect) -> PollContext {
        PollContext { now, extent }
    }
}

/// The seat's input devices: what they report, and their configuration.
pub trait InputFeed: std::fmt::Debug {
    /// Configures `device` with `settings`.
    fn apply(&mut self, device: DeviceId, settings: &DeviceSettings) -> Applied;

    /// What the devices reported since the last poll, in order.
    fn poll(&mut self, context: &PollContext) -> Vec<FedInput>;

    /// The seat was lent away: stop reading (libinput's file descriptors are revoked).
    fn suspend(&mut self);

    /// The seat is back: read again.
    fn resume(&mut self);
}
