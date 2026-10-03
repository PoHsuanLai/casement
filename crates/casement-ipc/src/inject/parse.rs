//! The inject line parser: words in, [`InjectLine`] out. Every refusal is
//! [`IpcError::BadLine`] naming the whole line.

use super::{DeviceVerb, InjectLine, InjectOrigin};
use crate::error::IpcError;
use casement_core::geometry::{LogicalPoint, SubPoint, SubPx};
use casement_core::input::{
    AxisFrame, AxisSource, AxisStop, ButtonCode, DeviceClass, Edge, GestureEnd, GestureStep,
    Keysym, PinchDelta, TouchStep,
};
use std::time::Duration;

/// One detent is `value120` 120; fake-input.sh's `axis vertical 15` is one detent of 15 px, so a
/// value counts 8 of the 120ths per pixel.
const V120_PER_PIXEL: i32 = 8;
/// The longest `sleep`: an hour of session time. A typo in a script should not park the jail.
const MAX_SLEEP_MS: u64 = 3_600_000;
/// A sub-pixel is 1/120 of a logical pixel (`SubPx`).
const SUB_PER_PIXEL: i32 = 120;
/// The largest magnitude `number` takes: scaled by 120 it still fits an `i32`.
const MAX_NUMBER: f64 = 17_000_000.0;
/// A scroll frame that names no pixel distance.
const NO_PIXELS: SubPoint = SubPoint {
    x: SubPx(0),
    y: SubPx(0),
};
/// The evdev button codes a number may name (`BTN_MISC` to the end of `BTN_*`).
const BUTTON_RANGE: std::ops::Range<u32> = 0x100..0x300;

/// The line `line` says.
pub fn parse_inject(line: &str) -> Result<InjectLine, IpcError> {
    let bad = || IpcError::BadLine(line.to_owned());
    let trimmed = line.trim();
    let (verb, rest) = trimmed
        .split_once(char::is_whitespace)
        .unwrap_or((trimmed, ""));
    let rest = rest.trim();
    let words: Vec<&str> = rest.split_whitespace().collect();
    let parsed = match verb {
        "move" => point(&words).map(InjectLine::Move),
        "press" => button(&words).map(InjectLine::Press),
        "release" => button(&words).map(InjectLine::Release),
        "key" => key(&words),
        "type" => text(rest).map(InjectLine::Type),
        "axis" => axis(&words),
        "scroll-finger" => sub_point(&words).map(InjectLine::ScrollFinger),
        "scroll" => words
            .split_first()
            .filter(|(kind, _)| **kind == "finger")
            .and_then(|(_, tail)| sub_point(tail))
            .map(InjectLine::ScrollFinger),
        "axis-stop" => words.is_empty().then_some(InjectLine::AxisStop),
        "swipe" => gesture(&words, sub_point).map(InjectLine::Swipe),
        "pinch" => gesture(&words, pinch).map(InjectLine::Pinch),
        "hold" => gesture(&words, |tail| tail.is_empty().then_some(())).map(InjectLine::Hold),
        "touch" => touch(&words).map(InjectLine::Touch),
        "device" => device(&words).map(InjectLine::Device),
        "origin" => origin(&words).map(InjectLine::Origin),
        "sleep" => sleep(&words).map(InjectLine::Sleep),
        _ => None,
    };
    parsed.ok_or_else(bad)
}

fn int<T: std::str::FromStr>(word: &str) -> Option<T> {
    word.parse().ok()
}

/// A finite number that fits a scaled `i32` with room to spare: fake-input.sh read pixel values
/// as floats, so `100.5` and `15.0` are accepted.
fn number(word: &str) -> Option<f64> {
    let value: f64 = word.parse().ok()?;
    (value.is_finite() && value.abs() <= MAX_NUMBER).then_some(value)
}

/// A logical-pixel position: a number rounded to the nearest pixel (halves away from zero).
fn pixels(word: &str) -> Option<i32> {
    number(word).map(|value| value.round() as i32)
}

fn point(words: &[&str]) -> Option<LogicalPoint> {
    match words {
        [x, y] => Some(LogicalPoint {
            x: pixels(x)?,
            y: pixels(y)?,
        }),
        _ => None,
    }
}

fn sub_point(words: &[&str]) -> Option<SubPoint> {
    match words {
        [x, y] => Some(SubPoint {
            x: SubPx(int(x)?),
            y: SubPx(int(y)?),
        }),
        _ => None,
    }
}

/// `left`, `right`, `middle`, the other `BTN_*` names, or an evdev code (`272`, `0x110`).
fn button(words: &[&str]) -> Option<ButtonCode> {
    let [word] = words else { return None };
    let named = [
        ("left", 0x110),
        ("right", 0x111),
        ("middle", 0x112),
        ("side", 0x113),
        ("extra", 0x114),
        ("forward", 0x115),
        ("back", 0x116),
        ("task", 0x117),
    ]
    .into_iter()
    .find_map(|(name, code)| (name == *word).then_some(code));
    let numeric = || match word.strip_prefix("0x") {
        Some(hex) => u32::from_str_radix(hex, 16).ok(),
        None => int(word),
    };
    named
        .or_else(numeric)
        .filter(|code| BUTTON_RANGE.contains(code))
        .map(ButtonCode)
}

fn edge(word: &str) -> Option<Edge> {
    match word {
        "down" => Some(Edge::Press),
        "up" => Some(Edge::Release),
        _ => None,
    }
}

fn key(words: &[&str]) -> Option<InjectLine> {
    match words {
        [name, edge_word] => Some(InjectLine::Key(keysym(name)?, edge(edge_word)?)),
        _ => None,
    }
}

