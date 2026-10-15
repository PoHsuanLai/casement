//! The drag-to-edge tile machine: which zone the pointer arms, and the steps of a drag.

use casement_core::WindowId;
use casement_core::geometry::{LogicalPoint, LogicalRect, LogicalSize};
use casement_core::settings::Enabled;
use casement_core::window::{TileDrag, TileDrop, TileKnobs, TileScene, TileTarget, target_at};
use casement_protocols::vocab::{MacMods, TileZone as Z};

fn rect(x: i32, y: i32, w: u32, h: u32) -> LogicalRect {
    LogicalRect::new(LogicalPoint { x, y }, LogicalSize { w, h })
}

fn at(x: i32, y: i32) -> LogicalPoint {
    LogicalPoint { x, y }
}

fn zone(zone: Z) -> TileTarget {
    TileTarget::Zone(zone)
}

const FILL: TileTarget = TileTarget::Fill;

const ALL_ON: TileKnobs = TileKnobs {
    edges: Enabled::On,
    top_fills: Enabled::On,
    option_drag: Enabled::On,
};

const HOME: fn() -> LogicalRect = || rect(0, 0, 1920, 1080);

/// name, where, modifiers, the other outputs, the knobs, the zone
type Case = (
    &'static str,
    LogicalPoint,
    MacMods,
    Vec<LogicalRect>,
    TileKnobs,
    Option<TileTarget>,
);

fn check(cases: Vec<Case>) {
    for (name, point, mods, others, knobs, want) in cases {
        let scene = TileScene {
            home: HOME(),
            others: &others,
        };
        assert_eq!(target_at(point, mods, &scene, &knobs), want, "{name}");
    }
}

#[test]
fn edges_and_corners_arm_halves_fill_and_quarters() {
    let none = MacMods::NONE;
    let case = |name, point, want| -> Case { (name, point, none, vec![], ALL_ON, want) };
    check(vec![
        case("left edge", at(0, 540), Some(zone(Z::Left))),
        case("right edge", at(1919, 540), Some(zone(Z::Right))),
        case("inside the band", at(2, 540), Some(zone(Z::Left))),
        case("just outside the band", at(3, 540), None),
        case("top edge fills", at(960, 0), Some(FILL)),
        case(
            "the menu bar below the edge is not the edge",
            at(960, 10),
            None,
        ),
        case("top left corner", at(0, 0), Some(zone(Z::TopLeft))),
        case("top right corner", at(1919, 0), Some(zone(Z::TopRight))),
        case("bottom left corner", at(0, 1079), Some(zone(Z::BottomLeft))),
        case(
            "bottom right corner",
            at(1919, 1079),
            Some(zone(Z::BottomRight)),
        ),
        case("left edge near the top", at(0, 95), Some(zone(Z::TopLeft))),
        case(
            "left edge past the corner's reach",
            at(0, 96),
            Some(zone(Z::Left)),
        ),
        case(
            "right edge near the bottom",
            at(1919, 1000),
            Some(zone(Z::BottomRight)),
        ),
        case(
            "top edge near the right",
            at(1900, 0),
            Some(zone(Z::TopRight)),
        ),
        case("top edge past the corner's reach", at(1800, 0), Some(FILL)),
        case("the bottom edge alone tiles nothing", at(960, 1079), None),
        case(
            "the bottom edge near a corner",
            at(50, 1079),
            Some(zone(Z::BottomLeft)),
        ),
        case("the middle of the screen", at(960, 540), None),
        case("off the output", at(-1, 540), None),
    ]);
}

