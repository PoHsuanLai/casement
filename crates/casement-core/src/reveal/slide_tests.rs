//! Where the surface is drawn while it comes and goes.

use super::{Look, RevealEdge, slid};
use crate::geometry::{LogicalPoint, LogicalRect, LogicalSize};
use ds_core::vocab::Fraction;

fn rect(x: i32, y: i32, w: u32, h: u32) -> LogicalRect {
    LogicalRect::new(LogicalPoint { x, y }, LogicalSize { w, h })
}

#[test]
fn a_slide_moves_the_surface_toward_its_edge_until_it_is_off_the_output() {
    let output = rect(100, 50, 1000, 600);
    let bar = rect(100, 50, 1000, 32);
    let dock = rect(300, 560, 600, 80);
    // name, edge, resting place, thousandths in, the y it is drawn at
    let cases = [
        ("bar in", RevealEdge::Top, bar, 1000, 50),
        ("bar half", RevealEdge::Top, bar, 500, 34),
        ("bar out", RevealEdge::Top, bar, 0, 18),
        ("dock in", RevealEdge::Bottom, dock, 1000, 560),
        ("dock half", RevealEdge::Bottom, dock, 500, 605),
        ("dock out", RevealEdge::Bottom, dock, 0, 650),
    ];
    for (name, edge, area, visible, y) in cases {
        let drawn = slid(edge, Look::Slide, output, area, Fraction(visible));
        assert_eq!(drawn.area.origin.y, y, "{name}");
        assert_eq!(drawn.area.origin.x, area.origin.x, "{name}");
        assert_eq!(drawn.area.size, area.size, "{name}");
        assert_eq!(drawn.opacity, Fraction(1000), "{name}");
    }
}

#[test]
fn a_cross_fade_stays_put_and_fades() {
    let output = rect(0, 0, 1000, 600);
    let bar = rect(0, 0, 1000, 32);
    let drawn = slid(RevealEdge::Top, Look::CrossFade, output, bar, Fraction(300));
    assert_eq!((drawn.area, drawn.opacity), (bar, Fraction(300)));
}
