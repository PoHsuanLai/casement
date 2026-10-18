//! The real seat input (feature `libinput`): `InputFeed` over libinput, a thin adapter (B7).
//!
//! libinput opens its device nodes through the seat's session (smithay's
//! `LibinputSessionInterface` over a handle on the libseat session), never directly: a revoked
//! session takes its devices with it, and on a return `resume` reopens them. The context is read
//! by smithay's `LibinputInputBackend` as an event source in the loop; each event it yields is
//! queued, and the loop turn's `poll` converts the queue into our words ([`input_kind_of`]),
//! stamping the poll's instant. Scroll factor is not a libinput setting (libinput scrolls by the
//! device's own step): the compositor scales the axis event, so it is never reported as
//! unsupported here.

use super::DeviceName;
use super::convert::input_kind_of;
use super::devices::{Applied, DeviceSetting, DeviceSettings, InputError};
use super::feed::{FedDevice, FedInput, InputFeed, PollContext};
use casement_core::DeviceId;
use casement_core::input::{DeviceClass, InputEvent as CoreEvent, KernelIds, Maker, Origin};
use casement_core::settings::{AccelProfile, Enabled};
use smithay::backend::input::{Event, InputEvent};
use smithay::backend::libinput::{LibinputInputBackend, LibinputSessionInterface};
use smithay::backend::session::Session;
use smithay::reexports::calloop::LoopHandle;
use smithay::reexports::input::{
    self as libinput, DeviceCapability, DeviceConfigError, DeviceConfigResult, Libinput,
};
use std::cell::RefCell;
use std::collections::{BTreeMap, VecDeque};
use std::rc::Rc;

/// One raw event, as libinput's backend yields it.
type Raw = InputEvent<LibinputInputBackend>;

/// libinput's devices on a seat.
pub struct RealInputFeed {
    /// The seat.
    pub seat: String,
    context: Libinput,
    queue: Rc<RefCell<VecDeque<Raw>>>,
    devices: BTreeMap<DeviceId, Known>,
    ids: BTreeMap<String, DeviceId>,
    next: u32,
}

/// A device libinput reported: its handle and what kind it is.
struct Known {
    device: libinput::Device,
    class: DeviceClass,
    /// Its bus type and the label the desktop's device service wrote into it, which tell a device
    /// made for a service from the person's own (`Origin::of_kernel_device`).
    ids: KernelIds,
}

/// The text of one attribute the kernel keeps for `sysname` (`event5`) under its `device` folder,
/// from sysfs. `None` when it cannot be read.
fn attribute_of(sysname: &str, attribute: &str) -> Option<String> {
    std::fs::read_to_string(format!("/sys/class/input/{sysname}/device/{attribute}")).ok()
}

/// The bus type the kernel gives `sysname`: libinput only reports it from 1.26, newer than Ubuntu
/// 24.04's. `None` when it cannot be read or parsed.
fn bus_of(sysname: &str) -> Option<u16> {
    u16::from_str_radix(attribute_of(sysname, "id/bustype")?.trim(), 16).ok()
}

/// Whom the device service made `sysname` for, from the `phys` it wrote. `None` when there is no
/// such label or `phys` cannot be read: unlabelled.
fn maker_of(sysname: &str) -> Option<Maker> {
    Maker::parse(&attribute_of(sysname, "phys")?)
}

impl RealInputFeed {
    /// Opens libinput on `seat`, its devices opened through `session`, and has `handle`'s loop
    /// read its events.
    pub fn open<D: 'static, S: Session + 'static>(
        seat: &str,
        session: S,
        handle: &LoopHandle<'static, D>,
    ) -> Result<RealInputFeed, InputError> {
        let mut context = Libinput::new_with_udev(LibinputSessionInterface::from(session));
        context.udev_assign_seat(seat).map_err(|()| {
            InputError::Io(std::io::Error::other(format!(
                "libinput could not take seat {seat}"
            )))
        })?;
        let queue = Rc::new(RefCell::new(VecDeque::new()));
        let into = Rc::clone(&queue);
        handle
            .insert_source(
                LibinputInputBackend::new(context.clone()),
                move |event, _, _| into.borrow_mut().push_back(event),
            )
            .map_err(|error| InputError::Io(std::io::Error::other(error.error)))?;
        Ok(RealInputFeed {
            seat: seat.to_owned(),
            context,
            queue,
            devices: BTreeMap::new(),
            ids: BTreeMap::new(),
            next: 0,
        })
    }

    /// The id of `device` for the rest of the run (its kernel name is stable while it is
    /// plugged; an unplugged and replugged device is a new one).
    fn id_of(&mut self, device: &libinput::Device) -> DeviceId {
        let name = device.sysname().to_owned();
        if let Some(id) = self.ids.get(&name) {
            return *id;
        }
        self.next = self.next.saturating_add(1);
        let id = DeviceId(self.next);
        self.ids.insert(name, id);
        id
    }

