//! Injected lines become virtual input: the grammar's lines turn into `InputKind`s pushed on
//! virtual devices (plugged when a line first needs one) and delivered by the core loop at
//! once, so a script's order is the order the clients see.

use crate::{State, core_loop};
use casement_backend::{BackendKind, DeviceName, DeviceSpec};
use casement_core::DeviceId;
use casement_core::input::{
    AxisFrame, AxisSource, ButtonCode, DeviceClass, InputKind, KeyCode, Origin, PortalSessionId,
};
use casement_ipc::{DeviceVerb, InjectLine, InjectOrigin};
use casement_protocols::vocab::PressEdge;
use smithay::input::keyboard::{Keycode, Keysym, Layout, xkb};
use std::collections::HashMap;

/// The evdev code of the left Shift key.
const KEY_LEFTSHIFT: u32 = 42;

/// The shifted ASCII punctuation and what it is the shifted form of on a US layout.
const SHIFTED: &str = "!@#$%^&*()_+{}|:\"<>?~";
const UNSHIFTED: &str = "1234567890-=[]\\;',./`";

/// The virtual devices injected lines go through, and the origin the lines carry.
#[derive(Debug)]
pub struct Injector {
    origin: InjectOrigin,
    devices: HashMap<(DeviceClass, InjectOrigin), Vec<DeviceId>>,
}

impl Default for Injector {
    fn default() -> Injector {
        Injector {
            origin: InjectOrigin::Test,
            devices: HashMap::new(),
        }
    }
}

impl Injector {
    /// Carries out one line.
    pub fn line(&mut self, state: &mut State, line: &InjectLine) {
        match line {
            InjectLine::Origin(origin) => self.origin = *origin,
            InjectLine::Device(DeviceVerb::Plug(class)) => {
                self.plug(state, *class);
            }
            InjectLine::Device(DeviceVerb::Unplug(class)) => self.unplug(state, *class),
            InjectLine::Sleep(_) => {}
            InjectLine::Key(keysym, edge) => self.key(state, Keysym::new(keysym.0), *edge),
            InjectLine::Type(text) => {
                for c in text.chars() {
                    let sym = xkb::utf32_to_keysym(u32::from(c));
                    self.key(state, sym, PressEdge::Press);
                    self.key(state, sym, PressEdge::Release);
                }
            }
            other => {
                if let Some((class, kind)) = kind_of(other) {
                    self.push(state, class, kind);
                }
            }
        }
        core_loop::turn(state);
    }

    /// The origin lines carry now, for a device of `class`.
    fn origin_of(&self, class: DeviceClass) -> Origin {
        match self.origin {
            InjectOrigin::Physical => Origin::Physical(class),
            InjectOrigin::Remote => Origin::Remote(PortalSessionId(0)),
            InjectOrigin::Test => Origin::Test,
        }
    }

    fn plug(&mut self, state: &mut State, class: DeviceClass) -> Option<DeviceId> {
        let layout = state.core.model.settings().keyboard.layout.clone();
        let spec = DeviceSpec {
            class,
            origin: self.origin_of(class),
            layout,
            name: DeviceName(format!("injected {class:?}")),
        };
        let BackendKind::Headless(backend) = &mut state.backend else {
            return None;
        };
        let id = backend.input().plug(spec);
        self.devices
            .entry((class, self.origin))
            .or_default()
            .push(id);
        Some(id)
    }

    fn unplug(&mut self, state: &mut State, class: DeviceClass) {
        let last = self
            .devices
            .get_mut(&(class, self.origin))
            .and_then(Vec::pop);
        if let (Some(id), BackendKind::Headless(backend)) = (last, &mut state.backend) {
            backend.input().unplug(id);
        }
    }

    /// The device of `class` with the current origin, plugged if there is none.
    fn device(&mut self, state: &mut State, class: DeviceClass) -> Option<DeviceId> {
        let known = self
            .devices
            .get(&(class, self.origin))
            .and_then(|ids| ids.last().copied());
        known.or_else(|| self.plug(state, class))
    }

    fn push(&mut self, state: &mut State, class: DeviceClass, kind: InputKind) {
        let Some(device) = self.device(state, class) else {
            return;
        };
        let at = state.clock.now();
        if let BackendKind::Headless(backend) = &mut state.backend {
            backend.input().push(device, kind, at);
        }
    }

