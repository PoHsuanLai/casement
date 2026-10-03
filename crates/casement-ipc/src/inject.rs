//! The inject grammar (11 §4.1): one command per line, each answered `ok <line>` once it
//! reached the compositor. A superset of shell-host's `fake-input.sh`:
//!
//! ```text
//! move X Y                          absolute pointer position (global logical px; decimals round)
//! press BUTTON    release BUTTON    left|right|middle|side|extra|forward|back|task, or an evdev code
//! key KEYSYM down|up                xkb keysym name: Return, a, Shift_L, Super_L, F5, 0xff09, U20AC
//! type TEXT | type "TEXT"           each character, with Shift where the keymap needs it; the text is
//!                                   the rest of the line as written (no escapes: a newline is
//!                                   `key Return down` / `key Return up`)
//! axis vertical|horizontal VALUE    one wheel event of VALUE logical px; 15 is one detent (value120 120)
//! axis wheel|finger|continuous|tilt V120X V120Y [PXX PXY]   a frame with its source named; the
//!                                   pixel distance (sub-pixels, 1/120 px) is 0 when left out
//! scroll-finger DX DY               a touchpad scroll in sub-pixels (`scroll finger DX DY` too)
//! axis-stop                         the fingers lifted
//! swipe begin N | swipe [update] DX DY | swipe end|cancel          (DX DY in sub-pixels)
//! pinch begin N | pinch [update] SCALE ROT DX DY | pinch end|cancel (thousandths, millidegrees, sub-px)
//! hold begin N | hold end|cancel
//! touch down SLOT X Y | touch move|motion SLOT X Y | touch up SLOT | touch frame | touch cancel
//! device plug|unplug keyboard|pointer|touchpad|touch|tablet|switch
//! origin physical|remote|test       the origin of the lines that follow (debug-inject only)
//! sleep MS                          session time (the stepped clock advances; never a wall sleep)
//! ```

use casement_core::geometry::{LogicalPoint, SubPoint};
use casement_core::input::{
    AxisFrame, ButtonCode, DeviceClass, Edge, GestureStep, Keysym, PinchDelta, TouchStep,
};
use std::time::Duration;

/// One inject line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InjectLine {
    /// `move X Y`.
    Move(LogicalPoint),
    /// `press BUTTON`.
    Press(ButtonCode),
    /// `release BUTTON`.
    Release(ButtonCode),
    /// `key KEYSYM down|up`.
    Key(Keysym, Edge),
    /// `type "text"`.
    Type(String),
    /// `axis ...`: one scroll frame.
    Axis(AxisFrame),
    /// `scroll-finger DX DY`.
    ScrollFinger(SubPoint),
    /// `axis-stop`.
    AxisStop,
    /// `swipe ...`.
    Swipe(GestureStep<SubPoint>),
    /// `pinch ...`.
    Pinch(GestureStep<PinchDelta>),
    /// `hold ...`.
    Hold(GestureStep<()>),
    /// `touch ...`.
    Touch(TouchStep),
    /// `device plug|unplug CLASS`.
    Device(DeviceVerb),
    /// `origin ORIGIN`.
    Origin(InjectOrigin),
    /// `sleep MS`.
    Sleep(Duration),
}

/// The origins an injected line may carry (Agent input enters only through a lease's EIS
/// socket; Feed only through palmrest's socket).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InjectOrigin {
    /// As if from a kernel device.
    Physical,
    /// As if from a RemoteDesktop session.
    Remote,
    /// A test.
    Test,
}

/// Plugging or unplugging a virtual device.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DeviceVerb {
    /// Plug one of this class.
    Plug(DeviceClass),
    /// Unplug the last one of this class.
    Unplug(DeviceClass),
}

mod parse;

