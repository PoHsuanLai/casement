//! Picture in picture in the model (P3-SH): such a window joins every Space of its output the
//! moment it is known to be one, keeps its aspect ratio while the person resizes it, and snaps
//! to the nearest corner when they let go. The arithmetic is `window::pip`.

use super::Model;
use super::moved::Moved;
use crate::cause::Cause;
use crate::geometry::LogicalRect;
use crate::ids::WindowId;
use crate::window::{SpacePlace, WindowState, fit_aspect, floats_above, snap_corner};

impl Model {
    /// A window whose role or title just changed: when it is picture in picture and sits on one
    /// Space, it moves to every Space of its output. A window that is full screen, minimised or
    /// already on every Space is left alone, and so is one whose app already has a picture in
    /// picture window on every Space (one per app, CASC-11).
    pub(super) fn pip_joined(self, id: WindowId) -> Moved {
        let Some(window) = self.windows.get(&id) else {
            return Moved::of(self);
        };
        let app = window.app.clone();
        let spread_already = self.windows.values().any(|other| {
            other.id != id
                && other.app == app
                && other.place == SpacePlace::AllSpaces
                && floats_above(other)
        });
        let joins = floats_above(window)
            && !spread_already
            && matches!(window.place, SpacePlace::On(_))
            && matches!(window.state, WindowState::New(_) | WindowState::Floating(_));
        let home = self.output_of(window);
        match (joins, home) {
            (true, Some(home)) => {
                let mut model = self;
                if let Some(held) = model.windows.get_mut(&id) {
                    held.place = SpacePlace::AllSpaces;
                    held.home = Some(home);
                }
                model.relaid_out(id, Cause::Configure)
            }
            _ => Moved::of(self),
        }
    }

    /// `wanted`, the rectangle a drag asks for, held to the aspect ratio of a picture in
    /// picture window's current one; any other window's is returned as it is.
    pub(super) fn pip_fitted(&self, id: WindowId, wanted: LogicalRect) -> LogicalRect {
        match self.windows.get(&id) {
            Some(window) if floats_above(window) => match &window.state {
                WindowState::Floating(held) => fit_aspect(*held, wanted),
                _ => wanted,
            },
            _ => wanted,
        }
    }

    /// The person let go of a dragged window: a picture in picture window snaps to the nearest
    /// corner of the usable area of its output.
    pub(super) fn drag_ended(self, id: WindowId) -> Moved {
        let snapped = self.windows.get(&id).and_then(|window| {
            let WindowState::Floating(rect) = &window.state else {
                return None;
            };
            let area = self
                .output_of(window)
                .and_then(|output| self.usable_rect(&output))?;
            floats_above(window)
                .then(|| snap_corner(*rect, area))
                .filter(|snapped| snapped != rect)
        });
        match snapped {
            Some(rect) => self
                .with_state(id, WindowState::Floating(rect))
                .then(|model| model.relaid_out(id, Cause::Drag)),
            None => Moved::of(self),
        }
    }
}
