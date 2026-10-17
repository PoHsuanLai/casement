//! Where an input event came from (C19): set where it enters, never changed downstream.

use super::maker::Maker;
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

/// The bus type of a device made through uinput, as turnstile makes every device.
pub const BUS_VIRTUAL: u16 = 0x06;

/// What a kernel input device says it is: its bus type and, for a virtual one, the label the
/// device service wrote into it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KernelIds {
    /// The bus type (`BUS_USB` 0x03, `BUS_I8042` 0x11, [`BUS_VIRTUAL`]), `None` when unreadable.
    pub bus: Option<u16>,
    /// Whom the device service made it for (`phys`), `None` when there is no such label or it
    /// could not be read.
    pub maker: Option<Maker>,
}

impl Origin {
    /// The origin of events from a kernel input device casement opened itself, by the bus type it
    /// reports and the label of its maker (NEW-PI-4, INPUT-02, NEW-FA-4). Every device that is not on
    /// the virtual bus is physical, the laptop's built-in keyboard among them. A virtual-bus
    /// device is the person's hand only when the device service labelled it for a service that
    /// replays their own hardware ([`Maker::origin`]); any other (no label, another label: ydotool,
    /// a `uinput` group member, a forged "pad") is [`Origin::Virtual`], so the trusted surfaces,
    /// the lock and VT switching never hear it. Virtual events still move the pointer, fire bindings
    /// and drive swipes.
    pub fn of_kernel_device(class: DeviceClass, ids: KernelIds) -> Origin {
        match ids {
            KernelIds {
                bus: Some(BUS_VIRTUAL),
                maker,
            } => maker.map_or(Origin::Virtual(class), |m| m.origin(class)),
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

    /// NEW-PI-4, NEW-FA-4: which kernel devices count as the person's hands. Only a virtual-bus
    /// device is ever in doubt; there, the label turnstile wrote decides (never the vendor id).
    #[test]
    fn a_virtual_device_is_the_persons_hand_only_when_made_for_keycap() {
        const USB: Option<u16> = Some(0x03);
        const I8042: Option<u16> = Some(0x11);
        const VIRTUAL: Option<u16> = Some(BUS_VIRTUAL);
        let ids = |bus, label: &str| KernelIds {
            bus,
            maker: Maker::parse(label),
        };
        let cases = [
            (
                "a laptop's AT keyboard",
                DeviceClass::Keyboard,
                ids(I8042, "isa0060/serio0/input0"),
                true,
            ),
            (
                "a USB mouse",
                DeviceClass::Pointer,
                ids(USB, "usb-0000:00:14.0-2/input0"),
                true,
            ),
            (
                "keycap's keyboard",
                DeviceClass::Keyboard,
                ids(VIRTUAL, "turnstile/keymap_daemon"),
                true,
            ),
            (
                "keycap's pointer",
                DeviceClass::Pointer,
                ids(VIRTUAL, "turnstile/keymap_daemon\n"),
                true,
            ),
            // palmrest turns gestures into synthetic key chords (see `Maker::origin`).
            (
                "palmrest's touchpad",
                DeviceClass::Touchpad,
                ids(VIRTUAL, "turnstile/pad_daemon"),
                false,
            ),
            (
                "palmrest's keys",
                DeviceClass::Keyboard,
                ids(VIRTUAL, "turnstile/pad_daemon"),
                false,
            ),
            (
                "an unlabelled virtual keyboard (ydotool)",
                DeviceClass::Keyboard,
                ids(VIRTUAL, ""),
                false,
            ),
            (
                "a label that only starts with turnstile",
                DeviceClass::Keyboard,
                ids(VIRTUAL, "turnstile/settings"),
                false,
            ),
            (
                "a label with more after the role",
                DeviceClass::Keyboard,
                ids(VIRTUAL, "turnstile/keymap_daemon/2"),
                false,
            ),
            (
                "a keycap label on a USB device",
                DeviceClass::Keyboard,
                ids(USB, "turnstile/keymap_daemon"),
                true,
            ),
            (
                "an unreadable bus",
                DeviceClass::Keyboard,
                KernelIds {
                    bus: None,
                    maker: None,
                },
                true,
            ),
        ];
        for (name, class, ids, physical) in cases {
            let origin = Origin::of_kernel_device(class, ids);
            assert_eq!(origin.is_physical(), physical, "{name}");
            // Whatever its maker, it fires bindings, and no kernel device is a remote session.
            assert!(origin.fires_bindings(), "{name}");
            assert_eq!(origin.switches_vt(), physical, "{name}");
        }
    }

    /// palmrest's pad is virtual but still a pad: its swipes drive the Spaces.
    #[test]
    fn palmrests_pad_still_drives_swipes() {
        let palmrest = KernelIds {
            bus: Some(BUS_VIRTUAL),
            maker: Some(Maker::PadDaemon),
        };
        assert!(Origin::of_kernel_device(DeviceClass::Touchpad, palmrest).is_pad());
        assert!(!Origin::of_kernel_device(DeviceClass::Pointer, palmrest).is_pad());
    }
}
