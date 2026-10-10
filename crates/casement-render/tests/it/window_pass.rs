//! `pass::render_window` on pixman: a window drawn alone is the size of its content, transparent
//! where nothing draws, and does not depend on anything else on the screen.
#![cfg(feature = "testing")]

use crate::support;

use casement_core::geometry::{PhysicalRect, PhysicalSize};
use casement_render::pass::render_window;
use casement_render::{CasementElement, Rgba8, solid_rect};
use ds_core::geometry::scale::Scale;
use smithay::backend::renderer::element::Id;
use smithay::backend::renderer::pixman::PixmanRenderer;
use smithay::backend::renderer::utils::CommitCounter;
use support::{Offscreen1, pixman};

const SIZE: PhysicalSize = PhysicalSize { w: 8, h: 6 };

fn solid(x: i32, y: i32, w: u32, h: u32) -> CasementElement<PixmanRenderer> {
    let colour = Rgba8 {
        r: 200,
        g: 40,
        b: 90,
        a: 255,
    };
    CasementElement::Solid(solid_rect(
        Id::new(),
        PhysicalRect { x, y, w, h },
        colour,
        CommitCounter::default(),
    ))
}

fn pixel(bytes: &[u8], x: usize, y: usize) -> [u8; 4] {
    let at = (y * 8 + x) * 4;
    [bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]]
}

#[test]
fn a_window_is_drawn_alone_on_a_transparent_image() {
    let mut renderer = pixman();
    let mut target = renderer.target((8, 6));
    let bytes = render_window(
        &mut renderer,
        &mut target,
        &[solid(2, 1, 3, 2)],
        SIZE,
        Scale(120),
    )
    .expect("a window render");
    assert_eq!(bytes.len(), 8 * 6 * 4);
    assert_eq!(pixel(&bytes, 3, 2), [200, 40, 90, 255], "inside the window");
    assert_eq!(
        pixel(&bytes, 0, 0),
        [0, 0, 0, 0],
        "outside it is transparent"
    );
    assert_eq!(pixel(&bytes, 7, 5), [0, 0, 0, 0]);
}

#[test]
fn a_second_render_into_the_same_target_draws_everything_again() {
    let mut renderer = pixman();
    let mut target = renderer.target((8, 6));
    let first = render_window(
        &mut renderer,
        &mut target,
        &[solid(0, 0, 8, 6)],
        SIZE,
        Scale(120),
    )
    .expect("the first");
    let second = render_window(&mut renderer, &mut target, &[], SIZE, Scale(120)).expect("again");
    assert_eq!(pixel(&first, 4, 3), [200, 40, 90, 255]);
    assert_eq!(pixel(&second, 4, 3), [0, 0, 0, 0], "no stale pixels");
}
