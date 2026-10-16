//! Split View: a window takes one half of its output in a Space of its own, the shell shows
//! the desktop's other windows to pick the second half from, and a pick completes the pair.
//!
//! The first window goes full screen at once (a Space of its own, as `Enter Full Screen`
//! does). With no other window to pick, that is all that happens. Otherwise the model is
//! *choosing* until a pick, a cancel, or the first window leaving; an unanswered choice leaves
//! the first window alone, full screen, as the Mac does. Leaving full screen on either window
//! ends the Split View (`requests.rs`); closing one leaves the other full screen
//! ([`Model::split_survivor`]). Where the halves sit is layout's (`casement-layout`).

use super::Model;
use super::moved::Moved;
use crate::app::AppVisibility;
use crate::cause::Cause;
use crate::effect::Effect;
use crate::focus::FocusIn;
use crate::ids::{OutputKey, WindowId};
use crate::shell::{ShellNote, SplitChoices};
use crate::space::{Space, SpaceContent, SplitPair};
use crate::window::{Parent, Restore, SpacePlace, SplitSide, Window, WindowState};
use casement_protocols::vocab::WindowRole;
use ds_core::vocab::Fraction;
use porter_core::SpaceId;

/// Where a new Split View's divider starts: the middle.
const MIDDLE: Fraction = Fraction(500);

/// A Split View whose other half is being chosen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SplitChoice {
    /// The window already full screen.
    first: WindowId,
    /// The half it took.
    side: SplitSide,
    /// Its full-screen Space, which becomes the Split View.
    space: SpaceId,
    /// The desktop the first window came from: only windows of that desktop can be picked. None
    /// when it was on every desktop (any desktop of its output then).
    desktop: Option<SpaceId>,
}

impl Model {
    /// `window` takes the `side` half of its output in a Split View. It goes full screen on a
    /// Space of its own first; when other windows can fill the other half the shell is told
    /// which (the model is then choosing), else the window just stays full screen. A window
    /// that cannot go full screen (already is, or in the dock) changes nothing.
    pub(super) fn split_view(self, window: &Window, side: SplitSide) -> Moved {
        if window.state.restore().is_none() {
            return Moved::of(self);
        }
        let first = window.id;
        let desktop = match &window.place {
            SpacePlace::On(id) => Some(id.clone()),
            SpacePlace::AllSpaces => None,
        };
        self.choice_over()
            .then(|model| model.full_screened(window, None))
            .then(|model| model.choosing(first, side, desktop))
    }

    /// The shell picked `picked` for the other half. A pick that is not a candidate now changes
    /// nothing (the choosing goes on); when the first window is no longer alone in its Space the
    /// choosing is over.
    pub(super) fn split_pick(self, picked: WindowId) -> Moved {
        let Some(choice) = self.split_choice.clone() else {
            return Moved::of(self);
        };
        if !self.first_is_alone(&choice) {
            return self.choice_over();
        }
        let restore = self.windows.get(&picked).and_then(|w| w.state.restore());
        let Some(restore) = restore.filter(|_| self.split_candidates(&choice).contains(&picked))
        else {
            return Moved::of(self);
        };
        let (leading, trailing) = match choice.side {
            SplitSide::Left => (choice.first, picked),
            SplitSide::Right => (picked, choice.first),
        };
        let pair = SplitPair {
            leading,
            trailing,
            divider: MIDDLE,
        };
        let output = self.spaces.get(&choice.space).map(|s| s.output.clone());
        let mut model = self;
        if let Some(window) = model.windows.get_mut(&picked) {
            window.place = SpacePlace::On(choice.space.clone());
            window.home = None;
            window.state = WindowState::FullScreen {
                space: choice.space.clone(),
                restore,
            };
        }
        let space = choice.space.clone();
        let shown = choice.space;
        let first = choice.first;
        model
            .with_content(&space, SpaceContent::SplitView(pair))
            .then(Model::choice_over)
            .then(|model| match output {
                Some(output) => model.feed(FocusIn::SpaceShown {
                    output,
                    space: shown,
                }),
                None => Moved::of(model),
            })
            .then(|model| model.relaid_out(picked, Cause::Configure))
            .then(|model| model.relaid_out(first, Cause::Configure))
    }

    /// Escape or a click elsewhere: the first window stays alone in its Space, full screen.
    pub(super) fn split_cancel(self) -> Moved {
        self.choice_over()
    }

    /// The divider of the Split View `window` is in moved to give the leading window `share`
    /// (kept between 200 and 800 thousandths; layout holds each half to its minimum width).
    pub(super) fn split_divide(self, window: WindowId, share: Fraction) -> Moved {
        let found = self
            .spaces_naming(window)
            .into_iter()
            .find_map(|space| match space.content {
                SpaceContent::SplitView(pair) => Some((space.id, pair)),
                _ => None,
            });
        let Some((id, pair)) = found else {
            return Moved::of(self);
        };
        let divider = Fraction(share.0.clamp(SplitPair::NEAREST.0, SplitPair::NEAREST.1));
        if divider == pair.divider {
            return Moved::of(self);
        }
        let moved = SplitPair { divider, ..pair };
        self.with_content(&id, SpaceContent::SplitView(moved))
            .then(|model| model.relaid_out(moved.leading, Cause::Configure))
            .then(|model| model.relaid_out(moved.trailing, Cause::Configure))
    }

