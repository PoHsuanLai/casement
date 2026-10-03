//! The knobs settings give.

use casement_core::settings::{CasementSettings, Enabled};
use casement_layout::LayoutKnobs;

#[test]
#[ignore = "P0-F1-layout"]
fn tile_margins_follow_their_setting() {
    let on = CasementSettings::defaults();
    let mut off = CasementSettings::defaults();
    off.windows.tile_margins = Enabled::Off;
    assert_eq!(LayoutKnobs::of(&off).tile_margin, 0);
    assert!(
        LayoutKnobs::of(&on).tile_margin > 0,
        "margins on means a margin"
    );
    assert_eq!(LayoutKnobs::of(&on), LayoutKnobs::of(&on.clone()));
}
