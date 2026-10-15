//! Mission Control: asked for by a desktop command or the shell, it spreads the shown Space's
//! windows (layout's slots, motion's flight) until a window is picked or the person leaves.
//!
//! The model is *entering* from the request until the windows have had their time to arrive,
//! then *open*; a leave (a pick, Escape, the same key again) makes it *leaving* until the
//! windows have flown back, then *closed*. A pick raises and keys the window as a press on it
//! would. While the overview is not closed, the edge keeps input from the windows.
//!
//! It also leaves by itself, with no pick and the same flight home, when what it shows is gone:
//! a Space switch, the session locking, the last window of the shown Space closing. A window
//! that arrives meanwhile is added to the overview instead (it gets a slot), as on the Mac.

use super::Model;
use super::moved::Moved;
use crate::cause::Cause;
use crate::effect::Effect;
use crate::focus::{FocusIn, HitPart, Press};
use crate::ids::{OutputKey, WindowId};
use crate::input::Origin;
use crate::lock::LockState;
use crate::overview::Overview;
use crate::shell::ShellNote;
use crate::time::{Due, Mono, TimerKey};
use casement_protocols::vocab::{MacMods, OverviewMode, Toggle, WindowRole};
use std::time::Duration;

/// How long the windows take to fly (the Mac's overview takes about this long): the time to
/// settle between entering and open, and between leaving and closed.
const SETTLE: Duration = Duration::from_millis(350);

/// The window the person picked, and where the pick came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OverviewPick {
    /// The window.
    pub window: WindowId,
    /// The pointer or touch that picked it.
    pub origin: Origin,
}

impl Model {
    /// Where Mission Control is.
    ///
    /// ```
    /// use casement_core::Model;
    /// use casement_core::overview::Overview;
    /// use casement_core::settings::CasementSettings;
    /// use casement_core::space::SpaceSeed;
    /// use casement_core::time::Mono;
    /// use std::time::Duration;
    ///
    /// let epoch = Mono(Duration::ZERO);
    /// let model = Model::new(CasementSettings::defaults(), epoch, SpaceSeed::from_entropy([7; 16]));
    /// assert_eq!(model.overview(), Overview::Closed);
    /// // With no output there is nothing to spread: opening changes nothing.
    /// let (model, effects) = model.toggle_overview(epoch);
    /// assert_eq!(model.overview(), Overview::Closed);
    /// assert!(effects.is_empty());
    /// ```
    pub fn overview(&self) -> Overview {
        self.overview
    }

    /// Mission Control opens, or closes when it is open (the command's meaning). While the
    /// session is locked it does nothing.
    pub fn toggle_overview(self, now: Mono) -> (Model, Vec<Effect>) {
        self.run(now, |model| match model.lock {
            LockState::Unlocked => model.toggled_overview(),
            _ => Moved::of(model),
        })
    }

    pub(super) fn toggled_overview(self) -> Moved {
        match self.overview {
            Overview::Closed | Overview::Leaving => self.enter_overview(),
            Overview::Entering | Overview::Open => self.leave_overview(None),
        }
    }

    /// Mission Control opens (already entering or open: nothing).
    pub fn open_overview(self, now: Mono) -> (Model, Vec<Effect>) {
        self.run(now, |model| match model.lock {
            LockState::Unlocked => match model.overview {
                Overview::Closed | Overview::Leaving => model.enter_overview(),
                Overview::Entering | Overview::Open => Moved::of(model),
            },
            _ => Moved::of(model),
        })
    }

    /// Mission Control closes, with `picked` raised and keyed when there is one (already
    /// leaving or closed: nothing).
    pub fn close_overview(self, picked: Option<OverviewPick>, now: Mono) -> (Model, Vec<Effect>) {
        self.run(now, |model| model.leave_overview(picked))
    }

    fn enter_overview(self) -> Moved {
        let outputs: Vec<OutputKey> = self.outputs.0.keys().cloned().collect();
        if outputs.is_empty() {
            return Moved::of(self);
        }
        let settle = self.settle_timer();
        Model {
            overview: Overview::Entering,
            ..self
        }
        .redrawn(Cause::Overview, outputs)
        .with(Effect::ArmTimer(settle))
        .with(Effect::Shell(ShellNote::Overview(Overview::Entering)))
    }

