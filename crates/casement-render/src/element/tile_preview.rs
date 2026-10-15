//! The preview a drag to a screen edge shows: the rectangle the window would tile to, a rounded
//! translucent plate (Sequoia). Three solid bands and four corner rasters, so the plate is
//! exact on whole pixels and only the corners are anti-aliased; the corners are one quadrant of a
//! circle each, a pure function of the radius and the scale, so a golden holds.

use super::frame::{Raster, element, to_buffer, whole};
use super::{CasementElement, Rgba8, solid_rect};
use crate::compositor::Compositor;
use crate::scene::OutputView;
use crate::scene::convert::{on_output, sub_of};
use casement_core::WindowId;
use casement_core::geometry::{LogicalPoint, LogicalRect, LogicalSize, PhysicalRect, buffer_size};
use ds_core::geometry::scale::Scale;
use smithay::backend::renderer::element::Id;
use smithay::backend::renderer::utils::CommitCounter;

/// The preview of one tile drag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TilePreview {
    /// The window being dragged: the plate is drawn behind it and above every other window.
    pub window: WindowId,
    /// The rectangle the window would take, in global logical pixels.
    pub rect: LogicalRect,
    /// The plate's colour (premultiplied, translucent).
    pub fill: Rgba8,
    /// The plate's corner radius, in logical pixels.
    pub radius: u32,
}

/// A corner of the plate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Corner {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

impl Corner {
    const ALL: [Corner; 4] = [
        Corner::TopLeft,
        Corner::TopRight,
        Corner::BottomLeft,
        Corner::BottomRight,
    ];

    /// Whether the corner's quadrant is the top-left one mirrored across (right) and down
    /// (bottom): the signs of its axes.
    fn signs(self) -> (i32, i32) {
        match self {
            Corner::TopLeft => (1, 1),
            Corner::TopRight => (-1, 1),
            Corner::BottomLeft => (1, -1),
            Corner::BottomRight => (-1, -1),
        }
    }
}

/// The plate's elements on `view`, front first. Nothing for a plate off this output or without
/// room.
pub fn tile_preview_elements<R: Compositor>(
    preview: &TilePreview,
    view: &OutputView,
    renderer: &mut R,
) -> Vec<CasementElement<R>> {
    let rect = preview.rect;
    if view.area.intersection(&rect).is_none() {
        return Vec::new();
    }
    let radius = preview.radius.min(rect.size.w / 2).min(rect.size.h / 2);
    let bands = bands(rect, radius)
        .into_iter()
        .filter(|band| band.size.w > 0 && band.size.h > 0)
        .map(|band| {
            CasementElement::Solid(solid_rect(
                Id::new(),
                physical(band, view),
                preview.fill,
                CommitCounter::default(),
            ))
        });
    let corners = corners(rect, radius)
        .into_iter()
        .filter(|_| radius > 0)
        .filter_map(|(corner, at)| {
            let raster = corner_raster(corner, radius, preview.fill, view.scale);
            let pixels = LogicalSize {
                w: radius,
                h: radius,
            };
            element(
                renderer,
                view,
                at,
                pixels,
                whole(raster.w, raster.h),
                1.0,
                &to_buffer(&raster),
            )
            .map(CasementElement::Memory)
        });
    corners.chain(bands).collect()
}

/// The three bands that make the plate between its corners: above, across and below.
fn bands(rect: LogicalRect, radius: u32) -> [LogicalRect; 3] {
    let at = |dx: u32, dy: u32| LogicalPoint {
        x: rect.origin.x.saturating_add_unsigned(dx),
        y: rect.origin.y.saturating_add_unsigned(dy),
    };
    let across = rect.size.w - 2 * radius;
    [
        LogicalRect::new(
            at(radius, 0),
            LogicalSize {
                w: across,
                h: radius,
            },
        ),
        LogicalRect::new(
            at(0, radius),
            LogicalSize {
                w: rect.size.w,
                h: rect.size.h - 2 * radius,
            },
        ),
        LogicalRect::new(
            at(radius, rect.size.h - radius),
            LogicalSize {
                w: across,
                h: radius,
            },
        ),
    ]
}

/// Where each corner's square goes (its top-left point).
fn corners(rect: LogicalRect, radius: u32) -> [(Corner, LogicalPoint); 4] {
    let at = |dx: u32, dy: u32| LogicalPoint {
        x: rect.origin.x.saturating_add_unsigned(dx),
        y: rect.origin.y.saturating_add_unsigned(dy),
    };
    let (right, bottom) = (rect.size.w - radius, rect.size.h - radius);
    Corner::ALL.map(|corner| {
        let place = match corner {
            Corner::TopLeft => at(0, 0),
            Corner::TopRight => at(right, 0),
            Corner::BottomLeft => at(0, bottom),
            Corner::BottomRight => at(right, bottom),
        };
        (corner, place)
    })
}

/// `rect` on `view` in physical pixels, both edges rounded the way every element's are, so the
/// bands meet without a seam.
fn physical(rect: LogicalRect, view: &OutputView) -> PhysicalRect {
    let far = LogicalPoint {
        x: rect.origin.x.saturating_add_unsigned(rect.size.w),
        y: rect.origin.y.saturating_add_unsigned(rect.size.h),
    };
    let (from, to) = (
        on_output(sub_of(rect.origin), view),
        on_output(sub_of(far), view),
    );
    PhysicalRect {
        x: from.x,
        y: from.y,
        w: u32::try_from(to.x - from.x).unwrap_or(0),
        h: u32::try_from(to.y - from.y).unwrap_or(0),
    }
}

/// One corner's square, `radius` logical pixels a side at `scale`: the plate's colour where the
/// circle of that radius, centred on the plate's inside corner of the square, covers the pixel,
/// anti-aliased by the signed distance to the circle. ARGB8888, premultiplied.
fn corner_raster(corner: Corner, radius: u32, fill: Rgba8, scale: Scale) -> Raster {
    let side = buffer_size(
        LogicalSize {
            w: radius,
            h: radius,
        },
        scale,
    );
    let (w, h) = (side.w.max(1), side.h.max(1));
    let per = scale.numerator() as f32 / Scale::DENOMINATOR as f32;
    let r = radius as f32;
    let (sx, sy) = corner.signs();
    let argb = (0..h)
        .flat_map(|y| (0..w).map(move |x| (x, y)))
        .flat_map(|(x, y)| {
            // The pixel's centre in logical pixels from the square's own top-left, mirrored so
            // every corner is the top-left one.
            let (cx, cy) = ((x as f32 + 0.5) / per, (y as f32 + 0.5) / per);
            let at = (
                if sx > 0 { cx } else { r - cx },
                if sy > 0 { cy } else { r - cy },
            );
            let distance = (at.0 - r).hypot(at.1 - r) - r;
            let cover = (0.5 - distance * per).clamp(0.0, 1.0);
            let level = |v: u8| (f32::from(v) * cover).round() as u8;
            [level(fill.b), level(fill.g), level(fill.r), level(fill.a)]
        })
        .collect();
    Raster { w, h, argb }
}
