//! Where an input event came from (C19): set where it enters, never changed downstream.

use crate::ids::LeaseId;
use serde::{Deserialize, Serialize};

/// The source of an input event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum Origin {
    /// A kernel input device casement opened itself, not a virtual one.
    Physical(DeviceClass),
    /// A user-writable virtual device (uinput, `virtual_keyboard`): never physical (I8).
    Virtual(DeviceClass),
    /// A trusted feed (palmrest's stream): scroll and system gestures only.
    Feed(FeedSource),
    /// The agent seat of this lease.
    Agent(LeaseId),
    /// A RemoteDesktop portal session.
    Remote(PortalSessionId),
    /// Headless tests only; never compiled into a release seat.
    Test,
}

/// The vendor id the device service (turnstile) gives the devices it makes for palmrest, the
/// touchpad, wheel and key daemon. Kernel drivers use it too (a laptop's built-in AT keyboard is
/// `Bus=0011 Vendor=0001`), so it only names palmrest's device together with [`BUS_VIRTUAL`].
pub const PALMREST_VENDOR: u32 = 0x0001;

/// The bus type of a device made through uinput, as turnstile makes every device.
pub const BUS_VIRTUAL: u16 = 0x06;

/// What a kernel input device says it is: its bus type (`None` when it could not be read) and
/// its vendor id.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KernelIds {
    /// The bus type (`BUS_USB` 0x03, `BUS_I8042` 0x11, [`BUS_VIRTUAL`]), `None` when unreadable.
    pub bus: Option<u16>,
    /// The vendor id.
    pub vendor: u32,
}

impl Origin {
    /// The origin of events from a kernel input device casement opened itself, by the bus and
    /// vendor ids it reports (NEW-PI-4, INPUT-02). A device the desktop's device service made for
    /// palmrest is not the person's hand: the service admits palmrest by a name a same-user process can
    /// borrow, so a forged "pad" could click a consent sheet or the lock. Its events still move
    /// the pointer, fire bindings and drive swipes like a pad's, but are [`Origin::Virtual`], so
    /// the trusted surfaces and the lock never hear them. Only a virtual-bus device with
    /// palmrest's vendor is; every other device is physical, the laptop keyboard that shares the
    /// vendor id and keycap's virtual keyboard (another vendor) among them.
    pub fn of_kernel_device(class: DeviceClass, ids: KernelIds) -> Origin {
        match ids {
            KernelIds {
                bus: Some(BUS_VIRTUAL),
                vendor: PALMREST_VENDOR,
            } => Origin::Virtual(class),
            _ => Origin::Physical(class),
        }
    }

    /// Whether this origin is a touchpad that moves the pointer and swipes spaces: the person's
    /// own pad, or palmrest's (whose swipes arrive as pad gestures, not as a feed).
    pub fn is_pad(&self) -> bool {
        matches!(
            self,
            Origin::Physical(DeviceClass::Touchpad) | Origin::Virtual(DeviceClass::Touchpad)
        )
    }

    /// Whether this origin counts as a person at the hardware (I8): a physical device, or the
    /// headless tests standing in for one. A virtual device never does.
    pub fn is_physical(&self) -> bool {
        matches!(self, Origin::Physical(_) | Origin::Test)
    }

    /// Whether a key chord from this origin may fire a binding: the keyboard the person
    /// types on, a virtual keyboard (an on-screen one, keycap's output) and the tests. Never
    /// the agent seat, a remote session or a feed.
    pub fn fires_bindings(&self) -> bool {
        matches!(
            self,
            Origin::Physical(_) | Origin::Virtual(_) | Origin::Test
        )
    }

    /// Whether a key from this origin may switch the virtual terminal (CASW-13, decided
    /// 2026-10-16): only a kernel keyboard casement opened itself. A virtual keyboard, a remote
    /// session, the agent seat and a feed never do, so no client can send the compositor to a
    /// text console, the lock included.
    pub fn switches_vt(&self) -> bool {
        matches!(self, Origin::Physical(_))
    }
}

/// What kind of device an event came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeviceClass {
    /// A keyboard.
    Keyboard,
    /// A mouse or trackball.
    Pointer,
    /// A touchpad.
    Touchpad,
    /// A drawing tablet.
    Tablet,
    /// A touchscreen.
    Touch,
    /// A switch (lid, tablet mode).
    Switch,
}

/// A trusted feed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeedSource {
    /// palmrest's touchpad stream.
    Palmrest,
}

/// One RemoteDesktop portal session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PortalSessionId(pub u64);

#[cfg(test)]
mod tests {
    use super::*;

    /// NEW-PI-4: which kernel devices count as the person's hands. A laptop's built-in AT
    /// keyboard shares palmrest's vendor id on another bus (final audit), and keycap's keyboard is
    /// on the virtual bus with another vendor: both are the person's.
    #[test]
    fn only_a_virtual_bus_device_with_palmrests_vendor_is_not_physical() {
        const USB: Option<u16> = Some(0x03);
        const I8042: Option<u16> = Some(0x11);
        const VIRTUAL: Option<u16> = Some(BUS_VIRTUAL);
        let ids = |bus, vendor| KernelIds { bus, vendor };
        let cases = [
            (
                "a USB keyboard",
                DeviceClass::Keyboard,
                ids(USB, 0x05ac),
                true,
            ),
            (
                "a USB touchpad",
                DeviceClass::Touchpad,
                ids(USB, 0x046d),
                true,
            ),
            (
                "a laptop's AT keyboard",
                DeviceClass::Keyboard,
                ids(I8042, PALMREST_VENDOR),
                true,
            ),
            (
                "keycap's keyboard",
                DeviceClass::Keyboard,
                ids(VIRTUAL, 0x4b43),
                true,
            ),
            (
                "an unreadable bus",
                DeviceClass::Keyboard,
                ids(None, PALMREST_VENDOR),
                true,
            ),
            (
                "palmrest's touchpad",
                DeviceClass::Touchpad,
                ids(VIRTUAL, PALMREST_VENDOR),
                false,
            ),
            (
                "palmrest's keys",
                DeviceClass::Keyboard,
                ids(VIRTUAL, PALMREST_VENDOR),
                false,
            ),
        ];
        for (name, class, ids, physical) in cases {
            let origin = Origin::of_kernel_device(class, ids);
            assert_eq!(origin.is_physical(), physical, "{name}");
            // Whatever its maker, it fires bindings, and no kernel device is a remote session.
            assert!(origin.fires_bindings(), "{name}");
        }
        let palmrest = ids(VIRTUAL, PALMREST_VENDOR);
        assert!(Origin::of_kernel_device(DeviceClass::Touchpad, palmrest).is_pad());
        assert!(!Origin::of_kernel_device(DeviceClass::Pointer, palmrest).is_pad());
    }
}