    /// One window of the Split View in `space` closed: the other stays, full screen, in a
    /// Space of its own again.
    pub(super) fn split_survivor(self, space: &SpaceId, pair: SplitPair, gone: WindowId) -> Moved {
        let other = match pair.leading == gone {
            true => pair.trailing,
            false => pair.leading,
        };
        self.with_content(space, SpaceContent::FullScreen(other))
            .then(|model| model.relaid_out(other, Cause::Configure))
    }

    /// Every window that is full screen in `space`, with the state it returns to.
    pub(super) fn full_screen_members(&self, space: &SpaceId) -> Vec<(WindowId, Restore)> {
        self.windows
            .values()
            .filter_map(|window| match &window.state {
                WindowState::FullScreen {
                    space: own,
                    restore,
                } if own == space => Some((window.id, restore.clone())),
                _ => None,
            })
            .collect()
    }

    /// Another Space than the choosing's own is shown: the choosing is cancelled and the first
    /// window stays alone, full screen (as on the Mac, where switching away abandons it).
    pub(super) fn choice_unless_on(self, shown: &SpaceId) -> Moved {
        match self.split_choice.as_ref().map(|choice| &choice.space) {
            Some(own) if own != shown => self.choice_over(),
            _ => Moved::of(self),
        }
    }

    /// The choosing is over when it was for one of `windows`.
    pub(super) fn choice_over_for(self, windows: &[WindowId]) -> Moved {
        match self.split_choice.as_ref().map(|choice| choice.first) {
            Some(first) if windows.contains(&first) => self.choice_over(),
            _ => Moved::of(self),
        }
    }

    /// No Split View is being chosen; the shell is told when one was.
    fn choice_over(self) -> Moved {
        let ended = self.split_choice.as_ref().map(|choice| choice.first);
        let model = Model {
            split_choice: None,
            ..self
        };
        Moved::of(model)
            .with_all(ended.map(|first| Effect::Shell(ShellNote::SplitChoiceEnd(first))))
    }

    /// `first` is full screen in its own Space: the shell is told which windows can fill the
    /// other half, unless there are none.
    fn choosing(self, first: WindowId, side: SplitSide, desktop: Option<SpaceId>) -> Moved {
        let space = self.windows.get(&first).and_then(|w| match &w.state {
            WindowState::FullScreen { space, .. } => Some(space.clone()),
            _ => None,
        });
        let Some(space) = space else {
            return Moved::of(self);
        };
        let choice = SplitChoice {
            first,
            side,
            space,
            desktop,
        };
        let candidates = self.split_candidates(&choice);
        match candidates.is_empty() {
            false => {
                let note = ShellNote::SplitChoose(SplitChoices {
                    first,
                    side,
                    candidates,
                });
                let model = Model {
                    split_choice: Some(choice),
                    ..self
                };
                Moved::of(model).with(Effect::Shell(note))
            }
            true => Moved::of(self),
        }
    }

    /// The first window is still the only one in its full-screen Space.
    fn first_is_alone(&self, choice: &SplitChoice) -> bool {
        let space = self.spaces.get(&choice.space);
        space.is_some_and(|space| space.content == SpaceContent::FullScreen(choice.first))
    }

    /// The windows that can fill the other half beside the choice's first window, most recently
    /// raised first: the visible, ordinary windows on the desktop it came from.
    fn split_candidates(&self, choice: &SplitChoice) -> Vec<WindowId> {
        let first = choice.first;
        let Some(output) = self.windows.get(&first).and_then(|w| self.output_of(w)) else {
            return Vec::new();
        };
        let mut found: Vec<&Window> = self
            .windows
            .values()
            .filter(|window| window.id != first)
            .filter(|window| can_share(self, window, &output, choice.desktop.as_ref()))
            .collect();
        found.sort_by_key(|window| std::cmp::Reverse(window.stack));
        found.into_iter().map(|window| window.id).collect()
    }
}

/// Whether `window` can take half a screen of `output`: an ordinary window of a shown app, out
/// of the dock, on `desktop` (any desktop of the output when none is named).
fn can_share(
    model: &Model,
    window: &Window,
    output: &OutputKey,
    desktop: Option<&SpaceId>,
) -> bool {
    let on_desktop = match &window.place {
        SpacePlace::On(id) => model.spaces.get(id).is_some_and(|space: &Space| {
            space.output == *output
                && matches!(space.content, SpaceContent::Desktop)
                && desktop.is_none_or(|wanted| wanted == id)
        }),
        SpacePlace::AllSpaces => false,
    };
    let hidden = model
        .apps
        .get(&window.app)
        .is_some_and(|app| matches!(app.visibility, AppVisibility::Hidden { .. }));
    let ordinary = !matches!(window.role, WindowRole::Shell)
        && window.parent == Parent::None
        && !matches!(
            window.state,
            WindowState::Minimised { .. } | WindowState::FullScreen { .. }
        );
    on_desktop && !hidden && ordinary
}
