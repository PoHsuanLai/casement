//! Which surface is under a point: the pointer focus the scene implies.

use super::SurfaceAt;
use casement_core::geometry::LogicalPoint;
use smithay::desktop::{WindowSurfaceType, utils::under_from_surface_tree};
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::utils::{Logical, Point};

/// A root surface and where its top-left sits, in global logical coordinates.
#[derive(Debug, Clone, PartialEq)]
pub struct Candidate {
    /// The surface (its subsurfaces and popups are searched too).
    pub root: WlSurface,
    /// Its top-left.
    pub origin: LogicalPoint,
}

/// The surface under `at`: the first of `candidates` (the caller lists them front to back:
/// lock surfaces, overlay and top layers, windows by stacking, bottom and background layers)
/// that has a surface, a subsurface or a popup with input there, and where that surface's
/// top-left is. A surface whose input region excludes the point is skipped, so a transparent
/// bar does not swallow the pointer.
pub fn surface_under(candidates: &[Candidate], at: LogicalPoint) -> Option<SurfaceAt> {
    let point: Point<f64, Logical> = Point::from((f64::from(at.x), f64::from(at.y)));
    candidates.iter().find_map(|candidate| {
        let origin: Point<i32, Logical> = Point::from((candidate.origin.x, candidate.origin.y));
        under_from_surface_tree(&candidate.root, point, origin, WindowSurfaceType::ALL).map(
            |(surface, top_left)| SurfaceAt {
                surface,
                origin: LogicalPoint {
                    x: top_left.x,
                    y: top_left.y,
                },
            },
        )
    })
}
