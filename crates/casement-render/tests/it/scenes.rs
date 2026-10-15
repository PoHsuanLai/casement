//! Renderer-only golden scenes (P0-R-scene). Everything casement draws without a client
//! surface: the named cursors at native scale, a cursor across two outputs, the lock cover,
//! blur v0 (its pixman stand-in in tier P, its GLES blur in tier L on llvmpipe), and the clear
//! output. Aligned scenes are also byte-identical between pixman and llvmpipe (tier X, A18).
//! The client-surface scenes (windows, popups, layers, lock surfaces) are the fixture's, in
//! `crates/casement-testkit/tests/it/scene/mod.rs`.
//!
//! Goldens live in `tests/golden/<name>.png`, blessed with `CASEMENT_BLESS=1`.
#![cfg(feature = "testing")]

use crate::support;

use casement_core::WindowId;
use casement_core::geometry::{
    LogicalPoint, LogicalRect, LogicalSize, PhysicalRect, PhysicalSize, SubPx,
};
use casement_motion::{OutputFrame, StripOffset};
use casement_render::golden::GoldenTier;
use casement_render::{
    BlurV0Element, CasementElement, ChromeLayer, CursorDraw, CursorShape, LayerDraw, OutputView,
    Rgba8, TextureBook, TilePreview, cursor_elements, output_elements, solid_rect,
    tile_preview_elements,
};
use ds_core::geometry::scale::Scale;
use smithay::backend::renderer::element::Id;
use smithay::backend::renderer::utils::CommitCounter;
use support::{Offscreen1, beside, check, gles, image_of, output_key, pixman, render_once};

const GREY: Rgba8 = Rgba8 {
    r: 128,
    g: 128,
    b: 128,
    a: 255,
};

/// A red that must never show (what a locked session would leak).
const LEAK: Rgba8 = Rgba8 {
    r: 255,
    g: 0,
    b: 0,
    a: 255,
};

const STRIPES: [Rgba8; 4] = [
    Rgba8 {
        r: 230,
        g: 60,
        b: 50,
        a: 255,
    },
    Rgba8 {
        r: 250,
        g: 250,
        b: 250,
        a: 255,
    },
    Rgba8 {
        r: 40,
        g: 110,
        b: 220,
        a: 255,
    },
    Rgba8 {
        r: 20,
        g: 20,
        b: 20,
        a: 255,
    },
];

/// A translucent window colour (premultiplied, alpha 0.6).
const PANE: Rgba8 = Rgba8 {
    r: 153,
    g: 153,
    b: 102,
    a: 153,
};

const SHAPES: [CursorShape; 11] = [
    CursorShape::Default,
    CursorShape::Text,
    CursorShape::Pointer,
    CursorShape::Grab,
    CursorShape::Grabbing,
    CursorShape::ResizeNs,
    CursorShape::ResizeEw,
    CursorShape::ResizeNesw,
    CursorShape::ResizeNwse,
    CursorShape::Wait,
    CursorShape::NotAllowed,
];

fn rect(x: i32, y: i32, w: u32, h: u32) -> PhysicalRect {
    PhysicalRect { x, y, w, h }
}

/// An output at `(x, y)` of `w` by `h` logical pixels at `scale` (120ths).
fn view(x: i32, y: i32, w: u32, h: u32, scale: u32) -> OutputView {
    let mode = casement_core::geometry::buffer_size(LogicalSize { w, h }, Scale(scale));
    OutputView {
        area: LogicalRect::new(LogicalPoint { x, y }, LogicalSize { w, h }),
        scale: Scale(scale),
        mode,
    }
}

fn size_of(view: &OutputView) -> (i32, i32) {
    let side = |n: u32| i32::try_from(n).expect("small output");
    (side(view.mode.w), side(view.mode.h))
}

fn page<R: Offscreen1>(view: &OutputView, colour: Rgba8) -> CasementElement<R> {
    let PhysicalSize { w, h } = view.mode;
    CasementElement::Solid(solid_rect(
        Id::new(),
        rect(0, 0, w, h),
        colour,
        CommitCounter::default(),
    ))
}

fn chrome(cursor: CursorDraw, at: (i32, i32)) -> ChromeLayer {
    ChromeLayer {
        cursor,
        cursor_at: LogicalPoint { x: at.0, y: at.1 },
        covers: Vec::new(),
    }
}

fn empty_frame(name: &str) -> OutputFrame {
    OutputFrame {
        output: output_key(name),
        strip: StripOffset(SubPx(0)),
        windows: Vec::new(),
    }
}

