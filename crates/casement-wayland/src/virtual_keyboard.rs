//! `zwp_virtual_keyboard_manager_v1`: keyboards a client creates (on-screen keyboards, input
//! tools). Their keys are `Origin::Virtual` (I8: never physical) and count as idle activity.
//!
//! smithay hands a virtual keyboard's keys straight to the focused client, past the model
//! (A36). It has no hook either, so this module carries its own global, manager and keyboard
//! user data with their own `GlobalDispatch2` and `Dispatch2` impls (the way
//! `quire_window` does); `delegate_dispatch2!` in the composition root routes to them and
//! they reach the root through [`VirtualKeyboardHandler`]. A key becomes an
//! `Event::Input` stamped with the session clock (the client's own `time` is not trusted),
//! and the model decides where it goes like any other key. Hidden from sandboxed clients (I7).
//!
//! Limits at P0: the keymap a client sends is checked (it must compile, or `key` is a
//! protocol error) but keys are delivered through the seat's own keymap, so a keyboard with a
//! different layout than the seat's types that layout's keys; and `modifiers` is checked but
//! not applied (an on-screen keyboard sends its modifier keys as keys).

use crate::states::GlobalFilter;
use casement_core::input::DeviceClass;
use casement_core::input::{Edge, InputEvent, InputKind, KeyCode, Origin};
use casement_core::policy::GlobalKind;
use casement_core::time::Mono;
use casement_core::{DeviceId, Event};
use smithay::input::keyboard::xkb;
use smithay::reexports::wayland_protocols_misc::zwp_virtual_keyboard_v1::server::{
    zwp_virtual_keyboard_manager_v1::{self, ZwpVirtualKeyboardManagerV1},
    zwp_virtual_keyboard_v1::{self, ZwpVirtualKeyboardV1},
};
use smithay::reexports::wayland_server::backend::{ClientId, GlobalId};
use smithay::reexports::wayland_server::protocol::wl_keyboard::KeymapFormat;
use smithay::reexports::wayland_server::{
    Client, DataInit, Dispatch, DisplayHandle, GlobalDispatch, New, Resource,
};
use smithay::wayland::{Dispatch2, GlobalDispatch2};
use std::collections::BTreeSet;
use std::fs::File;
use std::os::fd::OwnedFd;
use std::os::unix::fs::FileExt;
use std::sync::{Mutex, MutexGuard, PoisonError};

/// The version casement serves.
pub const VERSION: u32 = 1;

/// The device id every virtual keyboard's events carry: far above the backend's own
/// numbering (which counts from 1), so it never names a physical device.
pub const VIRTUAL_KEYBOARD_DEVICE: DeviceId = DeviceId(0x8000_0000);

/// The largest keymap text taken from a client.
const KEYMAP_LIMIT: usize = 1 << 20;

/// `zwp_virtual_keyboard_manager_v1`.
#[derive(Debug)]
pub struct VirtualKeyboardPart {
    /// The global.
    pub global: GlobalId,
    /// The device id its keys carry.
    pub device: DeviceId,
}

impl VirtualKeyboardPart {
    /// Creates the global (version 1), filtered by the I7 table.
    pub fn new<D>(display: &DisplayHandle, role_filter: GlobalFilter) -> VirtualKeyboardPart
    where
        D: GlobalDispatch<ZwpVirtualKeyboardManagerV1, VirtualKeyboardGlobal> + 'static,
    {
        VirtualKeyboardPart {
            global: display.create_global::<D, ZwpVirtualKeyboardManagerV1, _>(
                VERSION,
                VirtualKeyboardGlobal {
                    filter: role_filter,
                },
            ),
            device: VIRTUAL_KEYBOARD_DEVICE,
        }
    }
}

/// What the composition root provides to this protocol's dispatch.
pub trait VirtualKeyboardHandler: 'static {
    /// The part.
    fn virtual_keyboard_part(&mut self) -> &mut VirtualKeyboardPart;
    /// The session clock's now (`Mono`): the instant a key is stamped with.
    fn virtual_keyboard_now(&self) -> Mono;
    /// Feeds the events a request produced into the model (`core_loop::feed`).
    fn virtual_keyboard_events(&mut self, events: Vec<Event>);
}

/// The global's data: the filter its `can_view` asks.
#[derive(Debug, Clone, Copy)]
pub struct VirtualKeyboardGlobal {
    filter: GlobalFilter,
}

