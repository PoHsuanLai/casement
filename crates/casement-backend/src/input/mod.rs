//! Input devices: [`VirtualInput`], smithay's `InputBackend` over queued events, the same
//! path libinput takes (11 §2.5), and the libinput settings seam ([`InputFeed::apply`], P1-Z), whose
//! real adapter is behind feature `libinput`.
//!
//! Physical, Virtual and Test origins are minted only under features `testing` or
//! `debug-inject`: the functions that plug devices and push events exist only there, and a
//! binary with either feature refuses every backend but headless and every run outside the
//! jail (`casement/src/jail_check.rs`).

mod config;
mod convert;
mod device_spec;
mod devices;
mod feed;
mod present;
mod virtual_events;
mod virtual_input;

#[cfg(feature = "libinput")]
mod libinput_real;

pub use config::{device_settings, scaled_scroll, xkb_of};
pub use convert::input_kind_of;
pub use device_spec::{DeviceName, DeviceSpec};
pub use devices::{AccelProfile, Applied, DeviceSetting, DeviceSettings, InputError};
pub use feed::{FedDevice, FedInput, InputFeed, PollContext};
#[cfg(feature = "libinput")]
pub use libinput_real::RealInputFeed;
pub use present::PresentDevices;
pub use virtual_events::{
    EventStamp, VirtualAxis, VirtualButton, VirtualGestureBegin, VirtualGestureEnd, VirtualKey,
    VirtualMotion, VirtualMotionTo, VirtualPinchUpdate, VirtualSwipeUpdate, VirtualSwitch,
    VirtualTouch,
};
pub use virtual_input::{VirtualDevice, VirtualInput, VirtualInputEvent};
