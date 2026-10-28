//! What a toplevel asks for: moving, resizing, zooming, minimising, full screen, activation
//! and placement.

use super::Model;
use super::moved::Moved;
use super::query::centred;
use crate::cause::{Cause, MinimiseTarget};
use crate::effect::{CoreNote, Effect, GrabSpec};
use crate::event::ToplevelRequest;
use crate::focus::{FocusIn, TokenCheck};
use crate::geometry::{LogicalPoint, LogicalRect, LogicalSize};
use crate::ids::{OutputKey, WindowId};
use crate::settings::Enabled;
use crate::space::SpaceContent;
use crate::window::{Restore, SpacePlace, Window, WindowState};
use casement_protocols::vocab::{PlaceTarget, Toggle};

impl Model {
    /// A toplevel asked for something.
    pub(super) fn request(self, id: WindowId, request: ToplevelRequest) -> Moved {
        let Some(window) = self.windows.get(&id).cloned() else {
            return Moved::of(self).with(Effect::Note(CoreNote::UnknownWindow(id)));
        };
        match request {
            ToplevelRequest::Move { serial } => {
                self.grab(&window, GrabSpec::Move { window: id, serial })
            }
            ToplevelRequest::Resize { serial, edges } => self.grab(
                &window,
                GrabSpec::Resize {
                    window: id,
                    serial,
                    edges,
                },
            ),
            ToplevelRequest::Maximise(Toggle::On) => self.place(&window, PlaceTarget::Fill),
            ToplevelRequest::Maximise(Toggle::Off) => self.place(&window, PlaceTarget::Restore),
            ToplevelRequest::Minimise => self.minimise(&window),
            ToplevelRequest::FullScreen {
                toggle: Toggle::On,
                output,
            } => self.enter_full_screen(&window, output),
            ToplevelRequest::FullScreen {
                toggle: Toggle::Off,
                ..
            } => self.leave_full_screen(&window),
            ToplevelRequest::Activate(token) => self.activate_window(&window, token),
            ToplevelRequest::WindowMenu { at } => self.window_menu(&window, at),
            ToplevelRequest::Place { target, .. } => self.place(&window, target),
        }
    }

    /// An interactive move or resize starts; a tiled or filled window leaves its place first.
    fn grab(self, window: &Window, grab: GrabSpec) -> Moved {
        let released = match &window.state {
            WindowState::Tiled { restore, .. } | WindowState::Filled { restore } => {
                Some(WindowState::Floating(*restore))
            }
            _ => None,
        };
        match window.state {
            WindowState::FullScreen { .. } | WindowState::Minimised { .. } => Moved::of(self),
            _ => match released {
                Some(state) => self
                    .with_state(window.id, state)
                    .with(Effect::StartGrab(grab))
                    .then(|model| model.relaid_out(window.id, Cause::Configure)),
                None => Moved::of(self).with(Effect::StartGrab(grab)),
            },
        }
    }

    /// An interactive move or resize put the window at `rect`: it floats there. A window in any
    /// other state (full screen, in the dock) does not follow a drag; a tiled or filled one
    /// left its place when the grab began.
    pub(super) fn dragged(self, id: WindowId, rect: LogicalRect) -> Moved {
        let mut model = self;
        let rule_changed = model.hand_placed.remove(&id);
        let rect = model.pip_fitted(id, rect);
        model.window_moved_to(id, rect, rule_changed)
    }

    /// A window was put at `rect` by hand (a test's placement, never the person's): layout keeps
    /// the rectangle where it is even outside the usable area, until the person drags the
    /// window. The person-facing rule (a drag keeps a window within reach) is unchanged.
    pub(super) fn placed_by_hand(self, id: WindowId, rect: LogicalRect) -> Moved {
        let mut model = self;
        let rule_changed = model.windows.contains_key(&id) && model.hand_placed.insert(id);
        model.window_moved_to(id, rect, rule_changed)
    }

    /// Whether window `id` was put by hand and layout keeps its rectangle as given.
    pub fn is_placed_by_hand(&self, id: WindowId) -> bool {
        self.hand_placed.contains(&id)
    }

    /// Window `id` is now at `rect`; `rule_changed` means layout's rule for it changed with
    /// the move, so it is laid out again even where the rectangle is the same.
    fn window_moved_to(self, id: WindowId, rect: LogicalRect, rule_changed: bool) -> Moved {
        let Some(window) = self.windows.get(&id) else {
            return Moved::of(self).with(Effect::Note(CoreNote::UnknownWindow(id)));
        };
        match &window.state {
            WindowState::Floating(held) if *held == rect && !rule_changed => Moved::of(self),
            WindowState::Floating(_) | WindowState::New(_) => self
                .with_state(id, WindowState::Floating(rect))
                .then(|model| model.relaid_out(id, Cause::Drag)),
            _ => Moved::of(self),
        }
    }

