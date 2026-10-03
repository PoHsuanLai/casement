//! `ext_background_effect_manager_v1`: a surface asks for the blur behind a region (C33: the
//! only blur protocol casement serves). Stored at P0 and drawn by blur v0.
//!
//! The region is double-buffered: smithay keeps the pending region in the surface's cached
//! state (the request writes it, the surface commit makes it current), so casement stores
//! nothing of its own. The renderer asks [`blur_region_of`] for the committed one; a surface
//! that never asked, or asked to unset, has none. smithay hands the region over as a list of
//! add and subtract rectangles; [`region_rects`] flattens it into disjoint rectangles.

use crate::states::GlobalFilter;
use casement_core::Event;
use casement_core::geometry::LogicalRect;
use smithay::reexports::wayland_protocols::ext::background_effect::v1::server::ext_background_effect_manager_v1::ExtBackgroundEffectManagerV1;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::reexports::wayland_server::{DisplayHandle, GlobalDispatch};
use smithay::wayland::GlobalData;
use smithay::wayland::background_effect::{
    BackgroundEffectState, BackgroundEffectSurfaceCachedState, ExtBackgroundEffectHandler,
};
use smithay::wayland::compositor::{RectangleKind, RegionAttributes, with_states};
use crate::convert::{rect_of, rect_to};
use smithay::utils::{Logical, Rectangle};

/// `ext_background_effect_manager_v1`.
#[derive(Debug)]
pub struct BackgroundEffectPart {
    /// smithay's state.
    pub state: BackgroundEffectState,
}

impl BackgroundEffectPart {
    /// Creates the global (version 1). Every connection sees it.
    pub fn new<D>(display: &DisplayHandle, role_filter: GlobalFilter) -> BackgroundEffectPart
    where
        D: ExtBackgroundEffectHandler + GlobalDispatch<ExtBackgroundEffectManagerV1, GlobalData>,
    {
        let _ = role_filter;
        BackgroundEffectPart {
            state: BackgroundEffectState::new::<D>(display),
        }
    }
}

/// `surface` set its blur region (surface-local logical rectangles), or cleared it with an
/// empty one. The model has no blur event (blur v0 reads [`blur_region_of`] at render), so
/// nothing is fed to it; a request that names no area is logged and treated as a clear.
pub fn on_blur_region(
    part: &mut BackgroundEffectPart,
    surface: &WlSurface,
    region: Vec<LogicalRect>,
) -> Vec<Event> {
    let _ = (part, surface);
    let area = region.iter().filter(|rect| has_area(rect)).count();
    tracing::debug!(rects = area, "blur region requested");
    Vec::new()
}

/// A region's add and subtract rectangles, applied in order, as disjoint rectangles in the
/// order they were added.
pub fn region_rects(region: &RegionAttributes) -> Vec<LogicalRect> {
    region
        .rects
        .iter()
        .fold(Vec::new(), |covered, (kind, rect)| match kind {
            RectangleKind::Add => add_rect(covered, *rect),
            RectangleKind::Subtract => Rectangle::subtract_rects_many(covered, [*rect]),
        })
        .into_iter()
        .map(rect_of)
        .filter(has_area)
        .collect()
}

/// `covered` with the part of `rect` it does not already cover appended.
fn add_rect(
    covered: Vec<Rectangle<i32, Logical>>,
    rect: Rectangle<i32, Logical>,
) -> Vec<Rectangle<i32, Logical>> {
    let new = rect.subtract_rects(covered.iter().copied());
    covered.into_iter().chain(new).collect()
}

/// Whether the rectangle covers any area.
fn has_area(rect: &LogicalRect) -> bool {
    rect.size.w > 0 && rect.size.h > 0
}

/// The blur region `surface` has committed, `None` when it has none (never asked, or unset).
pub fn blur_region_of(surface: &WlSurface) -> Option<Vec<LogicalRect>> {
    with_states(surface, |states| {
        let mut cached = states
            .cached_state
            .get::<BackgroundEffectSurfaceCachedState>();
        cached.current().blur_region.as_ref().map(region_rects)
    })
}

/// A region made of these rectangles, for tests and for blur v0's own scenes.
pub fn region_of(rects: &[LogicalRect]) -> RegionAttributes {
    RegionAttributes {
        rects: rects
            .iter()
            .map(|rect| (RectangleKind::Add, rect_to(*rect)))
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x: i32, y: i32, w: i32, h: i32) -> Rectangle<i32, Logical> {
        Rectangle::new((x, y).into(), (w, h).into())
    }

    fn area(rects: &[LogicalRect]) -> u64 {
        rects
            .iter()
            .map(|rect| u64::from(rect.size.w) * u64::from(rect.size.h))
            .sum()
    }

    #[test]
    fn overlapping_adds_count_their_union_once() {
        let region = RegionAttributes {
            rects: vec![
                (RectangleKind::Add, rect(0, 0, 10, 10)),
                (RectangleKind::Add, rect(5, 5, 10, 10)),
            ],
        };
        assert_eq!(area(&region_rects(&region)), 100 + 100 - 25);
    }

    #[test]
    fn a_subtract_cuts_what_was_added_before_it_only() {
        let region = RegionAttributes {
            rects: vec![
                (RectangleKind::Add, rect(0, 0, 10, 10)),
                (RectangleKind::Subtract, rect(0, 0, 5, 5)),
                (RectangleKind::Add, rect(0, 0, 2, 2)),
            ],
        };
        assert_eq!(area(&region_rects(&region)), 100 - 25 + 4);
    }

    #[test]
    fn an_empty_region_has_no_rectangles_and_degenerate_ones_drop() {
        assert!(region_rects(&RegionAttributes::default()).is_empty());
        let flat = RegionAttributes {
            rects: vec![(RectangleKind::Add, rect(3, 3, 0, 9))],
        };
        assert!(region_rects(&flat).is_empty());
    }

    #[test]
    fn a_region_of_rectangles_flattens_back_to_them() {
        let rects = [LogicalRect::new(
            casement_core::geometry::LogicalPoint { x: 1, y: 2 },
            casement_core::geometry::LogicalSize { w: 30, h: 40 },
        )];
        assert_eq!(region_rects(&region_of(&rects)), rects);
    }
}
