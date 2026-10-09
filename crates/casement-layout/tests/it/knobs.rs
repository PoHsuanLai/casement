//! The knobs settings give.

use casement_core::settings::{CasementSettings, Enabled};
use casement_layout::LayoutKnobs;

#[test]
fn tile_margins_follow_their_setting() {
    let on = CasementSettings::defaults();
    let mut off = CasementSettings::defaults();
    off.windows.tile_margins = Enabled::Off;
    assert_eq!(LayoutKnobs::of(&off).tile_margin, 0);
    assert_eq!(
        LayoutKnobs::of(&on).tile_margin,
        8,
        "margins on means the default margin"
    );
}

#[test]
fn the_cascade_steps_one_titlebar_and_the_spread_has_the_macs_gap_and_floor() {
    use casement_core::geometry::LogicalPoint;
    use ds_core::vocab::Fraction;
    let knobs = LayoutKnobs::of(&CasementSettings::defaults());
    assert_eq!(knobs.cascade_step, LogicalPoint { x: 28, y: 28 });
    assert_eq!((knobs.spread_gap, knobs.spread_floor), (20, Fraction(150)));
}
