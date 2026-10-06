//! One output's frame as render elements.

pub(crate) mod convert;

use crate::compositor::Compositor;
use crate::element::{
    CasementElement, LockCoverElement, Rgba8, cursor_elements, layer_elements, popup_elements,
    surface_origin_logical, window_elements,
};
use crate::texture_book::TextureBook;
use casement_core::geometry::{LogicalPoint, LogicalRect, PhysicalRect, PhysicalSize, SubPoint};
use casement_core::layer::Arranged;
use casement_core::{LayerKey, OutputKey, WindowId};
use casement_motion::{OutputFrame, PlacedWindow};
use convert::{on_output, sub_of};
use ds_core::geometry::scale::Scale;
use smithay::backend::renderer::element::surface::render_elements_from_surface_tree;
use smithay::backend::renderer::element::{Id, Kind};
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;

/// Where an output sits and how dense it is: what turns the frame's global sub-pixels into this
/// output's physical pixels. (The plan's sketch had no source for it; FINDINGS.)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OutputView {
    /// The output's rectangle in global logical coordinates.
    pub area: LogicalRect,
    /// Its scale.
    pub scale: Scale,
    /// Its mode's size in physical pixels.
    pub mode: PhysicalSize,
}

/// The layer-shell surfaces on an output, per level, bottom of each level first, and the
/// full-screen windows, which are drawn between the top and overlay levels.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LayerDraw {
    /// The full-screen windows on the output: above `top` (the bar and dock), below `overlay`.
    pub full_screen: Vec<WindowId>,
    /// `background`.
    pub background: Vec<LayerKey>,
    /// `bottom`.
    pub bottom: Vec<LayerKey>,
    /// `top`.
    pub top: Vec<LayerKey>,
    /// `overlay`.
    pub overlay: Vec<LayerKey>,
    /// Where each of them is arranged: the one placement the hit test reads too
    /// (`LayerBook::arrange`). `None` when there are no layers.
    pub arranged: Option<Arranged>,
}

/// What the compositor draws over everything: the cursor and lock covers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChromeLayer {
    /// The cursor.
    pub cursor: CursorDraw,
    /// Where the cursor's hotspot is, in global logical coordinates.
    pub cursor_at: LogicalPoint,
    /// Outputs covered because the session is locked and their lock surface is missing or its
    /// locker died (H3: "a dead locker leaves the lock cover").
    pub covers: Vec<OutputKey>,
}

/// How the cursor is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CursorDraw {
    /// Not at all.
    Hidden,
    /// From the compositor's cursor theme.
    Named(CursorShape),
    /// The client surface in [`TextureBook::cursor`], with this hotspot. (The sketch said
    /// `Surface(WindowId)`; a cursor surface is not a window: FINDINGS.)
    ClientSurface {
        /// The hotspot inside the surface.
        hotspot: LogicalPoint,
    },
}

/// A named cursor (`wp_cursor_shape_v1` and the compositor's own grabs).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum CursorShape {
    /// The arrow.
    Default,
    /// The I-beam.
    Text,
    /// The pointing hand.
    Pointer,
    /// An open hand.
    Grab,
    /// A closed hand.
    Grabbing,
    /// Resize up and down.
    ResizeNs,
    /// Resize left and right.
    ResizeEw,
    /// Resize along the rising diagonal.
    ResizeNesw,
    /// Resize along the falling diagonal.
    ResizeNwse,
    /// Busy.
    Wait,
    /// Refused.
    NotAllowed,
}

/// The elements of one output's frame, front first (smithay's order): chrome (cursor, lock
/// covers), overlay layers, full-screen windows, top layers, the frame's other windows top of
/// the stack first with their popups, bottom and background layers. Windows and layers missing from `book` are skipped.
///
/// A locked output (one with a lock surface in `book`, or listed in `chrome.covers`) shows the
/// cursor, its lock surface if any, and under it an opaque cover over the whole output; nothing
/// of the session is drawn, so a translucent, small or dead locker reveals nothing.
pub fn output_elements<R: Compositor>(
    frame: &OutputFrame,
    view: &OutputView,
    layers: &LayerDraw,
    book: &TextureBook,
    chrome: &ChromeLayer,
    renderer: &mut R,
) -> Vec<CasementElement<R>> {
    let mut elements = cursor_elements(chrome, view, book, renderer);
    let lock = book.lock(&frame.output);
    if lock.is_some() || chrome.covers.contains(&frame.output) {
        if let Some(entry) = lock {
            elements.extend(tree(&entry.root, sub_of(view.area.origin), view, renderer));
        }
        elements.push(CasementElement::Cover(cover(view)));
        return elements;
    }
    let draw_level =
        |keys: &[LayerKey], elements: &mut Vec<CasementElement<R>>, renderer: &mut R| {
            for key in keys.iter().rev() {
                let Some(entry) = book.layer(key) else {
                    continue;
                };
                let Some(area) = layers
                    .arranged
                    .as_ref()
                    .and_then(|arranged| arranged.place(key, entry.committed))
                else {
                    continue;
                };
                elements.extend(popup_elements(&entry.root, area.origin, view, renderer));
                elements.extend(layer_elements(key, area, view, book, renderer));
            }
        };
    let (covering, others): (Vec<_>, Vec<_>) = frame
        .windows
        .iter()
        .partition(|placed| layers.full_screen.contains(&placed.window));
    let draw_windows =
        |placed: &[&PlacedWindow], elements: &mut Vec<CasementElement<R>>, renderer: &mut R| {
            for placed in placed.iter().rev() {
                if let Some(entry) = book.window(&placed.window) {
                    let origin = surface_origin_logical(
                        &entry.root,
                        LogicalPoint {
                            x: placed.at.x.round_logical(),
                            y: placed.at.y.round_logical(),
                        },
                    );
                    elements.extend(popup_elements(&entry.root, origin, view, renderer));
                    elements.extend(window_elements(placed, view, book, renderer));
                }
            }
        };
    draw_level(&layers.overlay, &mut elements, renderer);
    draw_windows(&covering, &mut elements, renderer);
    draw_level(&layers.top, &mut elements, renderer);
    draw_windows(&others, &mut elements, renderer);
    draw_level(&layers.bottom, &mut elements, renderer);
    draw_level(&layers.background, &mut elements, renderer);
    elements
}

/// The cover's colour: opaque black.
const COVER: Rgba8 = Rgba8 {
    r: 0,
    g: 0,
    b: 0,
    a: 255,
};

thread_local! {
    /// The cover's id: one for every output (each output has its own damage tracker), kept
    /// for the life of the thread so a standing cover damages nothing.
    static COVER_ID: Id = Id::new();
}

/// The lock cover over all of `view`.
fn cover(view: &OutputView) -> LockCoverElement {
    let area = PhysicalRect {
        x: 0,
        y: 0,
        w: view.mode.w,
        h: view.mode.h,
    };
    LockCoverElement::new(COVER_ID.with(Id::clone), area, COVER)
}

/// A surface tree with its top-left at `origin` (global sub-pixels), opaque, front first.
fn tree<R: Compositor>(
    root: &WlSurface,
    origin: SubPoint,
    view: &OutputView,
    renderer: &mut R,
) -> Vec<CasementElement<R>> {
    render_elements_from_surface_tree(
        renderer,
        root,
        on_output(origin, view),
        view.scale.as_f64(),
        1.0,
        Kind::Unspecified,
    )
}