pub use parse::parse_inject;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::IpcError;
    use casement_core::geometry::SubPx;
    use casement_core::input::{AxisSource, AxisStop, GestureEnd};

    #[test]
    fn every_fake_input_line_parses() {
        // shell-host's `fake-input.sh` grammar, line for line (the superset's floor).
        let rows: [(&str, InjectLine); 7] = [
            (
                "move 300 200",
                InjectLine::Move(LogicalPoint { x: 300, y: 200 }),
            ),
            ("press left", InjectLine::Press(ButtonCode(0x110))),
            ("release right", InjectLine::Release(ButtonCode(0x111))),
            (
                "key Return down",
                InjectLine::Key(Keysym(0xff0d), Edge::Press),
            ),
            ("key a up", InjectLine::Key(Keysym(0x61), Edge::Release)),
            ("type \"héllo\"", InjectLine::Type("héllo".to_owned())),
            ("sleep 40", InjectLine::Sleep(Duration::from_millis(40))),
        ];
        for (line, expected) in rows {
            assert_eq!(parse_inject(line).expect(line), expected, "{line}");
        }
    }

    #[test]
    fn a_wheel_detent_carries_its_source_and_value120() {
        // fake-input.sh's `axis vertical 15` is one detent (SP1 B1: the nested path lost it).
        let detent = AxisFrame {
            source: AxisSource::Wheel,
            v120: (0, 120),
            px: SubPoint {
                x: SubPx(0),
                y: SubPx(15 * 120),
            },
            stop: AxisStop::Running,
        };
        assert_eq!(
            parse_inject("axis vertical 15").expect("axis"),
            InjectLine::Axis(detent)
        );
        assert_eq!(
            parse_inject("axis wheel 0 120").expect("axis"),
            InjectLine::Axis(AxisFrame {
                px: SubPoint {
                    x: SubPx(0),
                    y: SubPx(0)
                },
                ..detent
            })
        );
    }

    #[test]
    fn gestures_touch_devices_and_origins_parse() {
        assert_eq!(
            parse_inject("swipe begin 3").expect("swipe"),
            InjectLine::Swipe(GestureStep::Begin { fingers: 3 })
        );
        assert_eq!(
            parse_inject("swipe end").expect("swipe"),
            InjectLine::Swipe(GestureStep::End(GestureEnd::Lifted))
        );
        assert_eq!(
            parse_inject("hold cancel").expect("hold"),
            InjectLine::Hold(GestureStep::End(GestureEnd::Cancelled))
        );
        assert_eq!(
            parse_inject("touch up 2").expect("touch"),
            InjectLine::Touch(TouchStep::Up { slot: 2 })
        );
        assert_eq!(
            parse_inject("touch frame").expect("touch"),
            InjectLine::Touch(TouchStep::Frame)
        );
        assert_eq!(
            parse_inject("device plug touchpad").expect("device"),
            InjectLine::Device(DeviceVerb::Plug(DeviceClass::Touchpad))
        );
        assert_eq!(
            parse_inject("origin remote").expect("origin"),
            InjectLine::Origin(InjectOrigin::Remote)
        );
        assert_eq!(
            parse_inject("axis-stop").expect("stop"),
            InjectLine::AxisStop
        );
    }

    #[test]
    fn malformed_lines_are_refused() {
        for line in [
            "",
            "move 1",
            "press thumb-9",
            "key NotAKeysym down",
            "origin agent",
            "sleep -1",
            "type",
        ] {
            assert!(
                matches!(parse_inject(line), Err(IpcError::BadLine(_))),
                "{line}"
            );
        }
    }

    fn sub(x: i32, y: i32) -> SubPoint {
        SubPoint {
            x: SubPx(x),
            y: SubPx(y),
        }
    }

    /// Every command shell-host's eight acceptance scripts send through `fake-input.sh`, with the
    /// shell variables filled in, and the cases of fake-input's own parser test: the grammar's floor.
    #[test]
    fn shell_hosts_fake_input_script_replays() {
        let lines = [
            "move 100 60",
            "sleep 100",
            "press left",
            "sleep 60",
            "move 140 60",
            "release left",
            "move 900 500",
            "move 901 500",
            "move 160 1020",
            "move 161 1020",
            "move 640 400",
            "key Return down",
            "key Return up",
            "key Super_L down",
            "key Super_L up",
            "key Caps_Lock down",
            "key Caps_Lock up",
            "key s down",
            "key u up",
            "key 3 down",
            "key c up",
            "key l down",
            "key BackSpace down",
            "key BackSpace up",
            "type abc",
            "type letmein",
            "type \"Hi there\"",
            "axis vertical 15",
            "axis horizontal -7.5",
            "move 10 20.5",
            "release right",
            "sleep 50",
            "  move 1 2  ",
        ];
        for line in lines {
            assert!(parse_inject(line).is_ok(), "{line}");
        }
        // fake-input refuses these too.
        for line in ["press thumb", "key a sideways", "jump", "axis diagonal 1"] {
            assert!(parse_inject(line).is_err(), "{line}");
        }
    }

    #[test]
    fn buttons_are_named_or_evdev_codes() {
        for (text, code) in [
            ("left", 0x110),
            ("middle", 0x112),
            ("side", 0x113),
            ("back", 0x116),
            ("272", 0x110),
            ("0x111", 0x111),
        ] {
            assert_eq!(
                parse_inject(&format!("press {text}")).expect(text),
                InjectLine::Press(ButtonCode(code)),
                "{text}"
            );
        }
        for text in ["0", "0x10", "0x300", "left right", "Left"] {
            assert!(parse_inject(&format!("release {text}")).is_err(), "{text}");
        }
    }

    #[test]
    fn keys_take_xkb_names_and_codes() {
        for (name, keysym) in [
            ("Shift_L", 0xffe1),
            ("Super_L", 0xffeb),
            ("F5", 0xffc2),
            ("space", 0x20),
            ("0xff09", 0xff09),
            ("U20AC", 0x0100_20ac),
        ] {
            assert_eq!(
                parse_inject(&format!("key {name} down")).expect(name),
                InjectLine::Key(Keysym(keysym), Edge::Press),
                "{name}"
            );
        }
        assert!(parse_inject("key a sideways").is_err());
        assert!(parse_inject("key a").is_err());
    }

    #[test]
    fn typed_text_is_the_rest_of_the_line_with_or_without_quotes() {
        for (line, text) in [
            ("type abc", "abc"),
            ("type letmein", "letmein"),
            (r#"type "abc""#, "abc"),
            (r#"type "two  spaces""#, "two  spaces"),
            ("type two words", "two words"),
            (r#"type "open"#, r#""open"#),
            (r#"type a\nb"#, r#"a\nb"#),
            ("type \"\"", ""),
        ] {
            assert_eq!(
                parse_inject(line).expect(line),
                InjectLine::Type(text.to_owned()),
                "{line}"
            );
        }
        assert!(parse_inject("type").is_err());
        assert!(parse_inject("type   ").is_err());
    }

    #[test]
    fn positions_and_wheel_values_may_be_decimals_like_fake_input_read_them() {
        assert_eq!(
            parse_inject("move 100.5 -20.4").expect("move"),
            InjectLine::Move(LogicalPoint { x: 101, y: -20 })
        );
        match parse_inject("axis vertical 7.5").expect("axis") {
            InjectLine::Axis(frame) => {
                assert_eq!(frame.v120, (0, 60));
                assert_eq!(frame.px, sub(0, 900));
            }
            other => panic!("{other:?}"),
        }
        for line in [
            "move nan 1",
            "move 1e12 1",
            "axis vertical inf",
            "axis sideways 3",
        ] {
            assert!(parse_inject(line).is_err(), "{line}");
        }
    }

    #[test]
    fn scroll_frames_name_their_source_and_distance() {
        let frame = |source, v120, px| {
            InjectLine::Axis(AxisFrame {
                source,
                v120,
                px,
                stop: AxisStop::Running,
            })
        };
        assert_eq!(
            parse_inject("axis horizontal -15").expect("axis"),
            frame(AxisSource::Wheel, (-120, 0), sub(-1800, 0))
        );
        assert_eq!(
            parse_inject("axis finger 0 0 30 -240").expect("axis"),
            frame(AxisSource::Finger, (0, 0), sub(30, -240))
        );
        assert_eq!(
            parse_inject("axis tilt 120 0").expect("axis"),
            frame(AxisSource::WheelTilt, (120, 0), sub(0, 0))
        );
        assert_eq!(
            parse_inject("scroll-finger 4 -8").expect("scroll"),
            InjectLine::ScrollFinger(sub(4, -8))
        );
        assert_eq!(
            parse_inject("scroll finger 4 -8").expect("scroll"),
            InjectLine::ScrollFinger(sub(4, -8))
        );
        assert!(parse_inject("axis wheel 1").is_err());
        assert!(parse_inject("axis spinner 0 120").is_err());
    }

    #[test]
    fn gesture_updates_parse_with_and_without_the_word() {
        for line in ["swipe 120 -240", "swipe update 120 -240"] {
            assert_eq!(
                parse_inject(line).expect(line),
                InjectLine::Swipe(GestureStep::Update(sub(120, -240))),
                "{line}"
            );
        }
        assert_eq!(
            parse_inject("swipe cancel").expect("swipe"),
            InjectLine::Swipe(GestureStep::End(GestureEnd::Cancelled))
        );
        assert_eq!(
            parse_inject("hold begin 2").expect("hold"),
            InjectLine::Hold(GestureStep::Begin { fingers: 2 })
        );
        match parse_inject("pinch update 1250 -3000 12 -6").expect("pinch") {
            InjectLine::Pinch(GestureStep::Update(delta)) => {
                assert_eq!(delta.rotation_millideg, -3000);
                assert_eq!(delta.delta, sub(12, -6));
                assert_eq!(delta.scale, ds_core::vocab::Fraction(1250));
            }
            other => panic!("{other:?}"),
        }
        for line in [
            "swipe begin 0",
            "swipe begin",
            "hold 1 2",
            "pinch 1 2 3",
            "swipe end now",
        ] {
            assert!(parse_inject(line).is_err(), "{line}");
        }
    }

    #[test]
    fn touch_steps_take_slot_and_position() {
        assert_eq!(
            parse_inject("touch down 1 30 40").expect("touch"),
            InjectLine::Touch(TouchStep::Down {
                slot: 1,
                at: LogicalPoint { x: 30, y: 40 }
            })
        );
        for line in ["touch move 0 -5 6", "touch motion 0 -5 6"] {
            assert_eq!(
                parse_inject(line).expect(line),
                InjectLine::Touch(TouchStep::Motion {
                    slot: 0,
                    at: LogicalPoint { x: -5, y: 6 }
                }),
                "{line}"
            );
        }
        assert_eq!(
            parse_inject("touch cancel").expect("touch"),
            InjectLine::Touch(TouchStep::Cancel)
        );
        assert!(parse_inject("touch down 1 30").is_err());
        assert!(parse_inject("touch up").is_err());
    }

    #[test]
    fn sleep_is_bounded_and_whitespace_is_trimmed() {
        assert_eq!(
            parse_inject("  sleep   3600000 ").expect("sleep"),
            InjectLine::Sleep(Duration::from_secs(3600))
        );
        assert!(parse_inject("sleep 3600001").is_err());
        assert!(parse_inject("sleep 1.5").is_err());
        assert_eq!(
            parse_inject("device unplug switch").expect("device"),
            InjectLine::Device(DeviceVerb::Unplug(DeviceClass::Switch))
        );
        assert!(parse_inject("device plug mouse").is_err());
    }

    #[test]
    fn a_refusal_names_the_whole_line() {
        match parse_inject("  move 1  ") {
            Err(IpcError::BadLine(line)) => assert_eq!(line, "  move 1  "),
            other => panic!("{other:?}"),
        }
    }
}
