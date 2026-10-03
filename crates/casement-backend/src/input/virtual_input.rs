//! The virtual input backend.

use super::device_spec::DeviceSpec;
use super::virtual_events::{
    EventStamp, VirtualAxis, VirtualButton, VirtualGestureBegin, VirtualGestureEnd, VirtualKey,
    VirtualMotion, VirtualMotionTo, VirtualPinchUpdate, VirtualSwipeUpdate, VirtualSwitch,
    VirtualTouch,
};
use casement_core::DeviceId;
use casement_core::input::DeviceClass;
#[cfg(any(feature = "testing", feature = "debug-inject"))]
use casement_core::input::{Edge, GestureStep, InputKind, KeyCode, TouchStep};
use smithay::backend::input::{Device, DeviceCapability, InputBackend, InputEvent, UnusedEvent};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::hash::{Hash, Hasher};
use std::path::PathBuf;

/// One event of the virtual backend, in smithay's shape.
pub type VirtualInputEvent = InputEvent<VirtualInput>;

/// Virtual input devices and the events queued on them, oldest first.
#[derive(Debug, Default)]
pub struct VirtualInput {
    devices: BTreeMap<DeviceId, VirtualDevice>,
    queue: VecDeque<VirtualInputEvent>,
    /// The next device number (never reused in a run).
    #[cfg(any(feature = "testing", feature = "debug-inject"))]
    next: u32,
    /// Keys held down, over every device (smithay's `count`).
    #[cfg(any(feature = "testing", feature = "debug-inject"))]
    held: BTreeSet<(DeviceId, KeyCode)>,
}

impl VirtualInput {
    /// No devices, nothing queued.
    pub fn empty() -> VirtualInput {
        VirtualInput::default()
    }

    /// The plugged devices.
    pub fn devices(&self) -> &BTreeMap<DeviceId, VirtualDevice> {
        &self.devices
    }

    /// Every event queued since the last drain, oldest first (the loop's turn).
    pub fn drain(&mut self) -> Vec<VirtualInputEvent> {
        self.queue.drain(..).collect()
    }

    /// Plugs a device (queues `DeviceAdded`) and returns its id.
    #[cfg(any(feature = "testing", feature = "debug-inject"))]
    pub fn plug(&mut self, spec: DeviceSpec) -> DeviceId {
        self.next = self.next.saturating_add(1);
        let id = DeviceId(self.next);
        let device = VirtualDevice { id, spec };
        self.devices.insert(id, device.clone());
        self.queue.push_back(InputEvent::DeviceAdded { device });
        id
    }

    /// Unplugs `device` (queues `DeviceRemoved`); keys it held are released silently.
    #[cfg(any(feature = "testing", feature = "debug-inject"))]
    pub fn unplug(&mut self, device: DeviceId) {
        self.held.retain(|(held_on, _)| *held_on != device);
        if let Some(device) = self.devices.remove(&device) {
            self.queue.push_back(InputEvent::DeviceRemoved { device });
        }
    }

    /// Queues `kind` from `device` at `at` (refused silently for an unplugged device).
    #[cfg(any(feature = "testing", feature = "debug-inject"))]
    pub fn push(
        &mut self,
        device: DeviceId,
        kind: casement_core::input::InputKind,
        at: casement_core::time::Mono,
    ) {
        let Some(device) = self.devices.get(&device).cloned() else {
            return;
        };
        let stamp = EventStamp { device, at };
        let event = self.event_of(stamp, kind);
        self.queue.push_back(event);
    }