    /// The shell asked for an overview: Mission Control opens or closes, any other changes
    /// nothing, and the session lock keeps the shell out of it.
    pub(super) fn overview_asked(self, mode: OverviewMode, toggle: Toggle) -> Moved {
        if !matches!(self.lock, LockState::Unlocked) {
            return Moved::of(self);
        }
        match (mode, toggle) {
            (OverviewMode::MissionControl, Toggle::On) => match self.overview {
                Overview::Closed | Overview::Leaving => self.enter_overview(),
                Overview::Entering | Overview::Open => Moved::of(self),
            },
            (OverviewMode::MissionControl, Toggle::Off) => self.leave_overview(None),
            _ => Moved::of(self),
        }
    }

    /// Mission Control leaves with no pick when a window the person could see has closed and the
    /// Space it was on, on an output in `outputs`, has no window left (a shell window, which is on
    /// every Space, is not one). Nothing when the window was not seen, or Mission Control is not
    /// up.
    pub(super) fn leave_when_bare(self, outputs: &[OutputKey]) -> Moved {
        let bare = outputs.iter().any(|output| self.is_bare(output));
        match bare {
            true => self.leave_overview(None),
            false => Moved::of(self),
        }
    }

    /// Whether no window is seen on `output`, shell windows apart.
    fn is_bare(&self, output: &OutputKey) -> bool {
        !self.windows.values().any(|window| {
            window.role != WindowRole::Shell
                && self.is_seen(window)
                && self.output_of(window).as_ref() == Some(output)
        })
    }

    pub(super) fn leave_overview(self, picked: Option<OverviewPick>) -> Moved {
        if !matches!(self.overview, Overview::Entering | Overview::Open) {
            return Moved::of(self);
        }
        let outputs: Vec<OutputKey> = self.outputs.0.keys().cloned().collect();
        let settle = self.settle_timer();
        Model {
            overview: Overview::Leaving,
            ..self
        }
        .then_pick(picked)
        .then(|model| model.redrawn(Cause::Overview, outputs))
        .with(Effect::ArmTimer(settle))
        .with(Effect::Shell(ShellNote::Overview(Overview::Leaving)))
    }

    /// The fingers lifted short of a verdict and Mission Control stays as it was. The windows
    /// were drawn where the fingers put them, so they are laid out again to return from there:
    /// closed, they fly back to the desktop (the overview shows as leaving until they land, the
    /// shell is told when it is closed); up, they return to their slots.
    pub(super) fn fingers_stayed(self) -> Moved {
        let outputs: Vec<OutputKey> = self.outputs.0.keys().cloned().collect();
        match self.overview {
            Overview::Closed if !outputs.is_empty() => {
                let settle = self.settle_timer();
                Model {
                    overview: Overview::Leaving,
                    ..self
                }
                .redrawn(Cause::Overview, outputs)
                .with(Effect::ArmTimer(settle))
            }
            _ => self.redrawn(Cause::Overview, outputs),
        }
    }

    /// The pick raises and keys its window like a press on it, when it is still there.
    fn then_pick(self, picked: Option<OverviewPick>) -> Moved {
        match picked.filter(|pick| self.windows.contains_key(&pick.window)) {
            Some(pick) => self.feed(FocusIn::Press(Press {
                window: pick.window,
                part: HitPart::Content,
                mods: MacMods::NONE,
                origin: pick.origin,
            })),
            None => Moved::of(self),
        }
    }

    fn settle_timer(&self) -> Due {
        Due {
            at: self.now.after(SETTLE),
            key: TimerKey::OverviewSettle,
        }
    }

    /// The windows have had their time to fly: entering is open, leaving is closed.
    pub(super) fn overview_settled(self) -> Moved {
        let next = match self.overview {
            Overview::Entering => Overview::Open,
            Overview::Leaving => Overview::Closed,
            other => other,
        };
        let outputs: Vec<OutputKey> = self.outputs.0.keys().cloned().collect();
        Moved::of(Model {
            overview: next,
            ..self
        })
        .with_all(outputs.into_iter().map(Effect::RequestFrames))
        .with(Effect::Shell(ShellNote::Overview(next)))
    }
}