/// Every named cursor in a 6 by 2 grid of 40-pixel cells on grey, at `scale`.
fn cursor_sheet<R: Offscreen1>(renderer: &mut R, scale: u32) -> image::RgbaImage {
    let view = view(0, 0, 240, 80, scale);
    let book = TextureBook::empty();
    let mut elements: Vec<CasementElement<R>> = SHAPES
        .iter()
        .zip(0_i32..)
        .flat_map(|(shape, n)| {
            let at = ((n % 6) * 40 + 14, (n / 6) * 40 + 14);
            cursor_elements(
                &chrome(CursorDraw::Named(*shape), at),
                &view,
                &book,
                renderer,
            )
        })
        .collect();
    elements.push(page(&view, GREY));
    let size = size_of(&view);
    image_of(
        size,
        render_once(renderer, size, view.scale.as_f64(), &elements),
    )
}

/// One output of a pair with the cursor at the seam: grey page, the frame's elements on top.
fn output_shot<R: Offscreen1>(
    renderer: &mut R,
    view: &OutputView,
    name: &str,
    chrome: &ChromeLayer,
) -> image::RgbaImage {
    let mut elements = output_elements(
        &empty_frame(name),
        view,
        &LayerDraw::default(),
        &TextureBook::empty(),
        chrome,
        renderer,
    );
    elements.push(page(view, GREY));
    let size = size_of(view);
    image_of(
        size,
        render_once(renderer, size, view.scale.as_f64(), &elements),
    )
}

/// The arrow at the seam of two outputs side by side; the right one at `right_scale`.
fn across_outputs<R: Offscreen1>(renderer: &mut R, right_scale: u32) -> image::RgbaImage {
    let left = view(0, 0, 160, 100, 120);
    let right = view(160, 0, 160, 100, right_scale);
    let at = chrome(CursorDraw::Named(CursorShape::Default), (156, 40));
    beside(&[
        output_shot(renderer, &left, "HEADLESS-1", &at),
        output_shot(renderer, &right, "HEADLESS-2", &at),
    ])
}

/// The blur scene: coloured stripes, a blur over a ring-shaped region (two bands and two
/// posts), a translucent pane above it.
fn blur_scene<R: Offscreen1>(renderer: &mut R) -> image::RgbaImage {
    let view = view(0, 0, 320, 200, 120);
    let mut elements: Vec<CasementElement<R>> = vec![
        CasementElement::Solid(solid_rect(
            Id::new(),
            rect(140, 20, 160, 60),
            PANE,
            CommitCounter::default(),
        )),
        CasementElement::Blur(BlurV0Element::new(
            Id::new(),
            vec![
                rect(40, 40, 240, 30),
                rect(40, 130, 240, 30),
                rect(40, 70, 40, 60),
                rect(240, 70, 40, 60),
            ],
            Rgba8 {
                r: 208,
                g: 208,
                b: 208,
                a: 217,
            },
        )),
    ];
    elements.extend((0_i32..16).map(|n| {
        let colour = STRIPES[usize::try_from(n % 4).expect("small")];
        CasementElement::Solid(solid_rect(
            Id::new(),
            rect(n * 20, 0, 20, 200),
            colour,
            CommitCounter::default(),
        ))
    }));
    let size = size_of(&view);
    image_of(
        size,
        render_once(renderer, size, view.scale.as_f64(), &elements),
    )
}

#[test]
fn the_ten_renderer_scenes_match_their_pixman_goldens() {
    let mut renderer = pixman();
    for scale in [120, 150, 180, 210, 240] {
        let percent = match scale {
            120 => "1x",
            150 => "1.25x",
            180 => "1.5x",
            210 => "1.75x",
            _ => "2x",
        };
        check(
            &format!("cursor-sheet-{percent}"),
            GoldenTier::Exact,
            &cursor_sheet(&mut renderer, scale),
        );
    }
    check(
        "cursor-across-outputs-1x",
        GoldenTier::Exact,
        &across_outputs(&mut renderer, 120),
    );
    check(
        "cursor-across-outputs-1x-2x",
        GoldenTier::Exact,
        &across_outputs(&mut renderer, 240),
    );

    // A locked output whose locker died: the cursor, then the cover, then a session that must
    // not show.
    let locked = view(0, 0, 200, 120, 180);
    let mut cover_chrome = chrome(CursorDraw::Named(CursorShape::Text), (100, 60));
    cover_chrome.covers = vec![output_key("HEADLESS-1")];
    let mut elements = output_elements(
        &empty_frame("HEADLESS-1"),
        &locked,
        &LayerDraw::default(),
        &TextureBook::empty(),
        &cover_chrome,
        &mut renderer,
    );
    elements.push(page(&locked, LEAK));
    let size = size_of(&locked);
    let shot = image_of(size, render_once(&mut renderer, size, 1.5, &elements));
    assert!(
        shot.pixels().all(|px| px.0 != [255, 0, 0, 255]),
        "nothing of the session shows"
    );
    check("lock-cover-dead-locker-1.5x", GoldenTier::Exact, &shot);

    check(
        "blur-v0-stand-in-1x",
        GoldenTier::Exact,
        &blur_scene(&mut renderer),
    );

    let blank = view(0, 0, 64, 40, 120);
    let elements = output_elements(
        &empty_frame("HEADLESS-1"),
        &blank,
        &LayerDraw::default(),
        &TextureBook::empty(),
        &chrome(CursorDraw::Hidden, (0, 0)),
        &mut renderer,
    );
    assert!(elements.is_empty());
    let size = size_of(&blank);
    check(
        "empty-output-1x",
        GoldenTier::Exact,
        &image_of(size, render_once(&mut renderer, size, 1.0, &elements)),
    );
}

