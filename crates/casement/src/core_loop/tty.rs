//! The tty backend's turn: what the seat, the devices and the input reported since the last
//! turn, told to the model by the same paths the headless backend uses; the frame it presents;
//! the VT chord.

use super::{connector, feed};
use crate::exec;
use crate::state::State;
use casement_backend::BackendError;
use casement_backend::BackendKind;
use casement_backend::headless::FrameSource;
use casement_backend::input::{
    Applied, FedDevice, FedInput, PollContext, device_settings, scaled_scroll,
};
use casement_backend::tty::{
    Health, PresentOutcome, SessionNote, TtyBackend, TtyNote, VtHeld, VtNumber, Watch, vt_of_chord,
};
use casement_core::geometry::{LogicalPoint, LogicalRect, LogicalSize};
use casement_core::input::{DeviceClass, Edge, InputEvent, Keysym, Origin};
use casement_core::output::OutputConfig;
use casement_core::settings::PointerSettings;
use casement_core::time::Mono;
use casement_core::{Effect, Event, OutputKey, OutputName};
use casement_wayland::image_capture::CaptureBook;
use casement_wayland::screencopy::ScreencopyPart;
use smithay::input::keyboard::Keysym as XkbKeysym;
use smithay::input::keyboard::ModifiersState;

/// Drains the tty backend: connector events go down the connector path (plug, change,
/// unplug); a pause needs nothing here (the backend presents no frame while the seat is lent);
/// a return re-applies every output's configuration, which re-modesets the displays; then the
/// input the seat's devices reported is told to the model and delivered. How many things the
/// backend reported. A backend that is not tty reports nothing.
pub(super) fn drain(state: &mut State) -> usize {
    let BackendKind::Tty(tty) = &mut state.backend else {
        return 0;
    };
    let notes = tty.dispatch();
    let count = notes.len();
    for note in notes {
        match note {
            TtyNote::Connector(event) => connector(state, event),
            TtyNote::Session(SessionNote::Activated) => remodeset(state),
            TtyNote::Reinit(name) => reapply(state, &name),
            _ => {}
        }
    }
    count + input(state)
}

/// Lights `name` again from the model's configuration: its frames stopped for good, and the
/// backend dropped its surface.
fn reapply(state: &mut State, name: &OutputName) {
    let found = state
        .core
        .model
        .outputs()
        .0
        .iter()
        .find(|(_, plugged)| plugged.facts.name == *name)
        .map(|(output, plugged)| (output.clone(), plugged.config));
    if let Some((output, config)) = found {
        exec::exec(state, vec![Effect::ApplyOutput { output, config }]);
    }
}

/// Whether the display shows the session: frozen when every output that should show frames has
/// shown none for long (the watchdog is then not fed). Any other backend is taken as alive.
pub(crate) fn display_health(state: &State) -> Health {
    match &state.backend {
        BackendKind::Tty(tty) => tty.health(state.clock.now()),
        _ => Health::Alive,
    }
}

/// Puts the model's configuration of every output back on the backend.
fn remodeset(state: &mut State) {
    let effects: Vec<Effect> = state
        .core
        .model
        .outputs()
        .0
        .iter()
        .map(|(output, plugged)| Effect::ApplyOutput {
            output: output.clone(),
            config: plugged.config,
        })
        .collect();
    exec::exec(state, effects);
}

/// The input the seat's devices reported: a device that appeared is configured with the stored
/// settings, an event is told to the model like any other input. How many things there were.
fn input(state: &mut State) -> usize {
    let context = PollContext::new(state.clock.now(), desktop_extent(state));
    let BackendKind::Tty(tty) = &mut state.backend else {
        return 0;
    };
    let fed = tty.poll_input(&context);
    let count = fed.len();
    for item in fed {
        match item {
            FedInput::Appeared(device) => configure(state, &device),
            FedInput::Event(event) => {
                let event = with_scroll_factor(state, event);
                feed(state, vec![Event::Input(event)]);
            }
            _ => {}
        }
    }
    count
}

/// A scroll of a mouse or a touchpad, scaled by the stored scroll factor of its profile
/// (libinput has no such setting).
pub(super) fn with_scroll_factor(state: &State, event: InputEvent) -> InputEvent {
    let pointer = &state.settings().pointer;
    let profile = match event.origin {
        Origin::Physical(DeviceClass::Touchpad) => pointer.touchpad,
        Origin::Physical(DeviceClass::Pointer) => pointer.mouse,
        _ => return event,
    };
    InputEvent {
        kind: scaled_scroll(event.kind, profile.scroll_factor),
        ..event
    }
}

