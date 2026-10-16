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
//! Consent (`Power::Keys`): a keyboard made while the answer is out keeps its requests and
//! replays them when the person allows; a refused or revoked keyboard is dead (`gate`).
//!
//! Limits at P0: the keymap a client sends is checked (it must compile, or `key` is a
//! protocol error) but keys are delivered through the seat's own keymap, so a keyboard with a
//! different layout than the seat's types that layout's keys; and `modifiers` is checked but
//! not applied (an on-screen keyboard sends its modifier keys as keys).

mod gate;

use crate::client_file;
use crate::consent::{self, Admission, ConsentHandler, Held, Mortal, Pushed};
use crate::limits::{self, Limit, Standing};
use crate::states::GlobalFilter;
use casement_core::ClientKey;
use casement_core::input::DeviceClass;
use casement_core::input::{Edge, InputEvent, InputKind, KeyCode, Origin};
use casement_core::policy::GlobalKind;
use casement_core::privilege::PowerOutcome;
use casement_core::time::Mono;
use casement_core::{DeviceId, Event};
use casement_protocols::vocab::Power;
use gate::{Gate, Routed, Start};
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
use std::sync::{Mutex, MutexGuard, PoisonError};

/// The version casement serves.
pub const VERSION: u32 = 1;

/// The first device id a virtual keyboard's events carry: far above the backend's own
/// numbering (which counts from 1), so it never names a physical device. Each keyboard takes
/// the next one ([`VirtualKeyboardPart::next_device`]), so two clients' pressed keys never
/// share a device and one's going releases only its own.
pub const VIRTUAL_KEYBOARD_DEVICE: DeviceId = DeviceId(0x8000_0000);

/// The most keys one virtual keyboard may hold down at once (a keyboard has ten fingers and a
/// handful of modifiers): a press past it is dropped, so a client cannot grow the set without
/// bound.
const MAX_PRESSED: usize = 64;

/// The largest keymap text taken from a client.
const KEYMAP_LIMIT: usize = 1 << 20;

/// `zwp_virtual_keyboard_manager_v1`.
#[derive(Debug)]
pub struct VirtualKeyboardPart {
    /// The global.
    pub global: GlobalId,
    /// The device id the next keyboard made gets.
    pub device: DeviceId,
    /// The display, for replaying what a keyboard kept (a replay has no request to take it
    /// from).
    display: DisplayHandle,
    /// Every live keyboard of each client (at most [`HELD_REQUESTS`] each), so the answer to a
    /// question reaches the ones that wait on it and a power taken back ends them all.
    ///
    /// [`HELD_REQUESTS`]: casement_core::privilege::HELD_REQUESTS
    pub keyboards: Held<ZwpVirtualKeyboardV1>,
}

impl Mortal for ZwpVirtualKeyboardV1 {
    fn alive(&self) -> bool {
        self.is_alive()
    }
}

impl VirtualKeyboardPart {
    /// A device id for a new keyboard, and the part after it. The ids count up from
    /// [`VIRTUAL_KEYBOARD_DEVICE`] and wrap inside the virtual range, never into a backend's.
    pub fn next_device(&mut self) -> DeviceId {
        let given = self.device;
        self.device = successor(given);
        given
    }

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
            display: display.clone(),
            keyboards: Held::default(),
        }
    }
}

/// What the composition root provides to this protocol's dispatch.
pub trait VirtualKeyboardHandler: ConsentHandler {
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
    device: DeviceId,
    gate: Gate,
    keymap: Mutex<KeymapState>,
    keymaps_sent: Mutex<usize>,
    pressed: Mutex<BTreeSet<KeyCode>>,
}

impl VirtualKeyboardData {
    fn new(device: DeviceId, start: Start) -> VirtualKeyboardData {
        VirtualKeyboardData {
            device,
            gate: Gate::new(start),
            keymap: Mutex::new(KeymapState::Missing),
            keymaps_sent: Mutex::new(0),
            pressed: Mutex::new(BTreeSet::new()),
        }
    }
}

