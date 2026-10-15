//! One output's frame as render elements.

pub(crate) mod convert;

use crate::compositor::Compositor;
use crate::element::{
    CasementElement, ClipShape, ClosingFrame, FlightDraw, FrameDraw, GenieWindow, LockCoverElement,
    PopupRide, Rgba8, TitleDraw, choose, closing_elements, cursor_elements, dim_elements,
    drawn_content, frame_elements, genie_window, label_element, layer_elements, popup_elements,
    popup_elements_riding, ring_elements, surface_origin_logical, window_content, window_elements,
};
use crate::element::{alpha, flight_ratio};
use crate::texture_book::TextureBook;
use casement_core::geometry::{
    LogicalPoint, LogicalRect, LogicalSize, PhysicalRect, PhysicalSize, SubPoint,
};
use casement_core::layer::Arranged;
use casement_core::settings::MinimiseEffect;
use casement_core::window::FrameStyle;
use casement_core::{LayerKey, OutputKey, WindowId};
use casement_motion::{Flight, OutputFrame, PlacedWindow};
use convert::{on_output, sub_of};
use ds_core::geometry::scale::Scale;
use ds_core::vocab::Fraction;
use smithay::backend::renderer::element::Kind;
use smithay::backend::renderer::element::surface::render_elements_from_surface_tree;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use std::collections::BTreeMap;

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
    /// The windows whose frame casement draws (server-side decoration), and how it looks. A
    /// window not in it is drawn as its client made it.
    pub frames: BTreeMap<WindowId, FrameStyle>,
    /// The title pixels laid over each framed window's titlebar (made by `casement-chrome`). A
    /// framed window not in it has no title drawn.
    pub titles: BTreeMap<WindowId, TitleDraw>,
    /// How a window in a minimise flight is drawn (`windows.minimise_effect`).
    pub minimise_effect: MinimiseEffect,
    /// Whether Mission Control is drawn, and the window the pointer is on.
    pub overview: OverviewDraw,
    /// The frames of windows that have closed and are still fading out: the snapshot taken
    /// while they lived. Drawn for a window the surface book no longer holds.
    pub closing: BTreeMap<WindowId, ClosingFrame>,
    /// A plain backdrop over the whole output, behind the full-screen windows and above the top
    /// layers: what shows between and beside the halves of a Split View.
    pub backdrop: Option<Rgba8>,
}

/// Mission Control on an output: its windows are drawn scaled to the size they have been sampled
/// at (their slots), their frames round them, over a dimmed picture.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum OverviewDraw {
    /// Not drawn: the desktop.
    #[default]
    Off,
    /// Drawn, with this window under the pointer, if any.
    On {
        /// The hovered window.
        hover: Option<OverviewHover>,
    },
}

/// The window under the pointer in Mission Control: ringed, with its title under it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverviewHover {
    /// The window.
    pub window: WindowId,
    /// Its title, as a raster of this logical size (centred under the window); none for a window
    /// with no title to show.
    pub label: Option<(LogicalSize, TitleDraw)>,
}