    /// The window goes where `target` says. Full-screen and minimised windows stay put, except that
    /// a full-screen window asked to fill will fill when it leaves full screen.
    pub(super) fn place(self, window: &Window, target: PlaceTarget) -> Moved {
        let Some(usable) = self
            .output_of(window)
            .and_then(|output| self.usable_rect(&output))
        else {
            return Moved::of(self);
        };
        let floor = floor_rect(&window.state, usable);
        let state = match (&window.state, target) {
            (WindowState::FullScreen { space, restore }, PlaceTarget::Fill) => {
                let behind = restore_rect(restore, usable);
                WindowState::FullScreen {
                    space: space.clone(),
                    restore: Restore::Filled { restore: behind },
                }
            }
            (WindowState::FullScreen { .. } | WindowState::Minimised { .. }, _) => {
                return Moved::of(self);
            }
            (_, PlaceTarget::Fill) => WindowState::Filled { restore: floor },
            (_, PlaceTarget::Centre) => WindowState::Floating(centred(floor.size, usable)),
            (
                WindowState::Tiled { restore, .. } | WindowState::Filled { restore },
                PlaceTarget::Restore,
            ) => WindowState::Floating(*restore),
            (_, PlaceTarget::Restore) => return Moved::of(self),
            (_, target) => match target.zone() {
                Some(zone) => WindowState::Tiled {
                    zone,
                    restore: floor,
                },
                None => return Moved::of(self),
            },
        };
        if state == window.state {
            return Moved::of(self);
        }
        self.with_state(window.id, state)
            .then(|model| model.relaid_out(window.id, Cause::Configure))
    }

    /// Where `window` travels when it is minimised: the dock tile its client or the shell set,
    /// else the bottom centre of its output (a point). None when it is on no plugged output.
    pub fn minimise_destination(&self, window: &Window) -> Option<MinimiseTarget> {
        let output = self.output_of(window)?;
        self.set_minimise_target(window.id).or_else(|| {
            self.output_rect(&output)
                .map(|full| dock_tile(&output, full))
        })
    }

    /// The window goes to the dock; focus looks again at what the person can see.
    pub(super) fn minimise(self, window: &Window) -> Moved {
        let (Some(restore), Some(output)) = (window.state.restore(), self.output_of(window)) else {
            return Moved::of(self);
        };
        let Some(target) = self.minimise_destination(window) else {
            return Moved::of(self);
        };
        let shown = self.spaces.shown_on(&output).cloned();
        self.with_state(window.id, WindowState::Minimised { restore })
            .then(|model| match shown {
                Some(space) => model.feed(FocusIn::SpaceShown {
                    output: output.clone(),
                    space: space.id,
                }),
                None => Moved::of(model),
            })
            .then(|model| model.relaid_out(window.id, Cause::Minimise(target)))
    }

    /// A window asked to go full screen. Only the key window is heard: a background app that
    /// takes the screen with a fake dialog is the hole this closes (CASC-07, decided 2026-10-16),
    /// so the ask is refused and noted. The person's own hand (the shell's window menu and desktop
    /// asks, Split View) goes through `full_screened` and is never refused.
    pub(super) fn enter_full_screen(self, window: &Window, wanted: Option<OutputKey>) -> Moved {
        match self.focus.key == Some(window.id) {
            true => self.full_screened(window, wanted),
            false => Moved::of(self).with(Effect::Note(CoreNote::RefusedFullScreen(window.id))),
        }
    }

    /// The shell, on the person's behalf, asks `id` to go full screen or leave it (the window menu,
    /// the desktop protocol): not gated on the window being key.
    pub(super) fn full_screen_by_hand(self, id: WindowId, toggle: Toggle) -> Moved {
        let Some(window) = self.windows.get(&id).cloned() else {
            return Moved::of(self).with(Effect::Note(CoreNote::UnknownWindow(id)));
        };
        match toggle {
            Toggle::On => self.full_screened(&window, None),
            Toggle::Off => self.leave_full_screen(&window),
        }
    }

    /// The window takes a Space of its own on its output (or the output it asked for).
    pub(super) fn full_screened(self, window: &Window, wanted: Option<OutputKey>) -> Moved {
        let output = wanted
            .filter(|output| self.outputs.0.contains_key(output))
            .or_else(|| self.output_of(window));
        let (Some(restore), Some(output)) = (window.state.restore(), output) else {
            return Moved::of(self);
        };
        let id = window.id;
        let (model, space) = self.with_space(&output, SpaceContent::FullScreen(id));
        let mut model = model;
        if let Some(held) = model.windows.get_mut(&id) {
            held.place = SpacePlace::On(space.clone());
            held.home = None;
            held.state = WindowState::FullScreen {
                space: space.clone(),
                restore,
            };
        }
        Moved::of(model)
            .then(|model| model.show(&space))
            .then(|model| model.relaid_out(id, Cause::Configure))
    }

