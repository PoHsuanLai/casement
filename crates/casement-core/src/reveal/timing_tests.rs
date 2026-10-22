//! The numbers are the timing table's and the dock's own.

use super::{
    BAR_DWELL, BAR_SLIDE, DOCK_DWELL, DOCK_SLIDE, Look, REDUCED_FADE, Reduced, RevealEdge,
    RevealTiming,
};
use ds_style::appearance::motion::MotionLevel;
use ds_style::tokens::timing::DurationToken;
use std::time::Duration;

fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}

#[test]
fn the_bar_slides_for_the_move_duration_and_fades_for_the_quick_one() {
    let standard = MotionLevel::Standard;
    assert_eq!(BAR_SLIDE, DurationToken::Move.duration(standard));
    assert_eq!(
        REDUCED_FADE,
        DurationToken::Move.duration(MotionLevel::Reduced)
    );
    assert_eq!(REDUCED_FADE, DurationToken::Quick.duration(standard));
}

#[test]
fn each_edge_has_its_numbers_and_reduced_only_changes_the_travel() {
    // name, edge, reduced, dwell, travel, look
    let cases = [
        (
            "bar",
            RevealEdge::Top,
            Reduced::No,
            (ms(450), ms(200)),
            Look::Slide,
        ),
        (
            "dock",
            RevealEdge::Bottom,
            Reduced::No,
            (ms(200), ms(500)),
            Look::Slide,
        ),
        (
            "bar reduced",
            RevealEdge::Top,
            Reduced::Yes,
            (ms(450), ms(120)),
            Look::CrossFade,
        ),
        (
            "dock reduced",
            RevealEdge::Bottom,
            Reduced::Yes,
            (ms(200), ms(120)),
            Look::CrossFade,
        ),
    ];
    for (name, edge, reduced, (dwell, travel), look) in cases {
        let timing = RevealTiming::of(edge, reduced);
        let got = (timing.dwell, timing.travel, timing.look);
        assert_eq!(got, (dwell, travel, look), "{name}");
    }
    assert_eq!(
        (BAR_DWELL, DOCK_DWELL, DOCK_SLIDE),
        (ms(450), ms(200), ms(500))
    );
}