/// The gap between a window's frame and its title in Mission Control, in logical pixels.
const LABEL_GAP: i32 = 8;

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
        elements.push(CasementElement::Cover(cover(view, book, COVER)));
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
                let title = layers.titles.get(&placed.window);
                let Some(entry) = book.window(&placed.window) else {
                    // A window that has closed: its frame fades and shrinks with it.
                    if let Some(closing) = layers.closing.get(&placed.window) {
                        elements.extend(closing_elements(
                            placed, closing, title, view, book, renderer,
                        ));
                    }
                    continue;
                };
                let (flown, effect) =
                    in_overview(placed, window_content(placed, &entry.root), layers);
                let placed = &flown;
                let flight = choose(effect, placed.flight);
                let style = layers.frames.get(&placed.window);
                // A window casement frames is clipped at its bottom corners, unless it
                // covers the screen (macOS drops the radius in full screen; a zoomed
                // window keeps it). A window with its own frame is not in `frames`.
                let clip_radius = style
                    .filter(|_| !layers.full_screen.contains(&placed.window))
                    .map(|style| style.look.radius);
                if let FlightDraw::Genie(flight) = flight {
                    // The genie cuts the window as it stands at rest, and the strips go
                    // where the curve says. Its popups are closed (a menu does not ride
                    // a minimise on a Mac, and a strip cannot carry what hangs outside the
                    // window).
                    let window = GenieWindow {
                        placed,
                        flight,
                        style,
                        title,
                        clip_radius,
                    };
                    elements.extend(genie_window(window, &entry.root, view, book, renderer));
                    continue;
                }
                let origin = surface_origin_logical(
                    &entry.root,
                    LogicalPoint {
                        x: placed.at.x.round_logical(),
                        y: placed.at.y.round_logical(),
                    },
                );
                match flight {
                    FlightDraw::Still => {
                        elements.extend(popup_elements(&entry.root, origin, view, renderer));
                    }
                    // A Scale flight: the popups shrink about the window's corner and fade
                    // with it.
                    _ => {
                        let about = LogicalPoint {
                            x: placed.at.x.round_logical(),
                            y: placed.at.y.round_logical(),
                        };
                        let ratio = flight_ratio(placed, &entry.root);
                        let ride = PopupRide::new(about, ratio, alpha(placed.opacity));
                        elements.extend(popup_elements_riding(
                            &entry.root,
                            origin,
                            ride,
                            view,
                            renderer,
                        ));
                    }
                }
                // The frame goes round the content as it is drawn: shrunk, for a window
                // that flies scaled.
                let content = drawn_content(placed, window_content(placed, &entry.root), effect);
                let clip = clip_radius.map(|radius| ClipShape::of_content(content, radius, view));
                let (front, back) = style
                    .map(|style| {
                        frame_elements(
                            placed.window,
                            FrameDraw {
                                style,
                                title,
                                content,
                                opacity: placed.opacity,
                            },
                            view,
                            book,
                            renderer,
                        )
                    })
                    .unwrap_or_default();
                if let OverviewDraw::On { hover: Some(hover) } = &layers.overview
                    && hover.window == placed.window
                {
                    let visual = style.map_or(content, |style| style.around(content).outline());
                    elements.extend(ring_elements(visual, view));
                    elements.extend(hover.label.as_ref().and_then(|(size, draw)| {
                        let at = label_place(visual, *size);
                        label_element(draw, at, view, renderer)
                    }));
                }
                elements.extend(front);
                elements.extend(window_elements(placed, clip, view, book, renderer));
                elements.extend(back);
            }
        }
    };
    draw_level(&layers.overlay, &mut elements, renderer);
    draw_windows(&covering, &mut elements, renderer);
    if let Some(colour) = layers.backdrop {
        elements.push(CasementElement::Cover(cover(view, book, colour)));
    }
    draw_level(&layers.top, &mut elements, renderer);
    draw_windows(&others, &mut elements, renderer);
    if matches!(layers.overview, OverviewDraw::On { .. }) {
        elements.extend(dim_elements(view, book));
    }
    draw_level(&layers.bottom, &mut elements, renderer);
    draw_level(&layers.background, &mut elements, renderer);
    elements
}

/// `placed` as Mission Control draws it, and the effect to draw it under: while the overview is
/// on, a window that is not in a minimise flight is drawn as one that is shrunk (or grown) to its
/// sampled size about its place, so its surfaces, frame and clip follow its slot. The "flight"
/// has the same two ends and goes nowhere. Anything else is as sampled.
fn in_overview(
    placed: &PlacedWindow,
    content: LogicalRect,
    layers: &LayerDraw,
) -> (PlacedWindow, MinimiseEffect) {
    match (&layers.overview, placed.flight) {
        (OverviewDraw::On { .. }, None) => (
            PlacedWindow {
                flight: Some(Flight::new(content, content, Fraction(0))),
                ..*placed
            },
            MinimiseEffect::Scale,
        ),
        _ => (*placed, layers.minimise_effect),
    }
}

/// Where a label of `size` goes: centred under `visual`, a gap below it.
fn label_place(visual: LogicalRect, size: LogicalSize) -> LogicalRect {
    let across = i64::from(visual.origin.x) + (i64::from(visual.size.w) - i64::from(size.w)) / 2;
    let below = i64::from(visual.origin.y) + i64::from(visual.size.h) + i64::from(LABEL_GAP);
    let whole = |n: i64| i32::try_from(n).unwrap_or(0);
    LogicalRect::new(
        LogicalPoint {
            x: whole(across),
            y: whole(below),
        },
        size,
    )
}

/// The cover's colour: opaque black.
const COVER: Rgba8 = Rgba8 {
    r: 0,
    g: 0,
    b: 0,
    a: 255,
};

/// An opaque cover over all of `view`; its id is `book`'s, so a standing cover damages nothing
/// (the lock cover and the backdrop never share a frame).
fn cover(view: &OutputView, book: &TextureBook, colour: Rgba8) -> LockCoverElement {
    let area = PhysicalRect {
        x: 0,
        y: 0,
        w: view.mode.w,
        h: view.mode.h,
    };
    LockCoverElement::new(book.cache().cover_id(), area, colour)
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
