//! The pure parts of the winit backend: a window's size and the host's scale factor are the
//! output's mode and scale in 120ths; the window is drawn flipped; focus loss releases what is
//! held.

use casement_backend::{HOST_TRANSFORM, HeldInput, VirtualOutputSpec, window_spec};
use casement_core::geometry::PhysicalSize;
use casement_core::input::{ButtonCode, Edge, InputKind, KeyCode};
use smithay::utils::Transform;

fn spec_at(size: (u32, u32), factor: f64) -> (u32, u32, u32) {
    let base = VirtualOutputSpec::parse("WINIT-1:1440x900@60000:120:0,0").expect("spec");
    let spec = window_spec(
        &base,
        PhysicalSize {
            w: size.0,
            h: size.1,
        },
        factor,
    );
    (spec.mode.size.w, spec.mode.size.h, spec.scale.0)
}

#[test]
fn a_window_is_a_mode_and_a_scale_in_120ths() {
    let table = [
        ((1440, 900), 1.0, (1440, 900, 120)),
        ((2880, 1800), 2.0, (2880, 1800, 240)),
        ((1920, 1200), 1.5, (1920, 1200, 180)),
        ((1000, 700), 1.25, (1000, 700, 150)),
        ((1000, 700), 1.3333, (1000, 700, 160)),
        ((1000, 700), 0.0, (1000, 700, 120)),
        ((1000, 700), -2.0, (1000, 700, 120)),
        ((1000, 700), f64::NAN, (1000, 700, 120)),
        ((1000, 700), f64::INFINITY, (1000, 700, 120)),
        ((1000, 700), 0.001, (1000, 700, 1)),
    ];
    for (size, factor, expected) in table {
        assert_eq!(spec_at(size, factor), expected, "{size:?} at {factor}");
    }
}

#[test]
fn a_window_keeps_the_rest_of_its_spec() {
    let base = VirtualOutputSpec::parse("WINIT-1:1440x900@60000:120:0,0").expect("spec");
    let spec = window_spec(&base, PhysicalSize { w: 10, h: 10 }, 2.0);
    assert_eq!(spec.name, base.name);
    assert_eq!(spec.mode.refresh, base.mode.refresh);
    assert_eq!(spec.position, base.position);
}

#[test]
fn the_window_is_drawn_flipped() {
    assert_eq!(HOST_TRANSFORM, Transform::Flipped180);
}

#[test]
fn losing_focus_releases_what_is_held() {
    let mut held = HeldInput::new();
    let key = |code, edge| InputKind::Key {
        code: KeyCode(code),
        edge,
    };
    held.note(&key(30, Edge::Press));
    held.note(&key(31, Edge::Press));
    held.note(&key(30, Edge::Release));
    held.note(&InputKind::Button {
        button: ButtonCode(0x110),
        edge: Edge::Press,
    });
    let released = held.release_all();
    assert_eq!(
        released,
        vec![
            key(31, Edge::Release),
            InputKind::Button {
                button: ButtonCode(0x110),
                edge: Edge::Release,
            },
        ]
    );
    assert!(held.release_all().is_empty());
}