/// The device id after `device`, wrapping back to the first virtual one.
fn successor(device: DeviceId) -> DeviceId {
    DeviceId(device.0.checked_add(1).unwrap_or(VIRTUAL_KEYBOARD_DEVICE.0))
}

/// A virtual keyboard sent a key: an `Event::Input` with `Origin::Virtual(Keyboard)`, from
/// that keyboard's own `device`.
pub fn on_vk_key(device: DeviceId, code: KeyCode, edge: Edge, at: Mono) -> Vec<Event> {
    vec![key_event(device, code, edge, at)]
}

/// Records `code`'s new `edge` in `pressed`. False when the key is not taken: a press past
/// [`MAX_PRESSED`] keys held, which is dropped.
fn track(pressed: &mut BTreeSet<KeyCode>, code: KeyCode, edge: Edge) -> Taken {
    match edge {
        Edge::Press if pressed.len() >= MAX_PRESSED && !pressed.contains(&code) => Taken::No,
        Edge::Press => {
            pressed.insert(code);
            Taken::Yes
        }
        Edge::Release => {
            pressed.remove(&code);
            Taken::Yes
        }
    }
}

/// Whether a key was taken or dropped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Taken {
    /// It was.
    Yes,
    /// It was dropped.
    No,
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
///
/// Only a memory file is read (`client_file`): a file a hostile process serves would block the
/// read, and with it the loop (CASW-09).
fn keymap_text(fd: OwnedFd, size: usize) -> Option<String> {
    if size == 0 || size > KEYMAP_LIMIT {
        return None;
    }
    let bytes = client_file::read_exactly(&File::from(fd), size)?;
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
    D: Dispatch<ZwpVirtualKeyboardV1, VirtualKeyboardData> + VirtualKeyboardHandler + 'static,
{
    fn request(
        &self,
        state: &mut D,
        client: &Client,
        _resource: &ZwpVirtualKeyboardManagerV1,
        request: zwp_virtual_keyboard_manager_v1::Request,
        _handle: &DisplayHandle,
        data_init: &mut DataInit<'_, D>,
    ) {
        if let zwp_virtual_keyboard_manager_v1::Request::CreateVirtualKeyboard { id, .. } = request
        {
            let device = state.virtual_keyboard_part().next_device();
            let start = match consent::admit(state, client, Power::Keys) {
                Admission::Go => Start::Open,
                Admission::Hold => Start::Waiting,
                Admission::Refuse => Start::Dead,
            };
            let keyboard = data_init.init(id, VirtualKeyboardData::new(device, start));
            let pushed = consent::key_of(client).map(|key| {
                state
                    .virtual_keyboard_part()
                    .keyboards
                    .push(key, keyboard.clone())
            });
            // A client past its cap of keyboards, and one nobody vouches for, gets a dead one.
            if !matches!(pushed, Some(Pushed::Kept)) {
                if let Some(data) = keyboard.data::<VirtualKeyboardData>() {
                    data.gate.close();
                }
            }
        }
    }
}

impl VirtualKeyboardData {
    /// Counts one more keymap request; a keyboard past [`Limit::Keymaps`] cuts its client.
    fn keymap_over_the_cap(&self, client: &Client, handle: &DisplayHandle) -> Standing {
        let mut sent = locked(&self.keymaps_sent);
        let standing = limits::admit(handle, client, Limit::Keymaps, *sent);
        *sent = sent.saturating_add(1);
        standing
    }

    /// Records `code`'s new edge and feeds the key to the model.
    fn key<D: VirtualKeyboardHandler>(&self, state: &mut D, code: KeyCode, edge: Edge) {
        if track(&mut locked(&self.pressed), code, edge) == Taken::No {
            return;
        }
        let at = state.virtual_keyboard_now();
        let events = on_vk_key(self.device, code, edge, at);
        state.virtual_keyboard_events(events);
    }
}

impl VirtualKeyboardData {
    /// Carries out one request that got past the consent gate.
    fn carry_out<D: VirtualKeyboardHandler>(
        &self,
        state: &mut D,
        client: &Client,
        resource: &ZwpVirtualKeyboardV1,
        request: zwp_virtual_keyboard_v1::Request,
        handle: &DisplayHandle,
    ) {
        let keymap = *locked(&self.keymap);
        match request {
            zwp_virtual_keyboard_v1::Request::Keymap { format, fd, size } => {
                if self.keymap_over_the_cap(client, handle) == Standing::Over {
                    return;
                }
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

    /// The person allowed the power: the gate opens and what it kept is carried out in order.
    fn allow<D: VirtualKeyboardHandler>(&self, state: &mut D, resource: &ZwpVirtualKeyboardV1) {
        let Some(client) = resource.client() else {
            return;
        };
        let display = state.virtual_keyboard_part().display.clone();
        for request in self.gate.open() {
            self.carry_out(state, &client, resource, request, &display);
        }
    }

    /// The power was refused or taken back: the gate closes for good, and the keys the
    /// keyboard still held down are released.
    fn end<D: VirtualKeyboardHandler>(&self, state: &mut D) {
        self.gate.close();
        let at = state.virtual_keyboard_now();
        let held = std::mem::take(&mut *locked(&self.pressed));
        let events = releases(&held, self.device, at);
        if !events.is_empty() {
            state.virtual_keyboard_events(events);
        }
    }
}

/// The answer to a question about `client`'s keys came out: its keyboards that wait are
/// let through ([`PowerOutcome::Allowed`]) or ended; a power taken back ends the ones in use.
pub fn on_resolved<D: VirtualKeyboardHandler>(
    state: &mut D,
    client: ClientKey,
    outcome: PowerOutcome,
) {
    let keyboards: Vec<ZwpVirtualKeyboardV1> =
        state.virtual_keyboard_part().keyboards.of(client).to_vec();
    for keyboard in &keyboards {
        let Some(data) = keyboard.data::<VirtualKeyboardData>() else {
            continue;
        };
        match outcome {
            PowerOutcome::Allowed => data.allow(state, keyboard),
            PowerOutcome::Denied | PowerOutcome::Revoked => data.end(state),
        }
    }
    if outcome != PowerOutcome::Allowed {
        state.virtual_keyboard_part().keyboards.take(client);
    }
}

impl<D> Dispatch2<ZwpVirtualKeyboardV1, D> for VirtualKeyboardData
where
    D: VirtualKeyboardHandler,
{
    fn request(
        &self,
        state: &mut D,
        client: &Client,
        resource: &ZwpVirtualKeyboardV1,
        request: zwp_virtual_keyboard_v1::Request,
        handle: &DisplayHandle,
        _data_init: &mut DataInit<'_, D>,
    ) {
        if let Routed::Now(request) = self.gate.route(request) {
            self.carry_out(state, client, resource, request, handle);
        }
    }

    fn destroyed(&self, state: &mut D, _client: ClientId, _resource: &ZwpVirtualKeyboardV1) {
        let at = state.virtual_keyboard_now();
        let events = releases(&locked(&self.pressed), self.device, at);
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
    fn every_keyboard_gets_a_device_of_its_own() {
        let first = VIRTUAL_KEYBOARD_DEVICE;
        let (second, third) = (successor(first), successor(successor(first)));
        assert!(first != second && second != third && first != third);
        assert_eq!(successor(DeviceId(u32::MAX)), VIRTUAL_KEYBOARD_DEVICE);
        assert!(second.0 > first.0, "ids stay above the backend's own");
    }

    #[test]
    fn a_keyboard_holds_at_most_a_bounded_set_of_keys() {
        let mut held = BTreeSet::new();
        for code in 0..u32::try_from(MAX_PRESSED).expect("a small cap") {
            assert_eq!(track(&mut held, KeyCode(code), Edge::Press), Taken::Yes);
        }
        assert_eq!(track(&mut held, KeyCode(1000), Edge::Press), Taken::No);
        assert_eq!(held.len(), MAX_PRESSED);
        // A key already down is no new key, and a release always frees room.
        assert_eq!(track(&mut held, KeyCode(0), Edge::Press), Taken::Yes);
        assert_eq!(track(&mut held, KeyCode(0), Edge::Release), Taken::Yes);
        assert_eq!(track(&mut held, KeyCode(1000), Edge::Press), Taken::Yes);
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
