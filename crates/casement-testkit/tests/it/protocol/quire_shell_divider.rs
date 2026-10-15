//! The Split View divider dragged with the pointer: the compositor routes the 8 px strip between
//! the halves (the shell draws no handle), a press on it and each motion until the release set
//! the leading window's share, and nothing past the release moves it.

use super::quire_shell::shell_client;
use crate::protocol::xdg_shell::{map_toplevel, pointer, settle, windows};
use casement_core::geometry::LogicalPoint;
use casement_core::input::{ButtonCode, Edge};
use casement_core::policy::ConnectionRole;
use casement_core::space::SpaceContent;
use casement_layout::{DividerTarget, LayoutKnobs, RustScene, SceneLayout, spec_of};
use casement_testclient::quire::shell::WindowEntry;
use casement_testclient::surface::toplevel::WmBase;
use casement_testkit::{Fixture, FixtureSpec};
use casement_wayland::foreign_toplevel_list::identifier_of;
use ds_core::vocab::Fraction;

const LEFT_BUTTON: ButtonCode = ButtonCode(0x110);

/// The divider layout makes for the Split View now.
fn strip(fixture: &Fixture) -> DividerTarget {
    let model = fixture.model();
    RustScene
        .arrange(&spec_of(model), &LayoutKnobs::of(model.settings()))
        .dividers
        .into_iter()
        .next()
        .expect("a Split View divider")
}

/// The share the model holds for the Split View.
fn share(fixture: &Fixture) -> Fraction {
    fixture
        .model()
        .spaces()
        .iter()
        .find_map(|space| match space.content {
            SpaceContent::SplitView(pair) => Some(pair.divider),
            _ => None,
        })
        .expect("a Split View Space")
}

#[test]
fn dragging_the_strip_sets_the_share_and_the_release_ends_the_drag() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let app = fixture.connect(ConnectionRole::Public);
    let base = WmBase::bind(fixture.client(app), 7);
    let _first = map_toplevel(&mut fixture, app, &base, (400, 300));
    let _second = map_toplevel(&mut fixture, app, &base, (300, 200));
    let (id, shell) = shell_client(&mut fixture);
    let (first, second) = match windows(&fixture)[..] {
        [first, second] => (first, second),
        ref other => panic!("two windows, got {other:?}"),
    };
    shell.window_menu_pick(&identifier_of(first), WindowEntry::TileLeftOfScreen);
    fixture.pump();
    shell.split_pick(&identifier_of(second));
    fixture.pump();
    settle(&mut fixture);
    assert_eq!(share(&fixture), Fraction(500), "a new Split View is even");

    let device = pointer(&mut fixture);
    let divider = strip(&fixture);
    let grip = divider.strip.centre();
    fixture.input().pointer_to(device, grip);
    fixture.input().button(device, LEFT_BUTTON, Edge::Press);
    assert_eq!(
        share(&fixture),
        Fraction(500),
        "a press alone moves nothing"
    );

    // name, pointer column
    let drags = [
        ("right", grip.x + 300),
        ("back past the middle", grip.x - 200),
    ];
    for (name, x) in drags {
        fixture
            .input()
            .pointer_to(device, LogicalPoint { x, y: grip.y });
        assert_eq!(share(&fixture), divider.share_at(x), "{name}");
    }
    let held = share(&fixture);
    assert_ne!(held, Fraction(500));

    fixture.input().button(device, LEFT_BUTTON, Edge::Release);
    fixture.input().pointer_to(
        device,
        LogicalPoint {
            x: grip.x + 500,
            y: grip.y,
        },
    );
    assert_eq!(
        share(&fixture),
        held,
        "after the release the pointer moves nothing"
    );
    assert!(fixture.client(id).protocol_error().is_none());
}

#[test]
fn a_press_beside_the_strip_does_not_move_the_divider() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let app = fixture.connect(ConnectionRole::Public);
    let base = WmBase::bind(fixture.client(app), 7);
    let _first = map_toplevel(&mut fixture, app, &base, (400, 300));
    let _second = map_toplevel(&mut fixture, app, &base, (300, 200));
    let (_, shell) = shell_client(&mut fixture);
    let (first, second) = match windows(&fixture)[..] {
        [first, second] => (first, second),
        ref other => panic!("two windows, got {other:?}"),
    };
    shell.window_menu_pick(&identifier_of(first), WindowEntry::TileLeftOfScreen);
    fixture.pump();
    shell.split_pick(&identifier_of(second));
    fixture.pump();
    settle(&mut fixture);

    let device = pointer(&mut fixture);
    let divider = strip(&fixture);
    let beside = LogicalPoint {
        x: divider.strip.origin.x - 20,
        y: divider.strip.centre().y,
    };
    fixture.input().pointer_to(device, beside);
    fixture.input().button(device, LEFT_BUTTON, Edge::Press);
    fixture.input().pointer_to(device, divider.strip.centre());
    fixture.input().button(device, LEFT_BUTTON, Edge::Release);
    assert_eq!(share(&fixture), Fraction(500));
}
