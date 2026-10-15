//! Where the shell's desktop manager puts windows and Spaces (P2-Q1): a window onto a named
//! Space, onto every Space or back onto the one it is on, and a Space to another place in its
//! output's row. A request that cannot be carried out changes nothing.

use super::Model;
use super::moved::Moved;
use crate::cause::Cause;
use crate::focus::FocusIn;
use crate::geometry::LogicalRect;
use crate::ids::{OutputKey, WindowId};
use crate::space::{Space, SpaceContent, SpaceIndex};
use crate::window::{SpacePlace, WindowState};
use casement_protocols::vocab::Assign;
use porter_core::SpaceId;
use std::collections::BTreeSet;

impl Model {
    /// The window is on `space` (a desktop or an agent's Space) and its home output is that
    /// Space's. A full-screen window keeps its own Space. Focus looks again when the key window
    /// left what the person can see.
    pub(super) fn window_to_space(self, window: WindowId, space: &SpaceId) -> Moved {
        let (Some(target), Some(held)) = (
            self.spaces.get(space).cloned(),
            self.windows.get(&window).cloned(),
        ) else {
            return Moved::of(self);
        };
        let ordinary = matches!(
            target.content,
            SpaceContent::Desktop | SpaceContent::Agent(_)
        );
        let own_space = matches!(held.state, WindowState::FullScreen { .. });
        if !ordinary || own_space || held.place == SpacePlace::On(space.clone()) {
            return Moved::of(self);
        }
        let was = self.output_of(&held);
        let outputs: Vec<OutputKey> = was
            .iter()
            .chain([&target.output])
            .cloned()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        let carry = self.carried_between(was.as_ref(), &target.output);
        let mut model = self;
        if let Some(moving) = model.windows.get_mut(&window) {
            moving.place = SpacePlace::On(space.clone());
            moving.home = None;
            // Onto another output: the window keeps its place relative to the output's corner.
            if let Some((dx, dy)) = carry {
                moving.state = shifted(&moving.state, dx, dy);
            }
        }
        let refocus = was
            .filter(|_| model.focus.key == Some(window) && !model.shows(window))
            .and_then(|output| {
                model
                    .spaces
                    .shown_on(&output)
                    .map(|shown| FocusIn::SpaceShown {
                        output: output.clone(),
                        space: shown.id.clone(),
                    })
            });
        Moved::of(model)
            .then(|model| match refocus {
                Some(input) => model.feed(input),
                None => Moved::of(model),
            })
            .then(|model| model.redrawn(Cause::Configure, outputs))
            .then(|model| {
                let configure = model.configure_effect(window);
                Moved::of(model).with_all(configure)
            })
    }

    /// How far a window's rectangles move when it goes from output `from` to output `to`: the
    /// difference of the two outputs' corners. None when it stays on one output or either is
    /// not plugged.
    fn carried_between(&self, from: Option<&OutputKey>, to: &OutputKey) -> Option<(i32, i32)> {
        let from = from.filter(|from| *from != to)?;
        let corner = |key: &OutputKey| Some(self.outputs.0.get(key)?.config.logical_rect().origin);
        let (a, b) = (corner(from)?, corner(to)?);
        Some((b.x.saturating_sub(a.x), b.y.saturating_sub(a.y)))
    }

    /// `AllSpaces` puts the window on every Space of the output it is on; `ThisSpace` pins a
    /// window that was on every Space to the desktop its output shows; `None` changes nothing.
    pub(super) fn window_assigned(self, window: WindowId, assign: Assign) -> Moved {
        let Some(held) = self.windows.get(&window).cloned() else {
            return Moved::of(self);
        };
        if matches!(held.state, WindowState::FullScreen { .. }) {
            return Moved::of(self);
        }
        match (assign, held.place.clone()) {
            (Assign::AllSpaces, SpacePlace::On(_)) => match self.output_of(&held) {
                Some(home) => self.placed(window, SpacePlace::AllSpaces, Some(home)),
                None => Moved::of(self),
            },
            (Assign::ThisSpace, SpacePlace::AllSpaces) => {
                let landing = held.home.as_ref().and_then(|home| self.landing_space(home));
                match landing {
                    Some(space) => self.placed(window, SpacePlace::On(space), None),
                    None => Moved::of(self),
                }
            }
            (Assign::AllSpaces | Assign::ThisSpace | Assign::None, _) => Moved::of(self),
        }
    }

