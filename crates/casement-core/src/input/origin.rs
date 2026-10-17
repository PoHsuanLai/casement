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
/// touchpad, wheel and key daemon. Nothing else in a session has it: a real keyboard or pad has
/// its maker's, keycap's own devices have another.
pub const PALMREST_VENDOR: u32 = 0x0001;

impl Origin {
    /// The origin of events from a kernel input device casement opened itself, by the vendor id
    /// it reports (NEW-PI-4, INPUT-02). A device the desktop's device service made for palmrest is
    /// not the person's hand: the service admits palmrest by a name a same-user process can
    /// borrow, so a forged "pad" could click a consent sheet or the lock. Its events still move
    /// the pointer, fire bindings and drive swipes like a pad's, but are [`Origin::Virtual`], so
    /// the trusted surfaces and the lock never hear them. Every other device is physical.
    pub fn of_kernel_device(class: DeviceClass, vendor: u32) -> Origin {
        match vendor {
            PALMREST_VENDOR => Origin::Virtual(class),
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

    /// NEW-PI-4: which kernel devices count as the person's hands.
    #[test]
    fn only_a_device_the_service_made_for_palmrest_is_not_physical() {
        let cases = [
            ("a real keyboard", DeviceClass::Keyboard, 0x05ac, true),
            ("a real touchpad", DeviceClass::Touchpad, 0x046d, true),
            ("keycap's keyboard", DeviceClass::Keyboard, 0x4b43, true),
            (
                "palmrest's touchpad",
                DeviceClass::Touchpad,
                PALMREST_VENDOR,
                false,
            ),
            (
                "palmrest's keys",
                DeviceClass::Keyboard,
                PALMREST_VENDOR,
                false,
            ),
        ];
        for (name, class, vendor, physical) in cases {
            let origin = Origin::of_kernel_device(class, vendor);
            assert_eq!(origin.is_physical(), physical, "{name}");
            // Whatever its maker, it fires bindings, and no kernel device is a remote session.
            assert!(origin.fires_bindings(), "{name}");
        }
        assert!(Origin::of_kernel_device(DeviceClass::Touchpad, PALMREST_VENDOR).is_pad());
        assert!(!Origin::of_kernel_device(DeviceClass::Pointer, PALMREST_VENDOR).is_pad());
    }
}
