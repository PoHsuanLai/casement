//! Which way a key goes to the focused client: through the keyboard (the held-key count, the
//! bindings, the input method's grab), or straight to the client's keyboards past them.
//!
//! While the input method holds the keyboard, every key goes to it, and what it does not use
//! (a letter in English mode, Escape) it sends back through its own virtual keyboard while the
//! person still holds the real key. smithay's keyboard counts held keys across every source, so
//! that copy is a key already down and is dropped, and the real key's release is then not a
//! release either: the field hears nothing while an input method runs (the trial login of
//! 2026-10-19, fcitx5 and the launcher).
//!
//! So a press from a virtual keyboard made through the protocol goes past, straight to the
//! focused client's keyboards, when it is a copy: the input method holds the keyboard and the
//! seat already holds that key (the real one, which went through the count, the bindings and the
//! grab). Any other key goes through as before, an on-screen keyboard's own keys among them. A
//! release goes the way its press went, whatever the grab does in between, so no key is left
//! held on either side ([`PastKeys`]). A key that goes past reaches exactly the client the
//! keyboard focus names, after the same reach check, so the route gives a virtual keyboard
//! nothing more.

use super::{KeyDelivery, edge};
use crate::convert::serial_to;
use crate::virtual_keyboard::VIRTUAL_KEYBOARD_DEVICE;
use casement_core::DeviceId;
use casement_core::input::{DeviceClass, KeyCode, Origin};
use casement_protocols::vocab::PressEdge;
use smithay::input::keyboard::KeyboardHandle;
use smithay::input::{Seat, SeatHandler};
use smithay::reexports::wayland_server::Resource;
use smithay::reexports::wayland_server::protocol::wl_keyboard::KeyState;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::wayland::input_method::InputMethodSeat;
use std::collections::BTreeSet;

/// The way a key goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyRoute {
    /// Through the keyboard: the held-key count, the bindings, the input method's grab.
    ThroughGrab,
    /// Straight to the focused client's keyboards.
    PastGrab,
}

/// What the seat says about one key now: the grab, and whether the key is down.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RouteFacts {
    /// Whether the input method holds the keyboard (no input method, or no field it works on:
    /// it does not): keys go to it.
    pub grabbed: bool,
    /// Whether the seat holds the key down already: a press of it now is a copy.
    pub down: bool,
}

/// What the seat says about `code` now.
pub fn route_facts<D: SeatHandler + 'static>(seat: &Seat<D>, code: KeyCode) -> RouteFacts {
    let grabbed = seat.input_method().keyboard_grabbed();
    let down = seat
        .get_keyboard()
        .is_some_and(|keyboard| keyboard.pressed_keys().contains(&edge::keycode_of(code)));
    RouteFacts { grabbed, down }
}

/// The keys whose press went past the grab, by device: their releases go past too.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct PastKeys(BTreeSet<(DeviceId, KeyCode)>);

impl PastKeys {
    /// The way `key` goes, given what the seat says now, and the record after it.
    pub fn route(mut self, key: &KeyDelivery, facts: RouteFacts) -> (PastKeys, KeyRoute) {
        let held = (key.device, key.code);
        let route = match key.edge {
            PressEdge::Release if self.0.remove(&held) => KeyRoute::PastGrab,
            PressEdge::Release => KeyRoute::ThroughGrab,
            PressEdge::Press => match press_route(&key.origin, key.device, facts) {
                KeyRoute::PastGrab => {
                    self.0.insert(held);
                    KeyRoute::PastGrab
                }
                KeyRoute::ThroughGrab => KeyRoute::ThroughGrab,
            },
        };
        (self, route)
    }

    /// A release that was refused (the session locked while the key was down): its record goes,
    /// so a later press of that key through the keyboard does not send its release past.
    pub fn refused_release(mut self, device: DeviceId, code: KeyCode) -> PastKeys {
        self.0.remove(&(device, code));
        self
    }
}

/// The way a press goes: past only for a copy from a virtual keyboard made through the protocol
/// (its ids start at [`VIRTUAL_KEYBOARD_DEVICE`]) while the input method holds the keyboard. A
/// kernel keyboard, keycap's among them, goes through as before.
fn press_route(origin: &Origin, device: DeviceId, facts: RouteFacts) -> KeyRoute {
    let protocol = device.0 >= VIRTUAL_KEYBOARD_DEVICE.0;
    match (origin, facts.grabbed, facts.down) {
        (Origin::Virtual(DeviceClass::Keyboard), true, true) if protocol => KeyRoute::PastGrab,
        _ => KeyRoute::ThroughGrab,
    }
}

/// Sends `delivery` to every keyboard the focused client bound, the seat's modifiers first (the
/// grab took the real modifier keys, so the client has not heard them), the way smithay sends a
/// focused client its keys. No focus, or a focus whose client is gone, hears nothing.
pub fn deliver_past_grab<D>(seat: &Seat<D>, delivery: &KeyDelivery)
where
    D: SeatHandler<KeyboardFocus = WlSurface> + 'static,
{
    let Some(keyboard) = seat.get_keyboard() else {
        return;
    };
    send_past_grab(&keyboard, delivery);
}

