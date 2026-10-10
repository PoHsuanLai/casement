//! Input devices: [`VirtualInput`], smithay's `InputBackend` over queued events, the same
//! path libinput takes (11 §2.5), and the libinput settings seam ([`InputDevices`], P1-Z), whose
//! real adapter is behind feature `libinput`.
//!
//! Physical, Virtual and Test origins are minted only under features `testing` or
//! `debug-inject`: the functions that plug devices and push events exist only there, and a
//! binary with either feature refuses every backend but headless and every run outside the
//! jail (`casement/src/jail_check.rs`).

mod config;
mod device_spec;
mod devices;
mod virtual_events;
mod virtual_input;

#[cfg(feature = "libinput")]
mod libinput_real;

pub use config::{device_settings, xkb_of};
pub use device_spec::{DeviceName, DeviceSpec};
pub use devices::{AccelProfile, Applied, DeviceSetting, DeviceSettings, InputDevices, InputError};
#[cfg(feature = "libinput")]
pub use libinput_real::RealInputDevices;
pub use virtual_events::{
    EventStamp, VirtualAxis, VirtualButton, VirtualGestureBegin, VirtualGestureEnd, VirtualKey,
    VirtualMotion, VirtualMotionTo, VirtualPinchUpdate, VirtualSwipeUpdate, VirtualSwitch,
    VirtualTouch,
};
pub use virtual_input::{VirtualDevice, VirtualInput, VirtualInputEvent};
