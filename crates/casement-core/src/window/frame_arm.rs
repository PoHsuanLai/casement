//! A traffic light acts on release, as on the Mac: a press arms it (it shows pressed), a
//! release over the same light fires it, moving off it cancels it and moving back arms it
//! again. Pure; the edge feeds it the pointer and carries out what fires.

use super::frame::Light;
use crate::ids::WindowId;
use crate::input::ButtonCode;

/// One light of one window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LightAt {
    /// The window.
    pub window: WindowId,
    /// The light.
    pub light: Light,
}

/// What the pointer did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ArmInput {
    /// `button` went down on a light.
    Press {
        /// The light.
        on: LightAt,
        /// The button.
        button: ButtonCode,
    },
    /// The pointer moved; it is over this light, or over no light.
    Move(Option<LightAt>),
    /// `button` came up with the pointer over this light, or over no light.
    Release {
        /// The button.
        button: ButtonCode,
        /// The light under the pointer.
        over: Option<LightAt>,
    },
    /// The window or its frame is gone: forget the press.
    Lost,
}

/// Whether the pointer is still on the light it pressed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArmReach {
    /// On the light: it shows pressed and would fire.
    On,
    /// Off the light: nothing would fire.
    Off,
}

/// Where the light press stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum LightArm {
    /// No light is held down.
    #[default]
    Idle,
    /// A light was pressed and the button is still down.
    Held {
        /// The light that was pressed.
        light: LightAt,
        /// The button holding it.
        button: ButtonCode,
        /// Whether the pointer is still on it.
        reach: ArmReach,
    },
}

/// What a step asks the edge to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ArmOutcome {
    /// Nothing.
    Nothing,
    /// The light was released on itself: do what it does.
    Fire(LightAt),
}

impl LightArm {
    /// The light drawn pressed: held down with the pointer on it.
    ///
    /// ```
    /// use casement_core::WindowId;
    /// use casement_core::input::ButtonCode;
    /// use casement_core::window::{ArmInput, Light, LightArm, LightAt};
    ///
    /// let on = LightAt { window: WindowId(1), light: Light::Close };
    /// let (arm, _) = LightArm::Idle.step(ArmInput::Press { on, button: ButtonCode(0x110) });
    /// assert_eq!(arm.pressed(), Some(on));
    /// ```
    pub fn pressed(self) -> Option<LightAt> {
        match self {
            LightArm::Held {
                light,
                reach: ArmReach::On,
                ..
            } => Some(light),
            _ => None,
        }
    }

    /// The state after `input`, and what to do about it.
    pub fn step(self, input: ArmInput) -> (LightArm, ArmOutcome) {
        match (self, input) {
            (_, ArmInput::Press { on, button }) => (
                LightArm::Held {
                    light: on,
                    button,
                    reach: ArmReach::On,
                },
                ArmOutcome::Nothing,
            ),
            (LightArm::Held { light, button, .. }, ArmInput::Move(over)) => (
                LightArm::Held {
                    light,
                    button,
                    reach: reach_of(light, over),
                },
                ArmOutcome::Nothing,
            ),
            (
                LightArm::Held { light, button, .. },
                ArmInput::Release {
                    button: released,
                    over,
                },
            ) if released == button => match reach_of(light, over) {
                ArmReach::On => (LightArm::Idle, ArmOutcome::Fire(light)),
                ArmReach::Off => (LightArm::Idle, ArmOutcome::Nothing),
            },
            (_, ArmInput::Lost) => (LightArm::Idle, ArmOutcome::Nothing),
            (held, _) => (held, ArmOutcome::Nothing),
        }
    }
}