#[test]
fn the_knobs_change_what_the_edges_do() {
    let none = MacMods::NONE;
    let option = MacMods::OPTION;
    let fills_off = TileKnobs {
        top_fills: Enabled::Off,
        ..ALL_ON
    };
    let edges_off = TileKnobs {
        edges: Enabled::Off,
        ..ALL_ON
    };
    let option_off = TileKnobs {
        option_drag: Enabled::Off,
        ..ALL_ON
    };
    check(vec![
        (
            "top half when fill is off",
            at(960, 0),
            none,
            vec![],
            fills_off,
            Some(zone(Z::Top)),
        ),
        (
            "edges off: the left edge",
            at(0, 540),
            none,
            vec![],
            edges_off,
            None,
        ),
        (
            "edges off: option still arms",
            at(100, 540),
            option,
            vec![],
            edges_off,
            Some(zone(Z::Left)),
        ),
        (
            "option off: it arms nothing inside",
            at(100, 540),
            option,
            vec![],
            option_off,
            None,
        ),
        (
            "option off: the edge still tiles",
            at(0, 540),
            option,
            vec![],
            option_off,
            Some(zone(Z::Left)),
        ),
    ]);
}

#[test]
fn option_arms_the_thirds_of_the_output() {
    let option = MacMods::OPTION;
    let case = |name, point, want| -> Case { (name, point, option, vec![], ALL_ON, want) };
    check(vec![
        case("left third", at(100, 540), Some(zone(Z::Left))),
        case("right third", at(1800, 540), Some(zone(Z::Right))),
        case("top middle fills", at(960, 100), Some(FILL)),
        case("bottom middle", at(960, 1000), Some(zone(Z::Bottom))),
        case("top left", at(100, 100), Some(zone(Z::TopLeft))),
        case("top right", at(1800, 100), Some(zone(Z::TopRight))),
        case("bottom left", at(100, 1000), Some(zone(Z::BottomLeft))),
        case("bottom right", at(1800, 1000), Some(zone(Z::BottomRight))),
        case("the centre arms nothing", at(960, 540), None),
        case(
            "option and shift",
            at(100, 540),
            MacMods::OPTION.with(MacMods::SHIFT),
            Some(zone(Z::Left)),
        ),
    ]);
}

#[test]
fn an_edge_shared_with_another_output_is_not_an_edge() {
    let none = MacMods::NONE;
    let right_of = vec![rect(1920, 0, 1920, 1080)];
    let above = vec![rect(0, -1080, 1920, 1080)];
    let case = |name, point, others: Vec<LogicalRect>, want| -> Case {
        (name, point, none, others, ALL_ON, want)
    };
    check(vec![
        case("the shared edge", at(1919, 540), right_of.clone(), None),
        case(
            "the far edge still tiles",
            at(0, 540),
            right_of.clone(),
            Some(zone(Z::Left)),
        ),
        case(
            "the top edge beside it fills",
            at(1919, 0),
            right_of.clone(),
            Some(FILL),
        ),
        case(
            "a corner on the open side",
            at(0, 0),
            right_of,
            Some(zone(Z::TopLeft)),
        ),
        case(
            "the top edge under another output",
            at(960, 0),
            above.clone(),
            None,
        ),
        case(
            "the bottom edge is open",
            at(0, 1079),
            above,
            Some(zone(Z::BottomLeft)),
        ),
    ]);
}

#[test]
fn a_drag_previews_while_over_a_zone_and_tiles_on_release() {
    let scene = TileScene {
        home: HOME(),
        others: &[],
    };
    let none = MacMods::NONE;
    let window = WindowId(7);
    let moving = TileDrag::begin(window);
    assert_eq!(moving.target(), None);
    let over = moving.moved(at(0, 540), none, &scene, &ALL_ON);
    assert_eq!(over.target(), Some(zone(Z::Left)));
    let away = over.moved(at(500, 540), none, &scene, &ALL_ON);
    assert_eq!(away.target(), None, "leaving the zone drops the preview");
    let again = away.moved(at(1919, 540), none, &scene, &ALL_ON);
    assert_eq!(
        again.released(),
        (
            TileDrag::Idle,
            Some(TileDrop {
                window,
                target: zone(Z::Right)
            })
        )
    );
    assert_eq!(
        away.released(),
        (TileDrag::Idle, None),
        "a release off every zone leaves the window floating"
    );
    assert_eq!(
        TileDrag::Idle.moved(at(0, 540), none, &scene, &ALL_ON),
        TileDrag::Idle,
        "no move, no preview"
    );
}
