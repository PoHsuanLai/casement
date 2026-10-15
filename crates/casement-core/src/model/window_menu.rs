//! The window menu: a window asks for it (the green light's dwell, a client-side frame's
//! `show_window_menu`), the shell draws it, and a pick comes back as a request to the model.
//! What each entry is and when it is enabled is `window::menu`'s.

use super::Model;
use super::moved::Moved;
use crate::effect::Effect;
use crate::event::ToplevelRequest;
use crate::geometry::LogicalPoint;
use crate::ids::WindowId;
use crate::shell::{ShellNote, WindowMenuShow};
use crate::window::{MenuAction, MenuEntry, SplitSupport, Window, is_offered, menu_for};

/// The model can put a window in a Split View (`model/split.rs`), so the Full Screen Tile
/// entries are offered.
const SPLIT: SplitSupport = SplitSupport::Available;

impl Model {
    /// `window` asked for its menu at `at`: the shell is told which entries it has and which
    /// are enabled.
    pub(super) fn window_menu(self, window: &Window, at: LogicalPoint) -> Moved {
        let show = WindowMenuShow {
            window: window.id,
            at,
            items: menu_for(&window.state, SPLIT),
        };
        Moved::of(self).with(Effect::Shell(ShellNote::WindowMenu(show)))
    }

    /// The shell says `entry` was picked in `window`'s menu. An entry that is dimmed now (the
    /// window changed while the menu was open) or a window that is gone changes nothing.
    pub(super) fn window_menu_pick(self, id: WindowId, entry: MenuEntry) -> Moved {
        let Some(window) = self.windows.get(&id).cloned() else {
            return Moved::of(self);
        };
        if !is_offered(&menu_for(&window.state, SPLIT), entry) {
            return Moved::of(self);
        }
        match entry.action() {
            MenuAction::FullScreen(toggle) => self.request(
                id,
                ToplevelRequest::FullScreen {
                    toggle,
                    output: None,
                },
            ),
            MenuAction::Place(target) => self.place(&window, target),
            MenuAction::SplitTile(side) => self.split_view(&window, side),
        }
    }
}
