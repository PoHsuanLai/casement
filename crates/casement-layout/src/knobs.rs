//! The numbers layout reads that are not in the spec.

use casement_core::geometry::{LogicalPoint, LogicalSize};
use casement_core::settings::{CasementSettings, Enabled};
use ds_core::vocab::Fraction;
use ds_style::tokens::chrome::ChromeToken;

/// The margin tiled windows keep from each other and from the edges, in logical pixels.
const TILE_MARGIN: u32 = 8;

/// The gap between spread windows (03 section 1.8: 20 logical pixels).
const SPREAD_GAP: u32 = 20;

/// The smallest a spread window shrinks to (03 section 1.8: 0.15 of its size).
const SPREAD_FLOOR: Fraction = Fraction(150);

/// How many titlebar heights of a dragged-off window stay on screen across: enough to grab.
const GRIP_TITLEBARS_WIDE: u32 = 4;

/// Layout's tunables.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LayoutKnobs {
    /// How far each new window cascades from the previous one.
    pub cascade_step: LogicalPoint,
    /// The margin around tiled windows (zero when `windows.tile_margins` is off).
    pub tile_margin: u32,
    /// The gap between windows spread out in Mission Control.
    pub spread_gap: u32,
    /// The smallest a spread window may shrink to, as a share of its size.
    pub spread_floor: Fraction,
    /// How much of a floating window stays inside the usable area, from its top edge: a window
    /// may go off the left, right and bottom (never above the menu bar) while this much is
    /// left to grab.
    pub grip: LogicalSize,
}

impl LayoutKnobs {
    /// The knobs `settings` give: each new window cascades by one titlebar height (the Mac's
    /// step, from the chrome token so the stagger shows exactly one title strip), tiled windows
    /// keep their margin only while `windows.tile_margins` is on, the spread uses the Mac's
    /// gap and floor, and a floating window keeps a strip one titlebar tall and four wide on
    /// screen.
    pub fn of(settings: &CasementSettings) -> LayoutKnobs {
        let step = i32::from(ChromeToken::TitlebarHeight.length().0);
        LayoutKnobs {
            cascade_step: LogicalPoint { x: step, y: step },
            tile_margin: match settings.windows.tile_margins {
                Enabled::On => TILE_MARGIN,
                Enabled::Off => 0,
            },
            spread_gap: SPREAD_GAP,
            spread_floor: SPREAD_FLOOR,
            grip: LogicalSize {
                w: step.unsigned_abs() * GRIP_TITLEBARS_WIDE,
                h: step.unsigned_abs(),
            },
        }
    }
}