/// Sends `delivery` to every keyboard the focused client bound, the seat's modifiers first.
pub(super) fn send_past_grab<D>(keyboard: &KeyboardHandle<D>, delivery: &KeyDelivery)
where
    D: SeatHandler<KeyboardFocus = WlSurface> + 'static,
{
    let Some(client) = keyboard
        .current_focus()
        .and_then(|surface| surface.client())
    else {
        return;
    };
    let mods = keyboard.modifier_state().serialized;
    let serial = u32::from(serial_to(delivery.serial));
    let time = edge::time_of(delivery.time).millis();
    let state = match delivery.edge {
        PressEdge::Press => KeyState::Pressed,
        PressEdge::Release => KeyState::Released,
    };
    for wire in keyboard.client_keyboards(&client) {
        wire.modifiers(
            serial,
            mods.depressed,
            mods.latched,
            mods.locked,
            mods.layout_effective,
        );
        wire.key(serial, time, delivery.code.0, state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use casement_core::InputSerial;
    use casement_core::time::Mono;

    const PROTOCOL: DeviceId = VIRTUAL_KEYBOARD_DEVICE;
    const KERNEL: DeviceId = DeviceId(7);
    const LAST_KERNEL: DeviceId = DeviceId(VIRTUAL_KEYBOARD_DEVICE.0 - 1);
    const LAST_PROTOCOL: DeviceId = DeviceId(u32::MAX);
    const VIRTUAL: Origin = Origin::Virtual(DeviceClass::Keyboard);
    const HAND: Origin = Origin::Physical(DeviceClass::Keyboard);
    const COPY: RouteFacts = RouteFacts {
        grabbed: true,
        down: true,
    };

    fn key(origin: Origin, device: DeviceId, edge: PressEdge) -> KeyDelivery {
        KeyDelivery {
            code: KeyCode(30),
            edge,
            serial: InputSerial(1),
            time: Mono(std::time::Duration::ZERO),
            origin,
            device,
        }
    }

    fn press(origin: Origin, device: DeviceId, facts: RouteFacts) -> KeyRoute {
        PastKeys::default()
            .route(&key(origin, device, PressEdge::Press), facts)
            .1
    }

    #[test]
    fn only_a_protocol_keyboards_copy_of_a_held_key_goes_past_a_held_grab() {
        let free = RouteFacts {
            grabbed: false,
            down: true,
        };
        let own_key = RouteFacts {
            grabbed: true,
            down: false,
        };
        let cases = [
            (VIRTUAL, PROTOCOL, COPY, KeyRoute::PastGrab),
            (VIRTUAL, LAST_PROTOCOL, COPY, KeyRoute::PastGrab),
            (VIRTUAL, PROTOCOL, free, KeyRoute::ThroughGrab),
            // An on-screen keyboard's own key: nothing real is held, the grab takes it.
            (VIRTUAL, PROTOCOL, own_key, KeyRoute::ThroughGrab),
            // An unlabelled uinput keyboard (ydotool) is virtual but a kernel device.
            (VIRTUAL, KERNEL, COPY, KeyRoute::ThroughGrab),
            (VIRTUAL, LAST_KERNEL, COPY, KeyRoute::ThroughGrab),
            (HAND, KERNEL, COPY, KeyRoute::ThroughGrab),
            (HAND, PROTOCOL, COPY, KeyRoute::ThroughGrab),
            (Origin::Test, PROTOCOL, COPY, KeyRoute::ThroughGrab),
            (
                Origin::Virtual(DeviceClass::Pointer),
                PROTOCOL,
                COPY,
                KeyRoute::ThroughGrab,
            ),
        ];
        for (origin, device, facts, want) in cases {
            assert_eq!(
                press(origin, device, facts),
                want,
                "{origin:?} {device:?} {facts:?}"
            );
        }
    }

    #[test]
    fn a_release_goes_the_way_its_press_went_whatever_the_grab_does() {
        let free = RouteFacts {
            grabbed: false,
            down: false,
        };
        // Past at the press, the grab ends: the release still goes past.
        let (past, route) =
            PastKeys::default().route(&key(VIRTUAL, PROTOCOL, PressEdge::Press), COPY);
        assert_eq!(route, KeyRoute::PastGrab);
        let (past, route) = past.route(&key(VIRTUAL, PROTOCOL, PressEdge::Release), free);
        assert_eq!(route, KeyRoute::PastGrab);
        assert_eq!(past, PastKeys::default(), "nothing is left held");
        // Through at the press, the grab starts: the release goes through.
        let (past, route) = past.route(&key(VIRTUAL, PROTOCOL, PressEdge::Press), free);
        assert_eq!(route, KeyRoute::ThroughGrab);
        let (past, route) = past.route(&key(VIRTUAL, PROTOCOL, PressEdge::Release), COPY);
        assert_eq!(route, KeyRoute::ThroughGrab);
        // Another device's release of the same key is its own.
        let (past, _) = past.route(&key(VIRTUAL, PROTOCOL, PressEdge::Press), COPY);
        let other = DeviceId(PROTOCOL.0 + 1);
        let (past, route) = past.route(&key(VIRTUAL, other, PressEdge::Release), COPY);
        assert_eq!(route, KeyRoute::ThroughGrab);
        let (past, route) = past.route(&key(VIRTUAL, PROTOCOL, PressEdge::Release), free);
        assert_eq!(route, KeyRoute::PastGrab);
        assert_eq!(past, PastKeys::default(), "nothing is left held");
    }

    #[test]
    fn a_refused_release_leaves_no_record_to_send_a_later_release_past() {
        let free = RouteFacts {
            grabbed: false,
            down: false,
        };
        let (past, _) = PastKeys::default().route(&key(VIRTUAL, PROTOCOL, PressEdge::Press), COPY);
        let past = past.refused_release(PROTOCOL, KeyCode(30));
        let (past, route) = past.route(&key(VIRTUAL, PROTOCOL, PressEdge::Press), free);
        assert_eq!(route, KeyRoute::ThroughGrab);
        let (past, route) = past.route(&key(VIRTUAL, PROTOCOL, PressEdge::Release), free);
        assert_eq!(
            route,
            KeyRoute::ThroughGrab,
            "the release follows its press"
        );
        assert_eq!(past, PastKeys::default());
    }
}
