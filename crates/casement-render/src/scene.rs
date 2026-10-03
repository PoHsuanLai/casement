//! One output's frame as render elements.

use crate::compositor::Compositor;
use crate::element::CasementElement;
use crate::texture_book::TextureBook;
use casement_core::geometry::{LogicalPoint, LogicalRect, PhysicalSize};
use casement_core::{LayerKey, OutputKey};
use casement_motion::OutputFrame;
use ds_core::geometry::scale::Scale;

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

/// The layer-shell surfaces on an output, per level, bottom of each level first.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LayerDraw {
    /// `background`.
    pub background: Vec<LayerKey>,
    /// `bottom`.
    pub bottom: Vec<LayerKey>,
    /// `top`.
    pub top: Vec<LayerKey>,
    /// `overlay`.
    pub overlay: Vec<LayerKey>,
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
/// covers), overlay and top layers, the frame's windows top of the stack first with their
/// popups, bottom and background layers. Windows and layers missing from `book` are skipped.
pub fn output_elements<R: Compositor>(
    frame: &OutputFrame,
    view: &OutputView,
    layers: &LayerDraw,
    book: &TextureBook,
    chrome: &ChromeLayer,
    renderer: &mut R,
) -> Vec<CasementElement<R>> {
    let _ = (frame, view, layers, book, chrome, renderer);
    todo!("P0-R-scene: output_elements")
}
