//! Renderer-only tests (P0-R-scene): elements that need no client surface, on pixman, with the
//! damage oracle (incremental damage rendering equals a full re-render, byte for byte).
#![cfg(feature = "testing")]

use casement_core::geometry::PhysicalRect;
use casement_render::{LockCoverElement, Rgba8};
use proptest::prelude::*;
use smithay::backend::renderer::element::Id;

/// The cover's colour (opaque black).
const COVER: Rgba8 = Rgba8 {
    r: 0,
    g: 0,
    b: 0,
    a: 255,
};

#[test]
#[ignore = "P0-R-scene"]
fn a_lock_cover_is_opaque_over_its_whole_area() {
    let area = PhysicalRect {
        x: 0,
        y: 0,
        w: 640,
        h: 480,
    };
    let cover = LockCoverElement::new(Id::new(), area, COVER);
    let _ = cover;
    todo!(
        "P0-R-scene: render the cover on pixman; every pixel is opaque black, the opaque region is the area"
    )
}

/// Renders `moves` (one solid rectangle per frame) incrementally with buffer age 1 and the
/// last frame once from scratch; whether the two are byte-identical.
fn incremental_matches_full(moves: &[(i32, i32, u32, u32)]) -> bool {
    let _ = moves;
    todo!(
        "P0-R-scene: render with OutputDamageTracker (age 1) and from scratch (age 0); compare bytes"
    )
}

proptest! {
    /// Damage oracle (11 §3.4): any sequence of solid rectangles moving between frames,
    /// rendered incrementally with buffer age, equals rendering the last frame from scratch.
    #[test]
    #[ignore = "P0-R-scene"]
    fn incremental_damage_equals_a_full_render(moves in proptest::collection::vec((0_i32..600, 0_i32..440, 1_u32..40, 1_u32..40), 1..12)) {
        prop_assert!(incremental_matches_full(&moves));
    }
}
