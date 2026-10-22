//! The two edges that reveal, the strip that triggers them and where the pointer is relative to
//! one.

use super::Cover;
use crate::geometry::{LogicalPoint, LogicalRect, LogicalSize};
use casement_protocols::vocab::{FullScreenBar, ScreenEdge, ShellRole};

/// How thick the trigger strip along the output's edge is, in logical pixels (the dock's
/// auto-hide strip, design/10 section 10.3.11; the bar uses the same).
pub const STRIP: u32 = 4;

/// An edge that reveals a surface in full screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum RevealEdge {
    /// The top edge: the surface with the role `bar`.
    Top,
    /// The bottom edge: the surface with the role `dock`.
    Bottom,
}

impl RevealEdge {
    /// Both edges.
    pub const ALL: [RevealEdge; 2] = [RevealEdge::Top, RevealEdge::Bottom];

    /// The edge as `reveal` names it on the wire.
    pub fn screen_edge(self) -> ScreenEdge {
        match self {
            RevealEdge::Top => ScreenEdge::Top,
            RevealEdge::Bottom => ScreenEdge::Bottom,
        }
    }

    /// The role of the surface this edge reveals.
    pub fn role(self) -> ShellRole {
        match self {
            RevealEdge::Top => ShellRole::Bar,
            RevealEdge::Bottom => ShellRole::Dock,
        }
    }

    /// Whether this edge reveals in full screen under `mode`, given `shown` (whether the output
    /// shows a full-screen Space with a surface to reveal). With `Keep` the bar stays up, so the
    /// top edge never covers; the dock's edge is not affected.
    ///
    /// ```
    /// use casement_core::reveal::{Cover, RevealEdge};
    /// use casement_protocols::vocab::FullScreenBar;
    ///
    /// let top = RevealEdge::Top;
    /// assert_eq!(top.covered(FullScreenBar::Keep, Cover::Yes), Cover::No);
    /// assert_eq!(top.covered(FullScreenBar::Reveal, Cover::Yes), Cover::Yes);
    /// assert_eq!(RevealEdge::Bottom.covered(FullScreenBar::Keep, Cover::Yes), Cover::Yes);
    /// ```
    pub fn covered(self, mode: FullScreenBar, shown: Cover) -> Cover {
        match (self, mode) {
            (RevealEdge::Top, FullScreenBar::Keep) => Cover::No,
            _ => shown,
        }
    }

    /// The trigger strip along this edge of `output`, across its whole width.
    ///
    /// ```
    /// use casement_core::geometry::{LogicalPoint, LogicalRect, LogicalSize};
    /// use casement_core::reveal::RevealEdge;
    ///
    /// let output = LogicalRect::new(LogicalPoint { x: 0, y: 0 }, LogicalSize { w: 100, h: 50 });
    /// let strip = RevealEdge::Bottom.strip(output);
    /// assert_eq!((strip.origin.y, strip.size.h), (46, 4));
    /// ```
    pub fn strip(self, output: LogicalRect) -> LogicalRect {
        let thick = STRIP.min(output.size.h);
        let size = LogicalSize {
            w: output.size.w,
            h: thick,
        };
        let down = i32::try_from(output.size.h - thick).unwrap_or(0);
        let y = match self {
            RevealEdge::Top => output.origin.y,
            RevealEdge::Bottom => output.origin.y.saturating_add(down),
        };
        LogicalRect::new(
            LogicalPoint {
                x: output.origin.x,
                y,
            },
            size,
        )
    }
}

/// Where the pointer is relative to an edge's surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Place {
    /// Neither in the strip nor on the surface.
    #[default]
    Away,
    /// In the trigger strip of the edge.
    InStrip,
    /// On the surface (or a pop-up of it).
    OnSurface,
}

/// Whether the pointer is over the edge's surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Over {
    /// It is.
    Surface,
    /// It is not.
    Elsewhere,
}

impl Place {
    /// The place of `pointer` for `edge` of `output`, given what the pointer is over. The strip
    /// wins: a pointer that is still at the edge has not left.
    ///
    /// ```
    /// use casement_core::geometry::{LogicalPoint, LogicalRect, LogicalSize};
    /// use casement_core::reveal::{Place, RevealEdge};
    /// use casement_core::reveal::Over;
    ///
    /// let output = LogicalRect::new(LogicalPoint { x: 0, y: 0 }, LogicalSize { w: 100, h: 50 });
    /// let at = |y| LogicalPoint { x: 10, y };
    /// assert_eq!(Place::of(RevealEdge::Top, output, at(1), Over::Elsewhere), Place::InStrip);
    /// assert_eq!(Place::of(RevealEdge::Top, output, at(20), Over::Surface), Place::OnSurface);
    /// assert_eq!(Place::of(RevealEdge::Top, output, at(20), Over::Elsewhere), Place::Away);
    /// ```
    pub fn of(edge: RevealEdge, output: LogicalRect, pointer: LogicalPoint, over: Over) -> Place {
        match (
            edge.strip(output).contains(pointer),
            output.contains(pointer),
            over,
        ) {
            (true, _, _) => Place::InStrip,
            (false, true, Over::Surface) => Place::OnSurface,
            _ => Place::Away,
        }
    }
}
