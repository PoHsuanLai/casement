//! Renderer-only tests (P0-R-scene): elements that need no client surface, on pixman, with the
//! damage oracle (incremental damage rendering equals a full re-render, byte for byte).
#![cfg(feature = "testing")]

use crate::support;

use casement_core::geometry::SubPx;
use casement_core::geometry::{LogicalPoint, LogicalRect, LogicalSize, PhysicalRect, PhysicalSize};
use casement_motion::{OutputFrame, StripOffset};
use casement_render::{
    BlurV0Element, CasementElement, ChromeLayer, CursorDraw, CursorShape, LayerDraw,
    LockCoverElement, OutputView, RenderSync, Rgba8, TextureBook, output_elements, solid_rect,
};
use ds_core::geometry::scale::Scale;
use proptest::prelude::*;
use smithay::backend::renderer::element::{Element, Id};
use smithay::backend::renderer::pixman::PixmanRenderer;
use smithay::backend::renderer::utils::CommitCounter;
use support::{Offscreen1, pixman, render_once, tracker};

/// The cover's colour (opaque black).
const COVER: Rgba8 = Rgba8 {
    r: 0,
    g: 0,
    b: 0,
    a: 255,
};

const WHITE: Rgba8 = Rgba8 {
    r: 255,
    g: 255,
    b: 255,
    a: 255,
};

/// A translucent blue, premultiplied: 0.5 alpha.
const GLASS: Rgba8 = Rgba8 {
    r: 0,
    g: 40,
    b: 128,
    a: 128,
};

/// The oracle's output: 640 by 480 at 1x.
const SIZE: (i32, i32) = (640, 480);

fn rect(x: i32, y: i32, w: u32, h: u32) -> PhysicalRect {
    PhysicalRect { x, y, w, h }
}

#[test]
fn a_lock_cover_is_opaque_over_its_whole_area() {
    let area = rect(0, 0, 640, 480);
    let cover = LockCoverElement::new(Id::new(), area, COVER);
    let opaque: Vec<_> = cover.opaque_regions(1.0.into()).into_iter().collect();
    assert_eq!(
        opaque,
        vec![smithay::utils::Rectangle::from_size((640, 480).into())]
    );
    // Over a white page and a translucent band, so a cover that drew nothing would show.
    let elements: Vec<CasementElement<PixmanRenderer>> = vec![
        CasementElement::Cover(cover),
        CasementElement::Solid(solid_rect(
            Id::new(),
            rect(100, 100, 200, 50),
            GLASS,
            CommitCounter::default(),
        )),
        CasementElement::Solid(solid_rect(Id::new(), area, WHITE, CommitCounter::default())),
    ];
    let bytes = render_once(&mut pixman(), SIZE, 1.0, &elements);
    assert!(
        bytes.chunks(4).all(|px| px == [0, 0, 0, 255]),
        "every pixel opaque black"
    );
}

/// The mixed oracle's scene.
#[derive(Debug, Clone, Copy)]
struct Step {
    /// The translucent solid.
    solid: (i32, i32, u32, u32),
    /// The blur stand-in's two rectangles.
    blur: (i32, i32, u32, u32),
    /// The named cursor's hotspot.
    cursor: (i32, i32),
    /// Whether the output is covered.
    covered: bool,
}

/// The scene the mixed oracle starts from.
const START: Step = Step {
    solid: (100, 100, 80, 60),
    blur: (200, 150, 120, 90),
    cursor: (320, 240),
    covered: false,
};

/// One change between two frames: most frames change one thing, so the rest is drawn from
/// the buffer's previous contents (what the oracle tests).
#[derive(Debug, Clone, Copy)]
enum Change {
    Solid((i32, i32, u32, u32)),
    Blur((i32, i32, u32, u32)),
    Cursor((i32, i32)),
    Cover(bool),
    Nothing,
}

impl Change {
    fn apply(self, step: Step) -> Step {
        match self {
            Change::Solid(solid) => Step { solid, ..step },
            Change::Blur(blur) => Step { blur, ..step },
            Change::Cursor(cursor) => Step { cursor, ..step },
            Change::Cover(covered) => Step { covered, ..step },
            Change::Nothing => step,
        }
    }
}

fn change() -> impl Strategy<Value = Change> {
    let rect = (0_i32..600, 0_i32..440, 1_u32..120, 1_u32..120);
    prop_oneof![
        3 => rect.clone().prop_map(Change::Solid),
        3 => rect.prop_map(Change::Blur),
        3 => (-20_i32..660, -20_i32..500).prop_map(Change::Cursor),
        1 => proptest::bool::weighted(0.3).prop_map(Change::Cover),
        1 => Just(Change::Nothing),
    ]
}

/// Ids that live across the frames of one oracle run.
struct Ids {
    solid: Id,
    page: Id,
    blur: Id,
}

