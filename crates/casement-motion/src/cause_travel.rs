//! How a cause moves things: which spring, or none (design/27 section 3.12 rules 2 and 5).

use casement_core::Cause;
use ds_core::press::Press;
use ds_motion::detail::touch::{Contact, Touch};
use ds_motion::spring_spec::{SpringResponse, SpringSpec};
use ds_style::appearance::motion::MotionLevel;

/// How a change of target is travelled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Travel {
    /// The new target is the position at once (the client's own configure, an output change,
    /// and everything under Reduced motion: no state is carried by movement alone).
    Snap,
    /// A spring from where the leg is, at the speed it has.
    Spring {
        /// Who caused it (a released swipe is a contact and may overshoot).
        touch: Touch,
        /// How quick it feels.
        response: SpringResponse,
    },
}

/// The travel `cause` asks for under `level`. A drag follows quickly; a keyboard command, an
/// overview, a minimise, an open and a Space change take the Move response; a close is quick; a
/// released swipe runs on the spring its release chose (a throw may overshoot, a hand having
/// spent it).
pub(crate) fn travel_of(cause: &Cause, level: MotionLevel) -> Travel {
    let spring = |touch, response| Travel::Spring { touch, response };
    match (level, cause) {
        (MotionLevel::Reduced, _) | (_, Cause::Configure | Cause::OutputChange) => Travel::Snap,
        (_, Cause::Drag | Cause::Close) => spring(Touch::Remote, SpringResponse::Quick),
        (
            _,
            Cause::Keyboard
            | Cause::Overview
            | Cause::Minimise(_)
            | Cause::Open
            | Cause::SpaceSwitch,
        ) => spring(Touch::Remote, SpringResponse::Move),
        (_, Cause::Swipe(release)) => spring(thrown_by(release.velocity), SpringResponse::Move),
    }
}

/// The touch a released swipe is. `Contact::pressed` is the one public way to a contact without
/// Dioxus; a swipe release is the compositor's own proof of the person's hand (A23: if quire
/// closes that constructor, this is the one line that changes).
fn thrown_by(velocity: ds_motion::velocity::Velocity) -> Touch {
    Touch::Contact(Contact::pressed(&Press::primary()).with_velocity(velocity))
}

impl Travel {
    /// The spring a window's position and size travel on (none for `Snap`).
    pub(crate) fn spring(self) -> Option<SpringSpec> {
        match self {
            Travel::Spring { touch, response } => {
                Some(SpringSpec::for_touch(touch).response(response))
            }
            Travel::Snap => None,
        }
    }

    /// The spring an opacity runs on: the travel's response and never a throw (a fade does not
    /// overshoot), or a quick one when the travel snaps.
    pub(crate) fn fade(self) -> SpringSpec {
        let response = match self {
            Travel::Spring { response, .. } => response,
            Travel::Snap => SpringResponse::Quick,
        };
        SpringSpec::for_touch(Touch::Remote).response(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use casement_core::geometry::{LogicalPoint, LogicalRect, LogicalSize};
    use casement_core::output::{EdidFacts, EdidSerial};
    use casement_core::{MinimiseTarget, OutputKey, OutputName, SwipeRelease};
    use ds_motion::velocity::Velocity;

    fn minimise() -> Cause {
        let edid = EdidFacts {
            make: String::new(),
            model: String::new(),
            serial: EdidSerial::Unknown,
        };
        Cause::Minimise(MinimiseTarget {
            output: OutputKey::of(&edid, &OutputName::parse("HEADLESS-1").expect("name")),
            rect: LogicalRect::new(LogicalPoint { x: 0, y: 0 }, LogicalSize { w: 1, h: 1 }),
        })
    }

    #[test]
    fn each_cause_picks_its_travel() {
        let remote = |response| Travel::Spring {
            touch: Touch::Remote,
            response,
        };
        let cases = [
            ("configure", Cause::Configure, Travel::Snap),
            ("output change", Cause::OutputChange, Travel::Snap),
            ("drag", Cause::Drag, remote(SpringResponse::Quick)),
            ("close", Cause::Close, remote(SpringResponse::Quick)),
            ("keyboard", Cause::Keyboard, remote(SpringResponse::Move)),
            ("overview", Cause::Overview, remote(SpringResponse::Move)),
            ("minimise", minimise(), remote(SpringResponse::Move)),
            ("open", Cause::Open, remote(SpringResponse::Move)),
            (
                "space switch",
                Cause::SpaceSwitch,
                remote(SpringResponse::Move),
            ),
        ];
        for (name, cause, want) in cases {
            assert_eq!(travel_of(&cause, MotionLevel::Standard), want, "{name}");
            assert_eq!(
                travel_of(&cause, MotionLevel::Reduced),
                Travel::Snap,
                "{name} reduced"
            );
        }
    }

    #[test]
    fn a_swipe_runs_on_its_releases_spring() {
        let swipe = Cause::Swipe(SwipeRelease {
            velocity: Velocity(900),
        });
        let travel = travel_of(&swipe, MotionLevel::Standard);
        let spec = travel.spring().expect("a swipe springs");
        assert_eq!(spec.thrown(MotionLevel::Standard), Velocity(900));
        assert_eq!(travel.fade().thrown(MotionLevel::Standard), Velocity(0));
        assert_eq!(travel_of(&swipe, MotionLevel::Reduced), Travel::Snap);
    }
}