    /// The window's place and home are these, and its output is laid out again.
    fn placed(self, window: WindowId, place: SpacePlace, home: Option<OutputKey>) -> Moved {
        let mut model = self;
        if let Some(held) = model.windows.get_mut(&window) {
            held.place = place;
            held.home = home;
        }
        model.relaid_out(window, Cause::Configure)
    }

    /// The desktop `space` takes place `index` in its output's row (past the end is the end);
    /// the others close up around it.
    pub(super) fn space_moved(self, space: &SpaceId, index: SpaceIndex) -> Moved {
        let Some(moving) = self.spaces.get(space).cloned() else {
            return Moved::of(self);
        };
        if !matches!(moving.content, SpaceContent::Desktop) {
            return Moved::of(self);
        }
        let mut order: Vec<SpaceId> = self
            .spaces
            .row(&moving.output)
            .into_iter()
            .map(|held| held.id.clone())
            .filter(|id| id != space)
            .collect();
        let at = usize::try_from(index.0)
            .unwrap_or(usize::MAX)
            .min(order.len());
        order.insert(at, space.clone());
        let spaces =
            order
                .into_iter()
                .enumerate()
                .fold(self.spaces.clone(), |book, (place, id)| {
                    let position = SpaceIndex(u32::try_from(place).unwrap_or(u32::MAX));
                    match book.get(&id).cloned() {
                        Some(held) => book.with_space(Space { position, ..held }),
                        None => book,
                    }
                });
        let model = Model { spaces, ..self };
        let output = moving.output;
        Moved::of(model).then(|model| model.redrawn(Cause::SpaceSwitch, vec![output]))
    }
}

/// `state` with the rectangles it holds moved by `dx`, `dy` (a window going to another output).
/// A window not yet placed, full screen or in the dock holds none worth moving.
fn shifted(state: &WindowState, dx: i32, dy: i32) -> WindowState {
    let moved = |rect: &LogicalRect| {
        LogicalRect::new(
            crate::geometry::LogicalPoint {
                x: rect.origin.x.saturating_add(dx),
                y: rect.origin.y.saturating_add(dy),
            },
            rect.size,
        )
    };
    match state {
        WindowState::Floating(rect) => WindowState::Floating(moved(rect)),
        WindowState::Tiled { zone, restore } => WindowState::Tiled {
            zone: *zone,
            restore: moved(restore),
        },
        WindowState::Filled { restore } => WindowState::Filled {
            restore: moved(restore),
        },
        other => other.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::{LogicalPoint, LogicalSize};
    use casement_protocols::vocab::TileZone;

    fn rect(x: i32, y: i32) -> LogicalRect {
        LogicalRect::new(LogicalPoint { x, y }, LogicalSize { w: 40, h: 30 })
    }

    #[test]
    fn the_rectangles_a_window_holds_move_with_it() {
        let new = WindowState::New(LogicalSize { w: 5, h: 5 });
        // name, state, want
        let cases = [
            (
                "floating",
                WindowState::Floating(rect(10, 20)),
                WindowState::Floating(rect(1930, 20)),
            ),
            (
                "tiled keeps the zone and moves the way back",
                WindowState::Tiled {
                    zone: TileZone::Left,
                    restore: rect(10, 20),
                },
                WindowState::Tiled {
                    zone: TileZone::Left,
                    restore: rect(1930, 20),
                },
            ),
            (
                "filled moves the way back",
                WindowState::Filled {
                    restore: rect(0, 0),
                },
                WindowState::Filled {
                    restore: rect(1920, 0),
                },
            ),
            ("unplaced stays", new.clone(), new),
        ];
        for (name, state, want) in cases {
            assert_eq!(shifted(&state, 1920, 0), want, "{name}");
        }
    }
}