/// A bound manager's data.
#[derive(Debug, Clone, Copy, Default)]
pub struct VirtualKeyboardManagerData;

/// Whether a keyboard has sent a keymap that compiled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum KeymapState {
    /// None yet (or the last one did not compile): `key` is a protocol error.
    Missing,
    /// One compiled.
    Loaded,
}

/// One virtual keyboard's data.
#[derive(Debug)]
pub struct VirtualKeyboardData {
    keymap: Mutex<KeymapState>,
    pressed: Mutex<BTreeSet<KeyCode>>,
}

impl VirtualKeyboardData {
    fn new() -> VirtualKeyboardData {
        VirtualKeyboardData {
            keymap: Mutex::new(KeymapState::Missing),
            pressed: Mutex::new(BTreeSet::new()),
        }
    }
}

/// A virtual keyboard sent a key: an `Event::Input` with `Origin::Virtual(Keyboard)`.
pub fn on_vk_key(
    part: &mut VirtualKeyboardPart,
    code: KeyCode,
    edge: Edge,
    at: Mono,
) -> Vec<Event> {
    vec![key_event(part.device, code, edge, at)]
}

fn key_event(device: DeviceId, code: KeyCode, edge: Edge, at: Mono) -> Event {
    Event::Input(InputEvent {
        origin: Origin::Virtual(DeviceClass::Keyboard),
        device,
        at,
        kind: InputKind::Key { code, edge },
    })
}

/// The wire's key state as an edge (`0` released, `1` pressed; anything else is not a state).
fn edge_of(state: u32) -> Option<Edge> {
    match state {
        0 => Some(Edge::Release),
        1 => Some(Edge::Press),
        _ => None,
    }
}

/// The releases of every key still down, in key order.
fn releases(pressed: &BTreeSet<KeyCode>, device: DeviceId, at: Mono) -> Vec<Event> {
    pressed
        .iter()
        .map(|code| key_event(device, *code, Edge::Release, at))
        .collect()
}

/// A poisoned lock is still a lock: the data is plain.
fn locked<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// The keymap text in `fd`, if it is at most [`KEYMAP_LIMIT`] bytes of UTF-8.
fn keymap_text(fd: OwnedFd, size: usize) -> Option<String> {
    if size == 0 || size > KEYMAP_LIMIT {
        return None;
    }
    let mut bytes = vec![0; size];
    File::from(fd).read_exact_at(&mut bytes, 0).ok()?;
    let text = String::from_utf8(bytes).ok()?;
    Some(text.trim_end_matches('\0').to_owned())
}

/// Whether `text` is an XKB keymap libxkbcommon accepts.
fn compiles(text: String) -> KeymapState {
    let context = xkb::Context::new(xkb::CONTEXT_NO_FLAGS);
    xkb::Keymap::new_from_string(
        &context,
        text,
        xkb::KEYMAP_FORMAT_TEXT_V1,
        xkb::KEYMAP_COMPILE_NO_FLAGS,
    )
    .map_or(KeymapState::Missing, |_| KeymapState::Loaded)
}

impl<D> GlobalDispatch2<ZwpVirtualKeyboardManagerV1, D> for VirtualKeyboardGlobal
where
    D: Dispatch<ZwpVirtualKeyboardManagerV1, VirtualKeyboardManagerData> + 'static,
{
    fn bind(
        &self,
        _state: &mut D,
        _handle: &DisplayHandle,
        _client: &Client,
        resource: New<ZwpVirtualKeyboardManagerV1>,
        data_init: &mut DataInit<'_, D>,
    ) {
        data_init.init(resource, VirtualKeyboardManagerData);
    }

    fn can_view(&self, client: &Client) -> bool {
        self.filter.sees(GlobalKind::VirtualKeyboard, client)
    }
}

impl<D> Dispatch2<ZwpVirtualKeyboardManagerV1, D> for VirtualKeyboardManagerData
where
    D: Dispatch<ZwpVirtualKeyboardV1, VirtualKeyboardData> + 'static,
{
    fn request(
        &self,
        _state: &mut D,
        _client: &Client,
        _resource: &ZwpVirtualKeyboardManagerV1,
        request: zwp_virtual_keyboard_manager_v1::Request,
        _handle: &DisplayHandle,
        data_init: &mut DataInit<'_, D>,
    ) {
        if let zwp_virtual_keyboard_manager_v1::Request::CreateVirtualKeyboard { id, .. } = request
        {
            data_init.init(id, VirtualKeyboardData::new());
        }
    }
}