    /// The window leaves full screen: back to the state it had, its Space gone. A Split View
    /// ends with it: both windows go back to the states they had.
    fn leave_full_screen(self, window: &Window) -> Moved {
        let WindowState::FullScreen { space, .. } = window.state.clone() else {
            return Moved::of(self);
        };
        let members = self.full_screen_members(&space);
        let ids: Vec<WindowId> = members.iter().map(|(id, _)| *id).collect();
        let restored = members
            .into_iter()
            .fold(Moved::of(self), |moved, (id, back)| {
                moved.then(|model| model.with_state(id, back.into_state()))
            });
        restored
            .then(|model| model.choice_over_for(&ids))
            .then(|model| model.retire_full_screen(&space))
            .then(|model| {
                ids.into_iter().fold(Moved::of(model), |moved, id| {
                    moved.then(|model| model.relaid_out(id, Cause::Configure))
                })
            })
    }

    /// A window asked for activation. A valid token brings it back from the dock, shows its
    /// Space (when `spaces.switch_on_activate` says to), and keys it if the person can then
    /// see it; anything else only asks for attention and is noted (A7). A missing token is a
    /// stale one.
    fn activate_window(self, window: &Window, token: TokenCheck) -> Moved {
        let id = window.id;
        let app = window.app.clone();
        let back = match &window.state {
            WindowState::Minimised { restore } => Some(restore.clone().into_state()),
            _ => None,
        };
        let space = match &window.place {
            SpacePlace::On(space) => Some(space.clone()),
            SpacePlace::AllSpaces => None,
        };
        let follow = self.settings.spaces.switch_on_activate;
        let cause = self.restore_cause(window, token);
        let moved = match token {
            TokenCheck::Valid => self
                .unhide_app(&app)
                .then(|model| match back {
                    Some(state) => model.with_state(id, state),
                    None => Moved::of(model),
                })
                .then(|model| match (space, follow) {
                    (Some(space), Enabled::On) => model.show(&space),
                    _ => Moved::of(model),
                }),
            TokenCheck::Stale | TokenCheck::Missing => Moved::of(self),
        };
        moved
            .then(|model| model.feed_token(id, token))
            .then(|model| model.relaid_out(id, cause))
    }

    /// Why the scene is laid out again after `window` was asked to activate: a window that comes
    /// back from the dock flies out of its tile (the cause that minimised it, so motion reverses
    /// the flight); any other activation is a client's own configure.
    fn restore_cause(&self, window: &Window, token: TokenCheck) -> Cause {
        match (&window.state, token) {
            (WindowState::Minimised { .. }, TokenCheck::Valid) => self
                .minimise_destination(window)
                .map_or(Cause::Configure, Cause::Minimise),
            _ => Cause::Configure,
        }
    }

    /// Focus hears the token: a valid one the person can act on keys the window; any other,
    /// or one for a window out of sight, only asks for attention, and the refusal is noted.
    fn feed_token(self, id: WindowId, token: TokenCheck) -> Moved {
        let seen = self.sees_window(id);
        match (token, seen) {
            (TokenCheck::Valid, true) => self.feed(FocusIn::Token {
                window: id,
                token: TokenCheck::Valid,
            }),
            _ => self
                .feed(FocusIn::Token {
                    window: id,
                    token: TokenCheck::Stale,
                })
                .with(Effect::Note(CoreNote::RefusedActivation(id))),
        }
    }

    /// The scene is laid out again for `cause`, and the window is told its new states.
    pub(super) fn relaid_out(self, id: WindowId, cause: Cause) -> Moved {
        let outputs = self.outputs_of(&[id]);
        let configure = self.configure_effect(id);
        self.redrawn(cause, outputs).with_all(configure)
    }
}

/// The rectangle a window returns to when it leaves a tile or a fill: its own, else the one
/// it would get centred at its size.
fn floor_rect(state: &WindowState, usable: LogicalRect) -> LogicalRect {
    match state {
        WindowState::Floating(rect) => *rect,
        WindowState::Tiled { restore, .. } | WindowState::Filled { restore } => *restore,
        WindowState::New(size) => centred(*size, usable),
        WindowState::FullScreen { .. } | WindowState::Minimised { .. } => usable,
    }
}

/// The floating rectangle behind a state a window returns to: its own, else the one it would
/// get centred at its size.
fn restore_rect(restore: &Restore, usable: LogicalRect) -> LogicalRect {
    match restore {
        Restore::Floating(rect) => *rect,
        Restore::Tiled { restore, .. } | Restore::Filled { restore } => *restore,
        Restore::New(size) => centred(*size, usable),
    }
}

/// Where a minimised window travels to: the bottom centre of its output, a point. The dock's
/// own tile is not known to the model.
fn dock_tile(output: &OutputKey, full: LogicalRect) -> MinimiseTarget {
    let x = i64::from(full.origin.x) + i64::from(full.size.w / 2);
    let y = i64::from(full.origin.y) + i64::from(full.size.h);
    let clamp = |v: i64| i32::try_from(v).unwrap_or(i32::MAX);
    MinimiseTarget {
        output: output.clone(),
        rect: LogicalRect::new(
            LogicalPoint {
                x: clamp(x),
                y: clamp(y),
            },
            LogicalSize { w: 0, h: 0 },
        ),
    }
}