/// One frame of the mixed oracle: the cursor and the lock cover from `output_elements`, then
/// a blur stand-in, a translucent solid and a page.
fn mixed(
    ids: &Ids,
    step: Step,
    renderer: &mut PixmanRenderer,
) -> Vec<CasementElement<PixmanRenderer>> {
    let output = support::output_key("HEADLESS-1");
    let view = OutputView {
        area: LogicalRect::new(LogicalPoint { x: 0, y: 0 }, LogicalSize { w: 640, h: 480 }),
        scale: Scale(120),
        mode: PhysicalSize { w: 640, h: 480 },
    };
    let frame = OutputFrame {
        output: output.clone(),
        strip: StripOffset(SubPx(0)),
        windows: Vec::new(),
    };
    let chrome = ChromeLayer {
        cursor: CursorDraw::Named(CursorShape::Pointer),
        cursor_at: LogicalPoint {
            x: step.cursor.0,
            y: step.cursor.1,
        },
        covers: if step.covered {
            vec![output]
        } else {
            Vec::new()
        },
    };
    let mut elements = output_elements(
        &frame,
        &view,
        &LayerDraw::default(),
        &TextureBook::empty(),
        &chrome,
        renderer,
    );
    let (x, y, w, h) = step.blur;
    let blur = BlurV0Element::new(
        ids.blur.clone(),
        vec![rect(x, y, w, h), rect(x + 10, y + 30, w, h / 2 + 1)],
        GLASS,
    );
    elements.extend([
        CasementElement::Blur(blur),
        CasementElement::Solid(solid_rect(
            ids.solid.clone(),
            {
                let (x, y, w, h) = step.solid;
                rect(x, y, w, h)
            },
            GLASS,
            CommitCounter::default(),
        )),
        CasementElement::Solid(solid_rect(
            ids.page.clone(),
            rect(40, 40, 560, 400),
            WHITE,
            CommitCounter::default(),
        )),
    ]);
    elements
}

/// The mixed oracle: every element kind this crate draws without a client surface.
fn mixed_incremental_matches_full(changes: &[Change]) -> bool {
    let ids = Ids {
        solid: Id::new(),
        page: Id::new(),
        blur: Id::new(),
    };
    let mut renderer = pixman();
    let mut target = renderer.target(SIZE);
    let mut tracker = tracker(SIZE, 1.0);
    let mut step = START;
    let mut last = Vec::new();
    for (n, change) in changes.iter().enumerate() {
        step = change.apply(step);
        let elements = mixed(&ids, step, &mut renderer);
        last = renderer
            .draw(
                &mut target,
                &mut tracker,
                SIZE,
                &elements,
                usize::from(n > 0),
            )
            .1;
    }
    let elements = mixed(&ids, step, &mut renderer);
    last == render_once(&mut renderer, SIZE, 1.0, &elements)
}

proptest! {
    /// The same oracle over the cursor raster, the lock cover coming and going, the blur
    /// stand-in and translucent solids.
    #[test]
    fn incremental_damage_equals_a_full_render_for_every_element_kind(changes in proptest::collection::vec(change(), 1..16)) {
        prop_assert!(mixed_incremental_matches_full(&changes));
    }
}

#[test]
fn an_unchanged_frame_damages_nothing() {
    let ids = Ids {
        solid: Id::new(),
        page: Id::new(),
        blur: Id::new(),
    };
    let still = Step {
        solid: (10, 10, 50, 50),
        blur: (200, 100, 80, 60),
        cursor: (300, 200),
        covered: false,
    };
    let mut renderer = pixman();
    let mut target = renderer.target(SIZE);
    let mut tracker = tracker(SIZE, 1.0);
    let first = mixed(&ids, still, &mut renderer);
    let (rendered, _) = renderer.draw(&mut target, &mut tracker, SIZE, &first, 0);
    assert!(!rendered.damage.is_empty());
    assert_eq!(rendered.sync, RenderSync::Done);
    let again = mixed(&ids, still, &mut renderer);
    let (rendered, _) = renderer.draw(&mut target, &mut tracker, SIZE, &again, 1);
    assert_eq!(
        rendered.damage,
        Vec::new(),
        "the cursor raster, cover and blur keep their ids"
    );
    let moved = mixed(
        &ids,
        Step {
            cursor: (310, 200),
            ..still
        },
        &mut renderer,
    );
    let (rendered, _) = renderer.draw(&mut target, &mut tracker, SIZE, &moved, 1);
    let bounds = rendered
        .damage
        .iter()
        .fold((i32::MAX, i32::MIN), |(lo, hi), r| {
            (lo.min(r.x), hi.max(r.x + r.w as i32))
        });
    assert!(
        bounds.0 >= 290 && bounds.1 <= 340,
        "only the cursor's old and new places: {:?}",
        rendered.damage
    );
}
