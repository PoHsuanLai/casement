//! The inject grammar (11 §4.1): one command per line, each answered `ok <line>` once it
//! reached the compositor. A superset of shell-host's `fake-input.sh`:
//!
//! ```text
//! move X Y                          absolute pointer position (global logical px)
//! press left|right|middle|BUTTON    release left|right|middle|BUTTON
//! key KEYSYM down|up                xkb keysym name: Return, a, Shift_L, Super_L, F5
//! type "text"                       each character, with Shift where the keymap needs it
//! axis vertical|horizontal VALUE    one wheel step in 120ths... and the source (SP1 B1):
//! axis wheel|finger|continuous V120X V120Y [PXX PXY]
//! scroll-finger DX DY               a touchpad scroll in sub-pixels
//! axis-stop                         the fingers lifted
//! swipe begin N | swipe DX DY | swipe end|cancel
//! pinch begin N | pinch SCALE ROT DX DY | pinch end|cancel
//! hold begin N | hold end|cancel
//! touch down SLOT X Y | touch move SLOT X Y | touch up SLOT | touch frame | touch cancel
//! device plug|unplug keyboard|pointer|touchpad|touch|tablet|switch
//! origin physical|remote|test       the origin of the lines that follow (debug-inject only)
//! sleep MS                          session time (the stepped clock advances; never a wall sleep)
//! ```

use crate::error::IpcError;
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

/// The line `line` says.
pub fn parse_inject(line: &str) -> Result<InjectLine, IpcError> {
    let _ = line;
    todo!("P0-T-dev: parse_inject")
}

#[cfg(test)]
mod tests {
    use super::*;
    use casement_core::geometry::SubPx;
    use casement_core::input::{AxisSource, AxisStop, GestureEnd};

    #[test]
    #[ignore = "P0-T-dev"]
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
    #[ignore = "P0-T-dev"]
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
    #[ignore = "P0-T-dev"]
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
    #[ignore = "P0-T-dev"]
    fn malformed_lines_are_refused() {
        for line in [
            "",
            "move 1",
            "press thumb-9",
            "key NotAKeysym down",
            "origin agent",
            "sleep -1",
            "type unquoted",
        ] {
            assert!(
                matches!(parse_inject(line), Err(IpcError::BadLine(_))),
                "{line}"
            );
        }
    }
}