    /// One raw event in our words, or what it said about the devices.
    fn convert(&mut self, event: &Raw, context: &PollContext) -> Option<FedInput> {
        match event {
            InputEvent::DeviceAdded { device } => {
                let id = self.id_of(device);
                let class = class_of(device);
                self.devices.insert(
                    id,
                    Known {
                        device: device.clone(),
                        class,
                        ids: KernelIds {
                            bus: bus_of(device.sysname()),
                            maker: maker_of(device.sysname()),
                        },
                    },
                );
                Some(FedInput::Appeared(FedDevice::new(
                    id,
                    class,
                    DeviceName(device.name().into_owned()),
                )))
            }
            InputEvent::DeviceRemoved { device } => {
                let id = self.ids.remove(device.sysname())?;
                self.devices.remove(&id);
                Some(FedInput::Gone(id))
            }
            other => {
                let device = device_of(other)?;
                let id = self.id_of(&device);
                let known = self.devices.get(&id)?;
                let origin = Origin::of_kernel_device(known.class, known.ids);
                let kind = input_kind_of(other, context.extent)?;
                Some(FedInput::Event(CoreEvent {
                    origin,
                    device: id,
                    at: context.now,
                    kind,
                }))
            }
        }
    }
}

/// The device an event came from.
fn device_of(event: &Raw) -> Option<libinput::Device> {
    macro_rules! from {
        ($($variant:ident),* $(,)?) => {
            match event {
                $(InputEvent::$variant { event } => Some(Event::<LibinputInputBackend>::device(event)),)*
                _ => None,
            }
        };
    }
    from!(
        Keyboard,
        PointerMotion,
        PointerMotionAbsolute,
        PointerButton,
        PointerAxis,
        GestureSwipeBegin,
        GestureSwipeUpdate,
        GestureSwipeEnd,
        GesturePinchBegin,
        GesturePinchUpdate,
        GesturePinchEnd,
        GestureHoldBegin,
        GestureHoldEnd,
        TouchDown,
        TouchMotion,
        TouchUp,
        TouchCancel,
        TouchFrame,
        SwitchToggle,
    )
}

/// What kind of device libinput describes: a keyboard wins over a pointer that rides along, a
/// pointer that taps with fingers is a touchpad.
fn class_of(device: &libinput::Device) -> DeviceClass {
    let has = |capability| device.has_capability(capability);
    if has(DeviceCapability::Keyboard) {
        DeviceClass::Keyboard
    } else if has(DeviceCapability::Pointer) && device.config_tap_finger_count() > 0 {
        DeviceClass::Touchpad
    } else if has(DeviceCapability::Pointer) {
        DeviceClass::Pointer
    } else if has(DeviceCapability::Touch) {
        DeviceClass::Touch
    } else if has(DeviceCapability::TabletTool) {
        DeviceClass::Tablet
    } else {
        DeviceClass::Switch
    }
}

fn on(enabled: Enabled) -> bool {
    matches!(enabled, Enabled::On)
}

/// What a device that refused a setting means for the result: unsupported and invalid both skip
/// the setting.
fn skipped(result: DeviceConfigResult, setting: DeviceSetting) -> Option<DeviceSetting> {
    match result {
        Ok(()) => None,
        Err(DeviceConfigError::Unsupported | DeviceConfigError::Invalid) => Some(setting),
    }
}

impl InputFeed for RealInputFeed {
    fn apply(&mut self, device: DeviceId, settings: &DeviceSettings) -> Applied {
        let Some(known) = self.devices.get_mut(&device) else {
            return Applied::Refused;
        };
        let device = &mut known.device;
        let profile = match settings.accel {
            AccelProfile::Flat => libinput::AccelProfile::Flat,
            _ => libinput::AccelProfile::Adaptive,
        };
        let skipped: Vec<DeviceSetting> = [
            skipped(
                device.config_accel_set_profile(profile),
                DeviceSetting::Accel,
            ),
            skipped(
                device.config_scroll_set_natural_scroll_enabled(on(settings.natural_scroll)),
                DeviceSetting::NaturalScroll,
            ),
            skipped(
                device.config_tap_set_enabled(on(settings.tap)),
                DeviceSetting::Tap,
            ),
            skipped(
                device.config_dwt_set_enabled(on(settings.disable_while_typing)),
                DeviceSetting::DisableWhileTyping,
            ),
        ]
        .into_iter()
        .flatten()
        .collect();
        if skipped.is_empty() {
            Applied::All
        } else {
            Applied::Partly(skipped)
        }
    }

    fn poll(&mut self, context: &PollContext) -> Vec<FedInput> {
        let raw: Vec<Raw> = self.queue.borrow_mut().drain(..).collect();
        raw.iter()
            .filter_map(|event| self.convert(event, context))
            .collect()
    }

    fn suspend(&mut self) {
        self.context.suspend();
        // What was read before the seat was lent is stale.
        self.queue.borrow_mut().clear();
    }

    fn resume(&mut self) {
        if self.context.resume().is_err() {
            tracing::warn!(
                seat = self.seat,
                "libinput did not resume after the seat came back"
            );
        }
    }
}

impl std::fmt::Debug for RealInputFeed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RealInputFeed")
            .field("seat", &self.seat)
            .field("devices", &self.devices.len())
            .finish_non_exhaustive()
    }
}
