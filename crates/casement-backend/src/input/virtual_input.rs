//! The virtual input backend.

use super::device_spec::DeviceSpec;
use super::virtual_events::{
    VirtualAxis, VirtualButton, VirtualGestureBegin, VirtualGestureEnd, VirtualKey, VirtualMotion,
    VirtualMotionTo, VirtualPinchUpdate, VirtualSwipeUpdate, VirtualSwitch, VirtualTouch,
};
use casement_core::DeviceId;
use smithay::backend::input::{Device, DeviceCapability, InputBackend, InputEvent, UnusedEvent};
use std::collections::{BTreeMap, VecDeque};
use std::hash::{Hash, Hasher};
use std::path::PathBuf;

/// One event of the virtual backend, in smithay's shape.
pub type VirtualInputEvent = InputEvent<VirtualInput>;

/// Virtual input devices and the events queued on them, oldest first.
#[derive(Debug, Default)]
pub struct VirtualInput {
    #[expect(dead_code, reason = "P0-B-headless: the plugged devices")]
    devices: BTreeMap<DeviceId, VirtualDevice>,
    #[expect(dead_code, reason = "P0-B-headless: the queued events")]
    queue: VecDeque<VirtualInputEvent>,
}

impl VirtualInput {
    /// No devices, nothing queued.
    pub fn empty() -> VirtualInput {
        VirtualInput::default()
    }

    /// Plugs a device (queues `DeviceAdded`) and returns its id.
    #[cfg(any(feature = "testing", feature = "debug-inject"))]
    pub fn plug(&mut self, spec: DeviceSpec) -> DeviceId {
        let _ = spec;
        todo!("P0-B-headless: VirtualInput::plug")
    }

    /// Unplugs `device` (queues `DeviceRemoved`).
    #[cfg(any(feature = "testing", feature = "debug-inject"))]
    pub fn unplug(&mut self, device: DeviceId) {
        let _ = device;
        todo!("P0-B-headless: VirtualInput::unplug")
    }

    /// Queues `kind` from `device` at `at` (refused silently for an unplugged device).
    #[cfg(any(feature = "testing", feature = "debug-inject"))]
    pub fn push(
        &mut self,
        device: DeviceId,
        kind: casement_core::input::InputKind,
        at: casement_core::time::Mono,
    ) {
        let _ = (device, kind, at);
        todo!("P0-B-headless: VirtualInput::push")
    }
}

/// A plugged virtual device.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VirtualDevice {
    /// Its id.
    pub id: DeviceId,
    /// What it is.
    pub spec: DeviceSpec,
}

impl Hash for VirtualDevice {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl Device for VirtualDevice {
    fn id(&self) -> String {
        todo!("P0-B-headless: VirtualDevice::id")
    }

    fn name(&self) -> String {
        todo!("P0-B-headless: VirtualDevice::name")
    }

    fn has_capability(&self, capability: DeviceCapability) -> bool {
        let _ = capability;
        todo!("P0-B-headless: VirtualDevice::has_capability")
    }

    fn usb_id(&self) -> Option<(u32, u32)> {
        todo!("P0-B-headless: VirtualDevice::usb_id")
    }

    fn syspath(&self) -> Option<PathBuf> {
        todo!("P0-B-headless: VirtualDevice::syspath")
    }
}

impl InputBackend for VirtualInput {
    type Device = VirtualDevice;
    type KeyboardKeyEvent = VirtualKey;
    type PointerAxisEvent = VirtualAxis;
    type PointerButtonEvent = VirtualButton;
    type PointerMotionEvent = VirtualMotion;
    type PointerMotionAbsoluteEvent = VirtualMotionTo;
    type GestureSwipeBeginEvent = VirtualGestureBegin;
    type GestureSwipeUpdateEvent = VirtualSwipeUpdate;
    type GestureSwipeEndEvent = VirtualGestureEnd;
    type GesturePinchBeginEvent = VirtualGestureBegin;
    type GesturePinchUpdateEvent = VirtualPinchUpdate;
    type GesturePinchEndEvent = VirtualGestureEnd;
    type GestureHoldBeginEvent = VirtualGestureBegin;
    type GestureHoldEndEvent = VirtualGestureEnd;
    type TouchDownEvent = VirtualTouch;
    type TouchUpEvent = VirtualTouch;
    type TouchMotionEvent = VirtualTouch;
    type TouchCancelEvent = VirtualTouch;
    type TouchFrameEvent = VirtualTouch;
    // The model has no tablet input kind at P0 (`casement_core::input::InputKind`): tablets
    // are a libinput-only path until a delta freeze adds one (FINDINGS).
    type TabletToolAxisEvent = UnusedEvent;
    type TabletToolProximityEvent = UnusedEvent;
    type TabletToolTipEvent = UnusedEvent;
    type TabletToolButtonEvent = UnusedEvent;
    type SwitchToggleEvent = VirtualSwitch;
    type SpecialEvent = ();
}