/// Our own names and codes first (`a`, `Return`, `U20AC`, `0xff09`), then every name
/// libxkbcommon knows (`Shift_L`, `Super_L`, `KP_Enter`).
fn keysym(name: &str) -> Option<Keysym> {
    Keysym::named(name).or_else(|| {
        let raw = xkbcommon::xkb::keysym_from_name(name, xkbcommon::xkb::KEYSYM_NO_FLAGS).raw();
        (raw != xkbcommon::xkb::keysyms::KEY_NoSymbol).then_some(Keysym(raw))
    })
}

/// What `type` types: the rest of the line as written, or, when it opens and closes with a quote,
/// what is between them (fake-input.sh's reading: no escapes, so `type abc` and `type "abc"` agree;
/// a newline is `key Return down`). Nothing at all is not a command.
fn text(rest: &str) -> Option<String> {
    let quoted = rest
        .strip_prefix('"')
        .and_then(|inner| inner.strip_suffix('"'));
    (!rest.is_empty()).then(|| quoted.unwrap_or(rest).to_owned())
}

fn axis(words: &[&str]) -> Option<InjectLine> {
    let frame = match words {
        // fake-input.sh: one wheel event, 15 is one detent.
        ["vertical", value] => wheel(0.0, number(value)?),
        ["horizontal", value] => wheel(number(value)?, 0.0),
        [source, x, y] => axis_frame(source, int(x)?, int(y)?, NO_PIXELS)?,
        [source, x, y, px, py] => axis_frame(source, int(x)?, int(y)?, sub_point(&[px, py])?)?,
        _ => return None,
    };
    Some(InjectLine::Axis(frame))
}

fn wheel(horizontal: f64, vertical: f64) -> AxisFrame {
    let scaled = |value: f64, per_pixel: i32| (value * f64::from(per_pixel)).round() as i32;
    AxisFrame {
        source: AxisSource::Wheel,
        v120: (
            scaled(horizontal, V120_PER_PIXEL),
            scaled(vertical, V120_PER_PIXEL),
        ),
        px: SubPoint {
            x: SubPx(scaled(horizontal, SUB_PER_PIXEL)),
            y: SubPx(scaled(vertical, SUB_PER_PIXEL)),
        },
        stop: AxisStop::Running,
    }
}

fn axis_frame(source: &str, x: i32, y: i32, px: SubPoint) -> Option<AxisFrame> {
    let source = match source {
        "wheel" => AxisSource::Wheel,
        "finger" => AxisSource::Finger,
        "continuous" => AxisSource::Continuous,
        "tilt" => AxisSource::WheelTilt,
        _ => return None,
    };
    Some(AxisFrame {
        source,
        v120: (x, y),
        px,
        stop: AxisStop::Running,
    })
}

/// `begin N`, an update (with or without the word `update`), `end` or `cancel`.
fn gesture<D>(words: &[&str], update: impl Fn(&[&str]) -> Option<D>) -> Option<GestureStep<D>> {
    match words {
        ["begin", fingers] => Some(GestureStep::Begin {
            fingers: int(fingers).filter(|n| *n > 0)?,
        }),
        ["end"] => Some(GestureStep::End(GestureEnd::Lifted)),
        ["cancel"] => Some(GestureStep::End(GestureEnd::Cancelled)),
        ["update", tail @ ..] => update(tail).map(GestureStep::Update),
        tail => update(tail).map(GestureStep::Update),
    }
}

/// `SCALE ROT DX DY`: the scale in thousandths, the rotation in thousandths of a degree.
fn pinch(words: &[&str]) -> Option<PinchDelta> {
    let [scale, rotation, x, y] = words else {
        return None;
    };
    Some(PinchDelta {
        // `Fraction(u16)` is ds-core's; casement-ipc does not name ds-core, so the field's own
        // type is read from the number (interface ask in the P0-T-dev report: a ds-core dependency).
        scale: serde_json::from_value(serde_json::Value::from(int::<u16>(scale)?)).ok()?,
        rotation_millideg: int(rotation)?,
        delta: sub_point(&[x, y])?,
    })
}

fn touch(words: &[&str]) -> Option<TouchStep> {
    match words {
        ["down", slot, x, y] => Some(TouchStep::Down {
            slot: int(slot)?,
            at: point(&[x, y])?,
        }),
        ["move" | "motion", slot, x, y] => Some(TouchStep::Motion {
            slot: int(slot)?,
            at: point(&[x, y])?,
        }),
        ["up", slot] => Some(TouchStep::Up { slot: int(slot)? }),
        ["frame"] => Some(TouchStep::Frame),
        ["cancel"] => Some(TouchStep::Cancel),
        _ => None,
    }
}

fn device(words: &[&str]) -> Option<DeviceVerb> {
    let [verb, class] = words else { return None };
    let class = match *class {
        "keyboard" => DeviceClass::Keyboard,
        "pointer" => DeviceClass::Pointer,
        "touchpad" => DeviceClass::Touchpad,
        "touch" => DeviceClass::Touch,
        "tablet" => DeviceClass::Tablet,
        "switch" => DeviceClass::Switch,
        _ => return None,
    };
    match *verb {
        "plug" => Some(DeviceVerb::Plug(class)),
        "unplug" => Some(DeviceVerb::Unplug(class)),
        _ => None,
    }
}

fn origin(words: &[&str]) -> Option<InjectOrigin> {
    match words {
        ["physical"] => Some(InjectOrigin::Physical),
        ["remote"] => Some(InjectOrigin::Remote),
        ["test"] => Some(InjectOrigin::Test),
        _ => None,
    }
}

fn sleep(words: &[&str]) -> Option<Duration> {
    let [ms] = words else { return None };
    int::<u64>(ms)
        .filter(|ms| *ms <= MAX_SLEEP_MS)
        .map(Duration::from_millis)
}
