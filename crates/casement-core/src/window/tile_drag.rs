//! Dragging a window to a screen edge to tile it (Sequoia). A pure machine: the loop tells it
//! the pointer's place and the modifiers held on each move, and asks it at the release what the
//! window is to do.
//!
//! - Left or right edge: that half; the top edge: Fill (or the top half when
//!   `windows.tile_top_fills` is off); near a corner: that quarter. The bottom edge tiles only
//!   at its corners (the dock lives there).
//! - Option held (`windows.tile_option_drag`): the zones show without reaching an edge, the one
//!   under the pointer arming (thirds of the output, the centre none).
//! - An edge counts only where no other output lies beyond it: the shared edge of two displays
//!   is where the pointer crosses, not where a window tiles.
//!
//! An edge is measured from the output's whole rectangle, so the menu bar at the top and a dock
//! at the side (both reserved from the usable area) do not push the edge in: the pointer
//! reaching the screen's own edge is what counts, and the zone is then laid out in the usable
//! area (`casement_layout::zone_rect`).

use crate::WindowId;
use crate::geometry::{LogicalPoint, LogicalRect};
use crate::settings::{Enabled, WindowSettings};
use casement_protocols::vocab::{MacMods, PlaceTarget, TileZone};

/// How far from an output's edge the pointer counts as at it, in logical pixels.
pub const EDGE_BAND: u32 = 3;

/// How far along an edge from a corner the pointer counts as in the corner, in logical pixels
/// (held to a quarter of the edge on a small output).
pub const CORNER_REACH: u32 = 96;

/// Where a drag tiles a window to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TileTarget {
    /// A half or a quarter of the usable area.
    Zone(TileZone),
    /// All of the usable area.
    Fill,
}

impl TileTarget {
    /// The placement that carries it out.
    pub fn place(self) -> PlaceTarget {
        match self {
            TileTarget::Fill => PlaceTarget::Fill,
            TileTarget::Zone(zone) => match zone {
                TileZone::Left => PlaceTarget::Left,
                TileZone::Right => PlaceTarget::Right,
                TileZone::Top => PlaceTarget::Top,
                TileZone::Bottom => PlaceTarget::Bottom,
                TileZone::TopLeft => PlaceTarget::TopLeft,
                TileZone::TopRight => PlaceTarget::TopRight,
                TileZone::BottomLeft => PlaceTarget::BottomLeft,
                TileZone::BottomRight => PlaceTarget::BottomRight,
            },
        }
    }
}

/// The settings that decide where a drag tiles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TileKnobs {
    /// `windows.tile_edges`: the edges and corners tile.
    pub edges: Enabled,
    /// `windows.tile_top_fills`: the top edge fills instead of tiling the top half.
    pub top_fills: Enabled,
    /// `windows.tile_option_drag`: Option shows the zones without reaching an edge.
    pub option_drag: Enabled,
}

impl TileKnobs {
    /// The knobs `settings` give.
    pub fn of(settings: &WindowSettings) -> TileKnobs {
        TileKnobs {
            edges: settings.tile_edges,
            top_fills: settings.tile_top_fills,
            option_drag: settings.tile_option_drag,
        }
    }
}

/// The outputs a drag is tiling among: the window's own, and the others (which only decide
/// which edges are open).
#[derive(Debug, Clone, Copy)]
pub struct TileScene<'a> {
    /// The whole rectangle of the output the window is on.
    pub home: LogicalRect,
    /// The whole rectangles of every other output.
    pub others: &'a [LogicalRect],
}

impl TileScene<'_> {
    /// Whether nothing lies at `p`: the edge next to it leads off the desktop.
    fn open(&self, p: LogicalPoint) -> bool {
        !self.others.iter().any(|other| other.contains(p))
    }
}

/// A window to tile, and where.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TileDrop {
    /// The window.
    pub window: WindowId,
    /// Where it goes (a tile zone or Fill).
    pub target: TileTarget,
}

