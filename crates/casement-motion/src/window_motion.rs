//! A window's legs: its rectangle's origin and size and its opacity, one spring leg each.

use crate::cause_travel::Travel;
use crate::pose::Pose;
use casement_core::OutputKey;
use casement_core::time::Mono;
use ds_motion::spring::{Leg, SpringPhase, SpringTuning, State};
use ds_motion::spring_spec::SpringSpec;
use ds_style::appearance::motion::MotionLevel;

/// How many pixels one unit of each channel draws, for the rest test (an opacity step of 1.0
/// is worth a thousand pixels).
const PX_PER_UNIT: [f64; 5] = [1.0, 1.0, 1.0, 1.0, 1000.0];

/// Whether a window is meant to be on screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Fate {
    /// It is in the arrangement.
    Present,
    /// It left it: fading out, then gone from every frame.
    Leaving,
}

/// The legs that move one window (x, y, width, height, opacity), all begun at `began`.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct WindowLegs {
    legs: [Leg; 5],
    began: Mono,
    /// The output it is drawn on.
    pub(crate) output: OutputKey,
    /// Its place in the arrangement's order, bottom of the stack first.
    pub(crate) stack: u32,
    /// Whether it is meant to be shown.
    pub(crate) fate: Fate,
}

impl WindowLegs {
    /// A window standing still at `pose`.
    pub(crate) fn still(pose: Pose, output: OutputKey, stack: u32, now: Mono) -> WindowLegs {
        let spring = at_rest_spring();
        WindowLegs {
            legs: pose.channels().map(|at| Leg::still(at, spring)),
            began: now,
            output,
            stack,
            fate: Fate::Present,
        }
    }

    /// Where it is at `now`, and how fast each channel moves.
    pub(crate) fn states(&self, now: Mono) -> [State; 5] {
        let elapsed = now.since(self.began);
        self.legs.map(|leg| leg.at(elapsed))
    }

    /// Where it is at `now`.
    pub(crate) fn pose(&self, now: Mono) -> Pose {
        let [x, y, w, h, opacity] = self.states(now).map(|state| state.position);
        Pose {
            x,
            y,
            w,
            h,
            opacity,
        }
    }

    /// Where each channel is going.
    pub(crate) fn target(&self) -> Pose {
        let [x, y, w, h, opacity] = self.legs.map(|leg| leg.target);
        Pose {
            x,
            y,
            w,
            h,
            opacity,
        }
    }

    /// Whether any channel still moves at `now`.
    pub(crate) fn is_moving(&self, now: Mono) -> bool {
        let elapsed = now.since(self.began);
        self.legs
            .iter()
            .zip(PX_PER_UNIT)
            .any(|(leg, px)| leg.phase(elapsed, px) == SpringPhase::Moving)
    }

    /// The legs aimed at `goal` from where they are at `now`, speeds kept; `Snap` puts them
    /// there at once. The opacity always runs on the travel's fade spring.
    pub(crate) fn aimed(
        &self,
        goal: Pose,
        travel: Travel,
        level: MotionLevel,
        now: Mono,
    ) -> WindowLegs {
        let here = self.states(now);
        let targets = goal.channels();
        let legs = std::array::from_fn(|channel| {
            let (state, target) = (here[channel], targets[channel]);
            match (travel.spring(), channel) {
                (None, 0..=3) => still_leg(target),
                (Some(spec), 0..=3) => leg_to(state, target, spec, level),
                (_, _) => leg_to(state, target, travel.fade(), level),
            }
        });
        WindowLegs {
            legs,
            began: now,
            ..self.clone()
        }
    }

    /// The same legs, marked as meant to be shown or as leaving.
    pub(crate) fn with_fate(self, fate: Fate) -> WindowLegs {
        WindowLegs { fate, ..self }
    }

    /// The same legs on another output and stack place.
    pub(crate) fn placed(self, output: OutputKey, stack: u32) -> WindowLegs {
        WindowLegs {
            output,
            stack,
            ..self
        }
    }
}

/// A leg from `state` to `target` under `spec`, its spring chosen for the travel it has.
pub(crate) fn leg_to(state: State, target: f64, spec: SpringSpec, level: MotionLevel) -> Leg {
    Leg {
        start: state,
        target,
        spring: spec.spring(target - state.position, level),
    }
}

