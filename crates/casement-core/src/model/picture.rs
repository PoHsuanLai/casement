//! Questions about Spaces that are not shown, for the pictures of the Spaces bar and for
//! dropping a window on a new Space.

use super::Model;
use super::moved::Moved;
use crate::app::AppVisibility;
use crate::ids::{OutputKey, WindowId};
use crate::space::SpaceContent;
use crate::window::{SpacePlace, WindowState};
use porter_core::SpaceId;

impl Model {
    /// The windows a picture of Space `id` shows, back first: those on it that are not in the
    /// dock and whose app is not hidden, whether or not an output shows the Space now, and the
    /// windows on every Space of its output. Empty for a Space the model does not have.
    pub fn pictured(&self, id: &SpaceId) -> Vec<WindowId> {
        let Some(space) = self.spaces.get(id) else {
            return Vec::new();
        };
        let mut held: Vec<_> = self
            .windows
            .values()
            .filter(|window| match &window.place {
                SpacePlace::On(on) => on == id,
                SpacePlace::AllSpaces => window.home.as_ref() == Some(&space.output),
            })
            .filter(|window| !matches!(window.state, WindowState::Minimised { .. }))
            .filter(|window| {
                !self
                    .apps
                    .get(&window.app)
                    .is_some_and(|app| matches!(app.visibility, AppVisibility::Hidden { .. }))
            })
            .collect();
        held.sort_by_key(|window| (window.stack, window.id));
        held.into_iter().map(|window| window.id).collect()
    }

    /// A new desktop Space at the end of `output`'s row, and the window moved to it (the drop
    /// on the Spaces bar's "add desktop" tile). A window that owns its Space (full screen) or
    /// an output that is not plugged changes nothing, and no Space is made.
    pub(super) fn window_to_new_space(self, window: WindowId, output: &OutputKey) -> Moved {
        let movable = self.outputs.0.contains_key(output)
            && self
                .windows
                .get(&window)
                .is_some_and(|held| !matches!(held.state, WindowState::FullScreen { .. }));
        match movable {
            false => Moved::of(self),
            true => {
                let (model, id) = self.with_space(output, SpaceContent::Desktop);
                model.window_to_space(window, &id)
            }
        }
    }
}