    /// A key by keysym: found on the keymap's first layout, with Shift held when it is the
    /// shifted form of a key (a capital, a US shifted punctuation mark). A keysym no key makes
    /// is dropped with a warning.
    fn key(&mut self, state: &mut State, sym: Keysym, edge: PressEdge) {
        let Some((code, shifted)) = keycode_for(state, sym) else {
            tracing::warn!(keysym = sym.raw(), "no key makes this keysym");
            return;
        };
        let shift = InputKind::Key {
            code: KeyCode(KEY_LEFTSHIFT),
            edge,
        };
        let key = InputKind::Key { code, edge };
        match (shifted, edge) {
            (false, _) => self.push(state, DeviceClass::Keyboard, key),
            (true, PressEdge::Press) => {
                self.push(state, DeviceClass::Keyboard, shift);
                self.push(state, DeviceClass::Keyboard, key);
            }
            (true, PressEdge::Release) => {
                self.push(state, DeviceClass::Keyboard, key);
                self.push(state, DeviceClass::Keyboard, shift);
            }
        }
    }
}

/// The evdev key that makes `sym`, and whether Shift must be held for it.
fn keycode_for(state: &mut State, sym: Keysym) -> Option<(KeyCode, bool)> {
    let (base, shifted) = unshift(sym);
    let keyboard = state.wl.seat.seat.get_keyboard()?;
    let raw = keyboard.with_xkb_state(state, |context| {
        let xkb = context.xkb().lock().ok()?;
        (8_u32..=255).find(|code| {
            xkb.raw_syms_for_key_in_layout(Keycode::new(*code), Layout(0))
                .contains(&base)
        })
    })?;
    Some((KeyCode(raw - 8), shifted))
}

/// `sym` as the unshifted keysym of its key, and whether Shift is needed to make it.
fn unshift(sym: Keysym) -> (Keysym, bool) {
    let Some(c) = sym.key_char() else {
        return (sym, false);
    };
    if c.is_ascii_uppercase() {
        return (Keysym::new(u32::from(c.to_ascii_lowercase())), true);
    }
    match SHIFTED.find(c).and_then(|at| UNSHIFTED.chars().nth(at)) {
        Some(base) => (Keysym::new(u32::from(base)), true),
        None => (sym, false),
    }
}

/// The device class a line goes through and what it pushes (lines that are not input give
/// none).
fn kind_of(line: &InjectLine) -> Option<(DeviceClass, InputKind)> {
    let finger = |px, stopped| AxisFrame {
        source: AxisSource::Finger,
        v120: (0, 0),
        px,
        stopped,
    };
    let zero = casement_core::geometry::SubPoint {
        x: casement_core::geometry::SubPx(0),
        y: casement_core::geometry::SubPx(0),
    };
    Some(match line {
        InjectLine::Move(at) => (DeviceClass::Pointer, InputKind::MotionTo { at: *at }),
        InjectLine::Press(button) => (DeviceClass::Pointer, button_kind(*button, PressEdge::Press)),
        InjectLine::Release(button) => (
            DeviceClass::Pointer,
            button_kind(*button, PressEdge::Release),
        ),
        InjectLine::Axis(frame) => (DeviceClass::Pointer, InputKind::Axis(*frame)),
        InjectLine::ScrollFinger(px) => {
            (DeviceClass::Touchpad, InputKind::Axis(finger(*px, false)))
        }
        InjectLine::AxisStop => (DeviceClass::Touchpad, InputKind::Axis(finger(zero, true))),
        InjectLine::Swipe(step) => (DeviceClass::Touchpad, InputKind::Swipe(*step)),
        InjectLine::Pinch(step) => (DeviceClass::Touchpad, InputKind::Pinch(*step)),
        InjectLine::Hold(step) => (DeviceClass::Touchpad, InputKind::Hold(*step)),
        InjectLine::Touch(step) => (DeviceClass::Touch, InputKind::Touch(*step)),
        InjectLine::Key(..)
        | InjectLine::Type(_)
        | InjectLine::Device(_)
        | InjectLine::Origin(_)
        | InjectLine::Sleep(_) => return None,
    })
}

fn button_kind(button: ButtonCode, edge: PressEdge) -> InputKind {
    InputKind::Button { button, edge }
}