/// Whether `over` is `light`.
fn reach_of(light: LightAt, over: Option<LightAt>) -> ArmReach {
    match over == Some(light) {
        true => ArmReach::On,
        false => ArmReach::Off,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LEFT: ButtonCode = ButtonCode(0x110);
    const RIGHT: ButtonCode = ButtonCode(0x111);

    fn at(window: u64, light: Light) -> LightAt {
        LightAt {
            window: WindowId(window),
            light,
        }
    }

    /// One scripted input.
    #[derive(Clone, Copy)]
    enum Step {
        Press(LightAt),
        PressWith(ButtonCode, LightAt),
        Move(Option<LightAt>),
        Release(Option<LightAt>),
        ReleaseWith(ButtonCode, Option<LightAt>),
        Lost,
    }

    fn input(step: Step) -> ArmInput {
        match step {
            Step::Press(on) => ArmInput::Press { on, button: LEFT },
            Step::PressWith(button, on) => ArmInput::Press { on, button },
            Step::Move(over) => ArmInput::Move(over),
            Step::Release(over) => ArmInput::Release { button: LEFT, over },
            Step::ReleaseWith(button, over) => ArmInput::Release { button, over },
            Step::Lost => ArmInput::Lost,
        }
    }

    /// Runs `steps`; what fired at each, and whether a light shows pressed after each.
    fn run(steps: &[Step]) -> Vec<(ArmOutcome, Option<LightAt>)> {
        let mut arm = LightArm::Idle;
        steps
            .iter()
            .map(|step| {
                let (next, outcome) = arm.step(input(*step));
                arm = next;
                (outcome, arm.pressed())
            })
            .collect()
    }

    #[test]
    fn a_light_acts_on_release_inside() {
        let red = at(1, Light::Close);
        let green = at(1, Light::Zoom);
        let other = at(2, Light::Close);
        use ArmOutcome::{Fire, Nothing};
        type Case = (&'static str, Vec<Step>, Vec<(ArmOutcome, Option<LightAt>)>);
        let cases: Vec<Case> = vec![
            (
                "press shows pressed, release fires",
                vec![Step::Press(red), Step::Release(Some(red))],
                vec![(Nothing, Some(red)), (Fire(red), None)],
            ),
            (
                "moving off cancels, releasing off does nothing",
                vec![Step::Press(red), Step::Move(None), Step::Release(None)],
                vec![(Nothing, Some(red)), (Nothing, None), (Nothing, None)],
            ),
            (
                "moving back re-arms",
                vec![
                    Step::Press(red),
                    Step::Move(None),
                    Step::Move(Some(red)),
                    Step::Release(Some(red)),
                ],
                vec![
                    (Nothing, Some(red)),
                    (Nothing, None),
                    (Nothing, Some(red)),
                    (Fire(red), None),
                ],
            ),
            (
                "over another light is off",
                vec![
                    Step::Press(red),
                    Step::Move(Some(green)),
                    Step::Release(Some(green)),
                ],
                vec![(Nothing, Some(red)), (Nothing, None), (Nothing, None)],
            ),
            (
                "the same light of another window is off",
                vec![Step::Press(red), Step::Release(Some(other))],
                vec![(Nothing, Some(red)), (Nothing, None)],
            ),
            (
                "a release with no press does nothing",
                vec![Step::Release(Some(red)), Step::Move(Some(red))],
                vec![(Nothing, None), (Nothing, None)],
            ),
            (
                "another button's release leaves it held",
                vec![
                    Step::Press(red),
                    Step::ReleaseWith(RIGHT, Some(red)),
                    Step::Release(Some(red)),
                ],
                vec![
                    (Nothing, Some(red)),
                    (Nothing, Some(red)),
                    (Fire(red), None),
                ],
            ),
            (
                "a second press replaces the first",
                vec![
                    Step::Press(red),
                    Step::PressWith(RIGHT, green),
                    Step::Release(Some(green)),
                    Step::ReleaseWith(RIGHT, Some(green)),
                ],
                vec![
                    (Nothing, Some(red)),
                    (Nothing, Some(green)),
                    (Nothing, Some(green)),
                    (Fire(green), None),
                ],
            ),
            (
                "a lost window forgets the press",
                vec![Step::Press(red), Step::Lost, Step::Release(Some(red))],
                vec![(Nothing, Some(red)), (Nothing, None), (Nothing, None)],
            ),
            (
                "it fires once",
                vec![
                    Step::Press(red),
                    Step::Release(Some(red)),
                    Step::Release(Some(red)),
                ],
                vec![(Nothing, Some(red)), (Fire(red), None), (Nothing, None)],
            ),
        ];
        for (name, steps, want) in cases {
            assert_eq!(run(&steps), want, "{name}");
        }
    }
}