impl VirtualKeyboardData {
    /// Records `code`'s new edge and feeds the key to the model.
    fn key<D: VirtualKeyboardHandler>(&self, state: &mut D, code: KeyCode, edge: Edge) {
        {
            let mut pressed = locked(&self.pressed);
            match edge {
                Edge::Press => pressed.insert(code),
                Edge::Release => pressed.remove(&code),
            };
        }
        let at = state.virtual_keyboard_now();
        let events = on_vk_key(state.virtual_keyboard_part(), code, edge, at);
        state.virtual_keyboard_events(events);
    }
}

impl<D> Dispatch2<ZwpVirtualKeyboardV1, D> for VirtualKeyboardData
where
    D: VirtualKeyboardHandler,
{
    fn request(
        &self,
        state: &mut D,
        _client: &Client,
        resource: &ZwpVirtualKeyboardV1,
        request: zwp_virtual_keyboard_v1::Request,
        _handle: &DisplayHandle,
        _data_init: &mut DataInit<'_, D>,
    ) {
        let keymap = *locked(&self.keymap);
        match request {
            zwp_virtual_keyboard_v1::Request::Keymap { format, fd, size } => {
                let loaded = (format == KeymapFormat::XkbV1 as u32)
                    .then(|| keymap_text(fd, size as usize))
                    .flatten()
                    .map_or(KeymapState::Missing, compiles);
                *locked(&self.keymap) = loaded;
            }
            zwp_virtual_keyboard_v1::Request::Key { .. }
            | zwp_virtual_keyboard_v1::Request::Modifiers { .. }
                if keymap == KeymapState::Missing =>
            {
                resource.post_error(
                    zwp_virtual_keyboard_v1::Error::NoKeymap,
                    "a keymap must be sent first",
                );
            }
            zwp_virtual_keyboard_v1::Request::Key {
                key, state: wire, ..
            } => {
                if let Some(edge) = edge_of(wire) {
                    self.key(state, KeyCode(key), edge);
                }
            }
            _ => {}
        }
    }

    fn destroyed(&self, state: &mut D, _client: ClientId, _resource: &ZwpVirtualKeyboardV1) {
        let at = state.virtual_keyboard_now();
        let device = state.virtual_keyboard_part().device;
        let events = releases(&locked(&self.pressed), device, at);
        if !events.is_empty() {
            state.virtual_keyboard_events(events);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    const AT: Mono = Mono(Duration::from_millis(1500));

    #[test]
    fn a_virtual_key_is_an_input_event_from_a_virtual_keyboard() {
        let event = key_event(VIRTUAL_KEYBOARD_DEVICE, KeyCode(30), Edge::Press, AT);
        let Event::Input(input) = event else {
            panic!("not an input event");
        };
        assert_eq!(input.origin, Origin::Virtual(DeviceClass::Keyboard));
        assert!(!input.origin.is_physical());
        assert_eq!(input.device, VIRTUAL_KEYBOARD_DEVICE);
        assert_eq!(input.at, AT);
        assert_eq!(
            input.kind,
            InputKind::Key {
                code: KeyCode(30),
                edge: Edge::Press
            }
        );
    }

    #[test]
    fn only_zero_and_one_are_key_states() {
        assert_eq!(edge_of(0), Some(Edge::Release));
        assert_eq!(edge_of(1), Some(Edge::Press));
        assert_eq!(edge_of(2), None);
    }

    #[test]
    fn a_vanished_keyboard_releases_the_keys_it_held_in_key_order() {
        let held = BTreeSet::from([KeyCode(42), KeyCode(30)]);
        let events = releases(&held, VIRTUAL_KEYBOARD_DEVICE, AT);
        assert_eq!(
            events,
            vec![
                key_event(VIRTUAL_KEYBOARD_DEVICE, KeyCode(30), Edge::Release, AT),
                key_event(VIRTUAL_KEYBOARD_DEVICE, KeyCode(42), Edge::Release, AT),
            ]
        );
    }

    #[test]
    fn only_a_keymap_that_compiles_counts() {
        let plain = "xkb_keymap {\n xkb_keycodes { <A> = 38; };\n xkb_types {};\n xkb_compat {};\n xkb_symbols { key <A> { [ a ] }; };\n};\n";
        assert_eq!(compiles(plain.to_owned()), KeymapState::Loaded);
        assert_eq!(compiles("not a keymap".to_owned()), KeymapState::Missing);
    }
}
