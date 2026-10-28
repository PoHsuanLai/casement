//! What the model tells the shell's protocol layer (the `Effect::Shell` of this crate).

use crate::app::ForceQuitRow;
use crate::geometry::{LogicalPoint, LogicalRect};
use crate::ids::{LayerKey, OutputKey, WindowId};
use crate::overview::Overview;
use crate::window::{MenuItem, enabled_mask};
use casement_protocols::vocab::{DragOutcome, HeldMods, ScreenEdge, ShellRole, Shown, SplitSide};

/// Something the shell's protocol layer records or sends.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ShellNote {
    /// A layer surface took a role: the protocol layer records which surface holds it, so
    /// the events addressed to a role (modifiers) find it.
    Role(LayerKey, ShellRole),
    /// `window` now minimises into this rectangle (global logical pixels); the protocol layer
    /// has nothing to send until P3's dock bounce, so it only keeps the table.
    MinimiseTarget(WindowId, LogicalRect),
    /// The modifier keys held changed (`quire_shell_v1.modifiers`, to the role surfaces).
    Modifiers(HeldMods),
    /// An edge reveal (the dock or the bar over a full-screen window) opened or closed on
    /// `output` (`reveal_on_output` to a shell that bound version 7, `reveal` to an older one).
    Reveal(OutputKey, ScreenEdge, Shown),
    /// Show the window menu: the shell draws it and answers with
    /// [`ShellRequest::WindowMenuPick`](super::ShellRequest::WindowMenuPick).
    WindowMenu(WindowMenuShow),
    /// A window went full screen to share the screen in a Split View: the shell shows the other
    /// windows of the desktop to pick from and answers with
    /// [`ShellRequest::SplitPick`](super::ShellRequest::SplitPick) or
    /// [`ShellRequest::SplitCancel`](super::ShellRequest::SplitCancel).
    SplitChoose(SplitChoices),
    /// The choosing for this first window is over (a pick, a cancel, or the window left): the
    /// shell takes its picker away.
    SplitChoiceEnd(WindowId),
    /// The Force Quit Applications window was asked for (Option-Command-Escape): the apps that
    /// are running, each answering or not. The shell draws the window and answers with
    /// `desktop_app.force_quit` for the app the person picks.
    ForceQuitPanel(Vec<ForceQuitRow>),
    /// Mission Control moved to this state (`overview_state`, Mission Control's mode): the shell
    /// draws its Spaces bar while it is not closed.
    Overview(Overview),
    /// The window dragged in Mission Control is over the Space `space` of a target on `output`,
    /// or over none (`drop_target`, version 2).
    DropTarget {
        /// The output whose bar holds the target.
        output: OutputKey,
        /// The Space, or none.
        space: Option<porter_core::SpaceId>,
    },
    /// The window dragged in Mission Control entered (`Shown`) or left (`Hidden`) the new-Space
    /// target on `output` (`drop_target_new`, version 3).
    DropTargetNew {
        /// The output whose bar holds the target.
        output: OutputKey,
        /// Whether the dragged window is over it.
        over: Shown,
    },
    /// A window drag started in Mission Control; one note per output Mission Control is up on
    /// (`window_drag_begin`, version 4).
    WindowDragBegin {
        /// The output.
        output: OutputKey,
        /// The window being dragged.
        window: WindowId,
    },
    /// The window drag in Mission Control is over (`window_drag_end`, version 4); one note per
    /// output, after the notes that cleared the highlight.
    WindowDragEnd {
        /// The output.
        output: OutputKey,
        /// The window that was dragged.
        window: WindowId,
        /// Where it went.
        outcome: DragOutcome,
    },
}

/// The other half of a Split View is being chosen.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct SplitChoices {
    /// The window already full screen on its half.
    pub first: WindowId,
    /// The half it took.
    pub side: SplitSide,
    /// The windows that can fill the other half, most recently used first.
    pub candidates: Vec<WindowId>,
}

impl SplitChoices {
    /// The choice for the half beside `first`, offering `candidates` in order.
    ///
    /// ```
    /// use casement_core::WindowId;
    /// use casement_core::shell::SplitChoices;
    /// use casement_protocols::vocab::SplitSide;
    ///
    /// let choices = SplitChoices::new(WindowId(1), SplitSide::Left, vec![WindowId(2)]);
    /// assert_eq!(choices.candidates, [WindowId(2)]);
    /// ```
    pub fn new(first: WindowId, side: SplitSide, candidates: Vec<WindowId>) -> Self {
        Self {
            first,
            side,
            candidates,
        }
    }
}

/// The window menu to show: for which window, where, and which entries can be picked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowMenuShow {
    /// The window the menu is for.
    pub window: WindowId,
    /// Where it opens, in global logical pixels.
    pub at: LogicalPoint,
    /// Every entry in menu order, each enabled or dimmed.
    pub items: Vec<MenuItem>,
}

impl WindowMenuShow {
    /// The enabled entries as the wire's bit set.
    ///
    /// ```
    /// use casement_core::WindowId;
    /// use casement_core::geometry::LogicalPoint;
    /// use casement_core::shell::WindowMenuShow;
    /// use casement_core::window::MenuItem;
    /// use casement_protocols::vocab::{WindowEntry, WireCode};
    ///
    /// let show = WindowMenuShow {
    ///     window: WindowId(1),
    ///     at: LogicalPoint { x: 0, y: 0 },
    ///     items: vec![MenuItem { entry: WindowEntry::Fill, enabled: true }],
    /// };
    /// assert_eq!(show.enabled(), 1 << WindowEntry::Fill.code());
    /// ```
    pub fn enabled(&self) -> u32 {
        enabled_mask(&self.items)
    }
}