/// A leg standing still at `at`.
pub(crate) fn still_leg(at: f64) -> Leg {
    Leg::still(at, at_rest_spring())
}

/// The spring a standing leg carries (it never moves, so only its shape matters).
fn at_rest_spring() -> SpringTuning {
    use ds_motion::detail::touch::Touch;
    SpringSpec::for_touch(Touch::Remote).spring(0.0, MotionLevel::Standard)
}

#[cfg(test)]
mod tests {
    use super::*;
    use casement_core::OutputName;
    use casement_core::geometry::{LogicalPoint, LogicalRect, LogicalSize};
    use casement_core::time::Mono;
    use ds_core::press::Press;
    use ds_motion::detail::touch::{Contact, Touch};
    use ds_motion::spring_spec::SpringResponse;
    use ds_motion::velocity::Velocity;
    use std::time::Duration;

    fn ms(n: u64) -> Mono {
        Mono(Duration::from_millis(n))
    }

    fn output() -> OutputKey {
        let edid = casement_core::output::EdidFacts {
            make: String::new(),
            model: String::new(),
            serial: casement_core::output::EdidSerial::Unknown,
        };
        OutputKey::of(&edid, &OutputName::parse("HEADLESS-1").expect("name"))
    }

    fn shown(x: i32, y: i32, w: u32, h: u32) -> Pose {
        Pose::shown(LogicalRect::new(
            LogicalPoint { x, y },
            LogicalSize { w, h },
        ))
    }

    fn spring(touch: Touch) -> Travel {
        Travel::Spring {
            touch,
            response: SpringResponse::Move,
        }
    }

    /// The retarget rule: a leg aimed anew has the position and the velocity the old one had at
    /// that instant, on every channel (the closed form, so the bound is arithmetic, not a
    /// sampling tolerance; the bar is 1%).
    #[test]
    fn a_new_leg_has_the_old_legs_position_and_velocity() {
        let thrown =
            Touch::Contact(Contact::pressed(&Press::primary()).with_velocity(Velocity(900)));
        let first = WindowLegs::still(shown(0, 0, 400, 300), output(), 0, ms(0));
        let mut worst: f64 = 0.0;
        for (touch, far) in [
            (Touch::Remote, shown(4_000, 800, 900, 700)),
            (thrown, shown(-3_000, 1_200, 300, 250)),
        ] {
            let moving = first.aimed(far, spring(touch), MotionLevel::Standard, ms(0));
            for t in [20u64, 80, 200, 330] {
                let there = [
                    shown(-2_000, 40, 500, 400),
                    shown(4_000, 800, 900, 700),
                    shown(0, 0, 400, 300),
                ];
                for goal in there {
                    let next = moving.aimed(goal, spring(touch), MotionLevel::Standard, ms(t));
                    for (old, new) in moving.states(ms(t)).iter().zip(next.states(ms(t))) {
                        assert!(
                            (old.position - new.position).abs()
                                <= 1e-9 * old.position.abs().max(1.0)
                        );
                        if old.velocity.abs() > 1e-3 {
                            worst =
                                worst.max((old.velocity - new.velocity).abs() / old.velocity.abs());
                        }
                    }
                }
            }
        }
        assert!(worst <= 0.01, "velocity jumped by {:.4}%", worst * 100.0);
        assert!(worst < 1e-9, "closed-form continuity is exact, got {worst}");
    }

    #[test]
    fn snapping_puts_the_geometry_at_the_target_at_once_and_still_fades() {
        let start = WindowLegs::still(shown(0, 0, 400, 300).faded(), output(), 0, ms(0));
        let snapped = start.aimed(
            shown(900, 500, 600, 400),
            Travel::Snap,
            MotionLevel::Reduced,
            ms(0),
        );
        let pose = snapped.pose(ms(0));
        assert_eq!(
            (pose.x, pose.y, pose.w, pose.h),
            (900.0, 500.0, 600.0, 400.0)
        );
        assert_eq!(pose.opacity, 0.0);
        assert!(snapped.is_moving(ms(0)), "the cross-fade is still running");
        assert!(!snapped.is_moving(ms(3_000)));
        assert_eq!(snapped.pose(ms(3_000)).opacity, 1.0);
    }
}