#[test]
fn blur_v0_matches_its_llvmpipe_golden() {
    let shot = blur_scene(&mut gles());
    // Inside a band, across a stripe edge, the blur mixes the two colours; outside the region
    // the stripes are untouched.
    let mixed = shot.get_pixel(100, 55).0;
    assert!(
        mixed != STRIPES[0].rgba() && mixed != STRIPES[1].rgba(),
        "blurred at the edge: {mixed:?}"
    );
    assert_eq!(
        shot.get_pixel(10, 100).0,
        STRIPES[0].rgba(),
        "outside the region"
    );
    assert_eq!(
        shot.get_pixel(150, 100).0,
        STRIPES[3].rgba(),
        "inside the ring's hole"
    );
    check(
        "blur-v0-1x",
        GoldenTier::Soft {
            channel: 2,
            parts_per_million: 1000,
        },
        &shot,
    );
}

#[test]
fn aligned_scenes_are_byte_identical_on_pixman_and_llvmpipe() {
    let (mut cpu, mut gpu) = (pixman(), gles());
    for scale in [120, 180, 240] {
        assert!(
            cursor_sheet(&mut cpu, scale) == cursor_sheet(&mut gpu, scale),
            "cursor sheet at {scale}"
        );
    }
    assert!(
        across_outputs(&mut cpu, 240) == across_outputs(&mut gpu, 240),
        "two outputs, two scales"
    );
}

/// The colour as the bytes a frame reads back.
trait Bytes {
    fn rgba(self) -> [u8; 4];
}

impl Bytes for Rgba8 {
    fn rgba(self) -> [u8; 4] {
        [self.r, self.g, self.b, self.a]
    }
}

/// The damage oracle for a framebuffer effect: a solid moving under a still blur on GLES,
/// rendered incrementally (buffer age 1), ends byte-identical to the last frame from scratch;
/// the blur is captured again whenever what lies under it changes.
#[test]
fn a_blur_follows_what_moves_under_it() {
    let mut renderer = gles();
    let size = (320, 200);
    let (blur_id, moving, page) = (Id::new(), Id::new(), Id::new());
    let frame = |x: i32| -> Vec<CasementElement<smithay::backend::renderer::gles::GlesRenderer>> {
        vec![
            CasementElement::Blur(BlurV0Element::new(
                blur_id.clone(),
                vec![rect(60, 40, 200, 120)],
                Rgba8 {
                    r: 208,
                    g: 208,
                    b: 208,
                    a: 217,
                },
            )),
            CasementElement::Solid(solid_rect(
                moving.clone(),
                rect(x, 70, 40, 60),
                STRIPES[2],
                CommitCounter::default(),
            )),
            CasementElement::Solid(solid_rect(
                page.clone(),
                rect(0, 0, 320, 200),
                STRIPES[1],
                CommitCounter::default(),
            )),
        ]
    };
    let mut target = renderer.target(size);
    let mut tracker = support::tracker(size, 1.0);
    let mut last = Vec::new();
    for (n, x) in [0, 30, 90, 260, 260, 150].into_iter().enumerate() {
        last = renderer
            .draw(
                &mut target,
                &mut tracker,
                size,
                &frame(x),
                usize::from(n > 0),
            )
            .1;
    }
    let full = render_once(&mut renderer, size, 1.0, &frame(150));
    assert!(last == full, "incremental blur equals a full render");
}

/// The plate of a drag to the left edge: a translucent accent over grey, rounded corners.
fn tile_preview_scene<R: Offscreen1>(renderer: &mut R, scale: u32) -> image::RgbaImage {
    let view = view(0, 0, 320, 200, scale);
    let preview = TilePreview {
        window: WindowId(1),
        rect: LogicalRect::new(LogicalPoint { x: 8, y: 8 }, LogicalSize { w: 148, h: 184 }),
        fill: Rgba8 {
            r: 4,
            g: 46,
            b: 89,
            a: 89,
        },
        radius: 10,
    };
    let mut elements = tile_preview_elements(&preview, &view, renderer);
    elements.push(page(&view, GREY));
    let size = size_of(&view);
    image_of(
        size,
        render_once(renderer, size, view.scale.as_f64(), &elements),
    )
}

#[test]
fn the_tile_preview_matches_its_pixman_goldens() {
    let mut renderer = pixman();
    for (scale, name) in [(120, "1x"), (180, "1.5x"), (240, "2x")] {
        check(
            &format!("tile-preview-{name}"),
            GoldenTier::Exact,
            &tile_preview_scene(&mut renderer, scale),
        );
    }
}