/// The tile drag of a window being moved.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TileDrag {
    /// No move in progress.
    #[default]
    Idle,
    /// A window is moving and no zone is armed.
    Moving {
        /// The window.
        window: WindowId,
    },
    /// A window is moving over a zone: its preview shows, and a release tiles it.
    Previewing {
        /// The window.
        window: WindowId,
        /// The zone armed.
        target: TileTarget,
    },
}

impl TileDrag {
    /// A move of `window` begins.
    pub fn begin(window: WindowId) -> TileDrag {
        TileDrag::Moving { window }
    }

    /// The pointer is at `at` with `mods` held.
    pub fn moved(
        self,
        at: LogicalPoint,
        mods: MacMods,
        scene: &TileScene<'_>,
        knobs: &TileKnobs,
    ) -> TileDrag {
        let (TileDrag::Moving { window } | TileDrag::Previewing { window, .. }) = self else {
            return TileDrag::Idle;
        };
        match target_at(at, mods, scene, knobs) {
            Some(target) => TileDrag::Previewing { window, target },
            None => TileDrag::Moving { window },
        }
    }

    /// The window being moved, while a move is in progress.
    pub fn window(&self) -> Option<WindowId> {
        match self {
            TileDrag::Idle => None,
            TileDrag::Moving { window } | TileDrag::Previewing { window, .. } => Some(*window),
        }
    }

    /// The zone armed, if any.
    pub fn target(&self) -> Option<TileTarget> {
        match self {
            TileDrag::Previewing { target, .. } => Some(*target),
            TileDrag::Idle | TileDrag::Moving { .. } => None,
        }
    }

    /// The move ends: the drag is over, and the window is to tile when a zone was armed.
    pub fn released(self) -> (TileDrag, Option<TileDrop>) {
        match self {
            TileDrag::Previewing { window, target } => {
                (TileDrag::Idle, Some(TileDrop { window, target }))
            }
            TileDrag::Idle | TileDrag::Moving { .. } => (TileDrag::Idle, None),
        }
    }
}

/// The zone the pointer at `at` arms, if any.
pub fn target_at(
    at: LogicalPoint,
    mods: MacMods,
    scene: &TileScene<'_>,
    knobs: &TileKnobs,
) -> Option<TileTarget> {
    if !scene.home.contains(at) {
        return None;
    }
    let option = mods.holds(MacMods::OPTION) && knobs.option_drag == Enabled::On;
    match (option, knobs.edges) {
        (true, _) => under_pointer(at, scene.home, *knobs),
        (false, Enabled::On) => at_edge(at, scene, *knobs),
        (false, Enabled::Off) => None,
    }
}

/// The top edge's target: Fill, or the top half.
fn top_of(knobs: TileKnobs) -> TileTarget {
    match knobs.top_fills {
        Enabled::On => TileTarget::Fill,
        Enabled::Off => TileTarget::Zone(TileZone::Top),
    }
}

/// With Option held: the zone of the third of `home` the pointer is in.
fn under_pointer(at: LogicalPoint, home: LogicalRect, knobs: TileKnobs) -> Option<TileTarget> {
    let third = |from: i32, len: u32, p: i32| {
        let along = i64::from(p) - i64::from(from);
        usize::try_from((along * 3 / i64::from(len.max(1))).clamp(0, 2)).unwrap_or(0)
    };
    let row = third(home.origin.y, home.size.h, at.y);
    let column = third(home.origin.x, home.size.w, at.x);
    [
        [
            Some(TileTarget::Zone(TileZone::TopLeft)),
            Some(top_of(knobs)),
            Some(TileTarget::Zone(TileZone::TopRight)),
        ],
        [
            Some(TileTarget::Zone(TileZone::Left)),
            None,
            Some(TileTarget::Zone(TileZone::Right)),
        ],
        [
            Some(TileTarget::Zone(TileZone::BottomLeft)),
            Some(TileTarget::Zone(TileZone::Bottom)),
            Some(TileTarget::Zone(TileZone::BottomRight)),
        ],
    ][row][column]
}

