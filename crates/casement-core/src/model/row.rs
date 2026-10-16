//! Where each window sits in its output's row of Spaces: what the renderer needs to draw the
//! row as one strip while a swipe moves it.

use super::Model;
use crate::ids::{OutputKey, WindowId};
use crate::window::SpacePlace;
use std::collections::BTreeMap;

impl Model {
    /// The column (the Space's place in `output`'s row, from 0) of each window that lives on one
    /// of its Spaces. A window on every Space has none: it stays where it is when the row moves.
    pub fn row_columns(&self, output: &OutputKey) -> BTreeMap<WindowId, u32> {
        let row = self.spaces.row(output);
        self.windows
            .iter()
            .filter_map(|(id, window)| {
                let SpacePlace::On(space) = &window.place else {
                    return None;
                };
                let at = row.iter().position(|held| &held.id == space)?;
                Some((*id, u32::try_from(at).ok()?))
            })
            .collect()
    }
}