/// Applies the stored pointer settings to a pointing device that appeared (a keyboard, a
/// touchscreen or a switch has none).
fn configure(state: &mut State, device: &FedDevice) {
    if !matches!(device.class, DeviceClass::Pointer | DeviceClass::Touchpad) {
        return;
    }
    let settings = device_settings(&state.settings().pointer, device.class);
    let BackendKind::Tty(tty) = &mut state.backend else {
        return;
    };
    match tty.configure_device(device.id, &settings) {
        Applied::All => {}
        Applied::Partly(skipped) => {
            tracing::debug!(
                device = device.name.0,
                ?skipped,
                "a device took the settings in part"
            );
        }
        _ => tracing::debug!(device = device.name.0, "a device did not take the settings"),
    }
}

/// Applies `pointer` to the pointing devices already present that it no longer matches, so a
/// changed setting takes effect at once, not after a replug. Nothing on a seat with no input.
pub(crate) fn reconfigure_pointers(state: &mut State, pointer: &PointerSettings) {
    let BackendKind::Tty(tty) = &mut state.backend else {
        return;
    };
    for (device, applied) in tty.reconfigure_pointers(pointer) {
        match applied {
            Applied::All => {}
            Applied::Partly(skipped) => {
                tracing::debug!(
                    device = device.0,
                    ?skipped,
                    "a device took the new settings in part"
                );
            }
            _ => tracing::debug!(device = device.0, "a device did not take the new settings"),
        }
    }
}

/// The union of the outputs in global logical coordinates (an absolute device covers it).
pub(crate) fn desktop_extent(state: &State) -> LogicalRect {
    let rects = state
        .core
        .model
        .outputs()
        .0
        .values()
        .map(|output| output.config.logical_rect());
    let (mut left, mut top, mut right, mut bottom) = (i64::MAX, i64::MAX, i64::MIN, i64::MIN);
    let mut any = false;
    for rect in rects {
        any = true;
        left = left.min(i64::from(rect.origin.x));
        top = top.min(i64::from(rect.origin.y));
        right = right.max(i64::from(rect.origin.x) + i64::from(rect.size.w));
        bottom = bottom.max(i64::from(rect.origin.y) + i64::from(rect.size.h));
    }
    if !any {
        return LogicalRect::new(LogicalPoint { x: 0, y: 0 }, LogicalSize::ZERO);
    }
    let clamp = |n: i64| i32::try_from(n).unwrap_or(if n < 0 { i32::MIN } else { i32::MAX });
    let span = |from: i64, to: i64| u32::try_from(to - from).unwrap_or(u32::MAX);
    LogicalRect::new(
        LogicalPoint {
            x: clamp(left),
            y: clamp(top),
        },
        LogicalSize {
            w: span(left, right),
            h: span(top, bottom),
        },
    )
}

/// Whether a capture is open on `output` (an `ext_image_copy_capture` session, or a
/// `zwlr_screencopy` copy waiting for its frame): then the frame shown is also kept as pixels.
pub(super) fn capture_watch(
    book: &CaptureBook,
    screencopy: &ScreencopyPart,
    output: &OutputKey,
) -> Watch {
    match book.sessions.values().any(|watched| watched == output) || screencopy.wants(output) {
        true => Watch::On,
        false => Watch::Off,
    }
}

/// A frame of `source` on `name`, with its capture shadow kept per `capture`.
pub(super) fn present(
    tty: &mut TtyBackend,
    name: &OutputName,
    config: &OutputConfig,
    capture: Watch,
    now: Mono,
    source: &dyn FrameSource,
) -> Result<PresentOutcome, BackendError> {
    tty.watch(name, config, capture)?;
    tty.present(name, source, now)
}

/// A key press whose symbol names a VT (`Ctrl+Alt+Fn` is `XF86Switch_VT_n` after xkb) asks the
/// seat's session to switch there, before any binding or client sees the key. The VT the key
/// names, press or release (the release is swallowed too); none for any other key and on a
/// backend that has no VTs.
pub(super) fn vt_chord(
    state: &mut State,
    mods: &ModifiersState,
    syms: &[XkbKeysym],
    raw: &[XkbKeysym],
    edge: Edge,
) -> Option<VtNumber> {
    let BackendKind::Tty(tty) = &mut state.backend else {
        return None;
    };
    let held = match (mods.ctrl, mods.alt) {
        (true, true) => VtHeld::CtrlAlt,
        _ => VtHeld::Other,
    };
    let own = |syms: &[XkbKeysym]| syms.iter().map(|sym| Keysym(sym.raw())).collect::<Vec<_>>();
    let vt = vt_of_chord(held, &own(syms), &own(raw))?;
    if edge == Edge::Press
        && let Err(error) = tty.switch_to(vt)
    {
        tracing::warn!(vt = vt.0, %error, "the VT switch was refused");
    }
    Some(vt)
}