    /// `kind` as smithay's event, stamped.
    #[cfg(any(feature = "testing", feature = "debug-inject"))]
    fn event_of(&mut self, stamp: EventStamp, kind: InputKind) -> VirtualInputEvent {
        match kind {
            InputKind::Key { code, edge } => {
                let key = (stamp.device.id, code);
                match edge {
                    Edge::Press => self.held.insert(key),
                    Edge::Release => self.held.remove(&key),
                };
                let count = u32::try_from(self.held.len()).unwrap_or(u32::MAX);
                InputEvent::Keyboard {
                    event: VirtualKey {
                        stamp,
                        code,
                        edge,
                        count,
                    },
                }
            }
            InputKind::Motion { delta } => InputEvent::PointerMotion {
                event: VirtualMotion { stamp, delta },
            },
            InputKind::MotionTo { at } => InputEvent::PointerMotionAbsolute {
                event: VirtualMotionTo { stamp, at },
            },
            InputKind::Button { button, edge } => InputEvent::PointerButton {
                event: VirtualButton {
                    stamp,
                    button,
                    edge,
                },
            },
            InputKind::Axis(frame) => InputEvent::PointerAxis {
                event: VirtualAxis { stamp, frame },
            },
            InputKind::Swipe(step) => match step {
                GestureStep::Begin { fingers } => InputEvent::GestureSwipeBegin {
                    event: VirtualGestureBegin { stamp, fingers },
                },
                GestureStep::Update(delta) => InputEvent::GestureSwipeUpdate {
                    event: VirtualSwipeUpdate { stamp, delta },
                },
                GestureStep::End(end) => InputEvent::GestureSwipeEnd {
                    event: VirtualGestureEnd { stamp, end },
                },
            },
            InputKind::Pinch(step) => match step {
                GestureStep::Begin { fingers } => InputEvent::GesturePinchBegin {
                    event: VirtualGestureBegin { stamp, fingers },
                },
                GestureStep::Update(delta) => InputEvent::GesturePinchUpdate {
                    event: VirtualPinchUpdate { stamp, delta },
                },
                GestureStep::End(end) => InputEvent::GesturePinchEnd {
                    event: VirtualGestureEnd { stamp, end },
                },
            },
            InputKind::Hold(step) => match step {
                GestureStep::Begin { fingers } => InputEvent::GestureHoldBegin {
                    event: VirtualGestureBegin { stamp, fingers },
                },
                // A hold has no update step on the wire: it ends or it does not.
                GestureStep::Update(()) => InputEvent::Special(()),
                GestureStep::End(end) => InputEvent::GestureHoldEnd {
                    event: VirtualGestureEnd { stamp, end },
                },
            },
            InputKind::Touch(step) => match step {
                TouchStep::Down { .. } => InputEvent::TouchDown {
                    event: VirtualTouch { stamp, step },
                },
                TouchStep::Motion { .. } => InputEvent::TouchMotion {
                    event: VirtualTouch { stamp, step },
                },
                TouchStep::Up { .. } => InputEvent::TouchUp {
                    event: VirtualTouch { stamp, step },
                },
                TouchStep::Frame => InputEvent::TouchFrame {
                    event: VirtualTouch { stamp, step },
                },
                TouchStep::Cancel => InputEvent::TouchCancel {
                    event: VirtualTouch { stamp, step },
                },
            },
            InputKind::Switch(step) => InputEvent::SwitchToggle {
                event: VirtualSwitch { stamp, step },
            },
        }
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
        format!("virtual-{}", self.id.0)
    }

    fn name(&self) -> String {
        self.spec.name.0.clone()
    }

    fn has_capability(&self, capability: DeviceCapability) -> bool {
        use DeviceCapability as Cap;
        match self.spec.class {
            DeviceClass::Keyboard => capability == Cap::Keyboard,
            DeviceClass::Pointer => capability == Cap::Pointer,
            DeviceClass::Touchpad => matches!(capability, Cap::Pointer | Cap::Gesture),
            DeviceClass::Tablet => matches!(capability, Cap::TabletTool | Cap::TabletPad),
            DeviceClass::Touch => capability == Cap::Touch,
            DeviceClass::Switch => capability == Cap::Switch,
        }
    }

    fn usb_id(&self) -> Option<(u32, u32)> {
        None
    }

    fn syspath(&self) -> Option<PathBuf> {
        None
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