/// One side of an axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Side {
    /// Left or top.
    Near,
    /// Right or bottom.
    Far,
}

/// The corner at `across` and `down`.
fn corner(across: Side, down: Side) -> TileTarget {
    match (across, down) {
        (Side::Near, Side::Near) => TileTarget::Zone(TileZone::TopLeft),
        (Side::Far, Side::Near) => TileTarget::Zone(TileZone::TopRight),
        (Side::Near, Side::Far) => TileTarget::Zone(TileZone::BottomLeft),
        (Side::Far, Side::Far) => TileTarget::Zone(TileZone::BottomRight),
    }
}

/// Which open side of an axis `p` is at the edge of: within `EDGE_BAND` of the near end of
/// `[from, from + len)`, or of the far end, and the point just beyond that end (`beyond`)
/// leads off the desktop.
fn at_end(
    p: i32,
    from: i32,
    len: u32,
    beyond: impl Fn(i64) -> LogicalPoint,
    scene: &TileScene<'_>,
) -> Option<Side> {
    let (p, from, len) = (i64::from(p), i64::from(from), i64::from(len));
    let band = i64::from(EDGE_BAND);
    let near = p - from < band && scene.open(beyond(from - 1));
    let far = from + len - 1 - p < band && scene.open(beyond(from + len));
    match (near, far) {
        (true, _) => Some(Side::Near),
        (_, true) => Some(Side::Far),
        (false, false) => None,
    }
}

/// Which corner end of an axis `p` is within `reach` of (and open beyond), if either.
fn in_reach(
    p: i32,
    from: i32,
    len: u32,
    beyond: impl Fn(i64) -> LogicalPoint,
    scene: &TileScene<'_>,
) -> Option<Side> {
    let reach = i64::from(CORNER_REACH.min(len / 4));
    let (p, from, len) = (i64::from(p), i64::from(from), i64::from(len));
    let near = p - from < reach && scene.open(beyond(from - 1));
    let far = from + len - 1 - p < reach && scene.open(beyond(from + len));
    match (near, far) {
        (true, _) => Some(Side::Near),
        (_, true) => Some(Side::Far),
        (false, false) => None,
    }
}

/// The zone of the edge or corner the pointer is at.
fn at_edge(at: LogicalPoint, scene: &TileScene<'_>, knobs: TileKnobs) -> Option<TileTarget> {
    let home = scene.home;
    let clamp = |v: i64| i32::try_from(v).unwrap_or(i32::MAX);
    // The point past an end of the x axis (or the y axis) at the pointer's other coordinate.
    let past_x = |x: i64| LogicalPoint {
        x: clamp(x),
        y: at.y,
    };
    let past_y = |y: i64| LogicalPoint {
        x: at.x,
        y: clamp(y),
    };
    let across = at_end(at.x, home.origin.x, home.size.w, past_x, scene);
    let down = at_end(at.y, home.origin.y, home.size.h, past_y, scene);
    match (across, down) {
        (Some(across), Some(down)) => Some(corner(across, down)),
        (Some(across), None) => {
            let down = in_reach(at.y, home.origin.y, home.size.h, past_y, scene);
            Some(down.map_or(
                match across {
                    Side::Near => TileTarget::Zone(TileZone::Left),
                    Side::Far => TileTarget::Zone(TileZone::Right),
                },
                |down| corner(across, down),
            ))
        }
        (None, Some(down)) => {
            let across = in_reach(at.x, home.origin.x, home.size.w, past_x, scene);
            match (across, down) {
                (Some(across), down) => Some(corner(across, down)),
                (None, Side::Near) => Some(top_of(knobs)),
                (None, Side::Far) => None,
            }
        }
        (None, None) => None,
    }
}
