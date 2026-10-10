//! Questions the handlers ask of the model: which output a window is on, where a new window
//! lands, how much room an output has.

use super::Model;
use crate::focus::FocusView;
use crate::geometry::{LogicalPoint, LogicalRect, LogicalSize};
use crate::ids::OutputKey;
use crate::output::OutputPower;
use crate::space::{Space, SpaceContent};
use crate::window::{SpacePlace, Window};
use porter_core::SpaceId;

impl Model {
    /// What the focus machine reads.
    pub(super) fn view(&self) -> FocusView<'_> {
        FocusView {
            windows: &self.windows,
            spaces: &self.spaces,
            apps: &self.apps,
        }
    }

    /// Whether the person can see the window now: on a Space an output shows, not in the dock,
    /// and its app not hidden.
    pub(super) fn is_seen(&self, window: &Window) -> bool {
        self.view().is_visible(window)
    }

    /// Whether the person can see window `id` now: it is mapped, on a Space an output shows, not
    /// in the dock, and its app not hidden.
    pub fn shows(&self, id: crate::ids::WindowId) -> bool {
        self.windows
            .get(&id)
            .is_some_and(|window| self.is_seen(window))
    }

    /// The output a window is on: its Space's, or its home for a window on every Space.
    pub fn output_of(&self, window: &Window) -> Option<OutputKey> {
        match &window.place {
            SpacePlace::On(id) => self.spaces.get(id).map(|space| space.output.clone()),
            SpacePlace::AllSpaces => window.home.clone(),
        }
    }

    /// The output new windows and the keyboard shortcuts go to: the leftmost lit output (the
    /// topmost, then by key, among equals), else the leftmost.
    pub(super) fn primary_output(&self) -> Option<OutputKey> {
        self.outputs
            .0
            .iter()
            .min_by_key(|(key, state)| {
                let config = state.config;
                let dark = matches!(config.power, OutputPower::Off);
                (dark, config.position.x, config.position.y, (*key).clone())
            })
            .map(|(key, _)| key.clone())
    }

    /// Where the pointer starts: the centre of the primary output (never a corner, where it would
    /// dwell in a hot corner before anyone touched the mouse).
    pub fn pointer_start(&self) -> Option<LogicalPoint> {
        self.primary_output()
            .and_then(|key| self.output_rect(&key))
            .map(|rect| rect.centre())
    }

    /// The output the key window is on, else the primary one: where the person is working.
    pub(super) fn working_output(&self) -> Option<OutputKey> {
        self.focus
            .key
            .and_then(|key| self.windows.get(&key))
            .and_then(|window| self.output_of(window))
            .or_else(|| self.primary_output())
    }

    /// The Space a new window of `output` goes to: the one it shows when that is a desktop,
    /// else its first desktop.
    pub(super) fn landing_space(&self, output: &OutputKey) -> Option<SpaceId> {
        let is_desktop = |space: &&Space| matches!(space.content, SpaceContent::Desktop);
        self.spaces
            .shown_on(output)
            .filter(is_desktop)
            .or_else(|| self.spaces.row(output).into_iter().find(is_desktop))
            .map(|space| space.id.clone())
    }

    /// The output's whole rectangle in the global logical space.
    pub(super) fn output_rect(&self, output: &OutputKey) -> Option<LogicalRect> {
        self.outputs
            .0
            .get(output)
            .map(|state| state.config.logical_rect())
    }

    /// What `output` leaves for windows after the layers' exclusive zones.
    pub(super) fn usable_rect(&self, output: &OutputKey) -> Option<LogicalRect> {
        self.output_rect(output)
            .map(|full| self.layers.usable_area(output, full))
    }

    /// The room a toplevel that asks to open maximised or full screen is given before it maps:
    /// the usable area of, and the whole of, the output new windows go to. `None` with no
    /// output plugged.
    pub fn opening_room(&self) -> Option<OpeningRoom> {
        let output = self.working_output()?;
        let full = self.output_rect(&output)?;
        Some(OpeningRoom {
            maximised: self.layers.usable_area(&output, full).size,
            full_screen: full.size,
        })
    }

    /// The stack rank above every window's.
    pub(super) fn top_rank(&self) -> crate::window::StackRank {
        let above = self
            .windows
            .values()
            .map(|window| window.stack.0)
            .max()
            .map_or(0, |top| top.saturating_add(1));
        crate::window::StackRank(above)
    }
}

/// The sizes a window that opens maximised or full screen is first configured to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpeningRoom {
    /// What the layers' exclusive zones leave of the output.
    pub maximised: LogicalSize,
    /// The whole output.
    pub full_screen: LogicalSize,
}

/// `size` centred in `area`, never left or above it.
pub(super) fn centred(size: LogicalSize, area: LogicalRect) -> LogicalRect {
    let spare = |room: u32, used: u32| i32::try_from(room.saturating_sub(used) / 2).unwrap_or(0);
    LogicalRect::new(
        LogicalPoint {
            x: area.origin.x.saturating_add(spare(area.size.w, size.w)),
            y: area.origin.y.saturating_add(spare(area.size.h, size.h)),
        },
        size,
    )
}
