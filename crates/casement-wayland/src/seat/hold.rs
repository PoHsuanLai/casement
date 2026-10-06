//! What an active pointer constraint does to a motion: a locked pointer stays put, a confined one
//! stops at the edge of its room (the surface cut to the region), and a motion that asks for
//! more is held there.

use super::SurfaceAt;
use super::edge::whole_of;
use super::focus_origin;
use casement_core::geometry::LogicalPoint;
use smithay::desktop::utils::bbox_from_surface_tree;
use smithay::input::SeatHandler;
use smithay::input::pointer::PointerHandle;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::utils::{Logical, Point, Rectangle};
use smithay::wayland::compositor::{RectangleKind, RegionAttributes};
use smithay::wayland::pointer_constraints::{PointerConstraint, with_pointer_constraint};

/// What an active constraint does to a motion over its surface.
#[derive(Debug)]
pub(super) enum Hold<S> {
    /// Nothing holds the pointer.
    Free,
    /// The pointer stays where it is.
    Locked(S),
    /// The pointer stays inside the region and the surface.
    Confined(S, Confine),
}

/// Where a confined pointer may be: its surface's top-left, the room it has in surface-local
/// pixels (the surface cut to the region's extent), and the region itself.
#[derive(Debug)]
pub(super) struct Confine {
    origin: Point<f64, Logical>,
    room: Rectangle<i32, Logical>,
    region: Option<RegionAttributes>,
}

/// The constraint that is active on the surface the pointer is over.
pub(super) fn hold_of<D>(pointer: &PointerHandle<D>) -> Hold<WlSurface>
where
    D: SeatHandler + 'static,
    D::PointerFocus: Into<Option<WlSurface>>,
{
    let Some(surface) = pointer.current_focus().and_then(Into::into) else {
        return Hold::Free;
    };
    let held =
        with_pointer_constraint::<D, _, _>(&surface, pointer, |constraint| match constraint {
            Some(held) if held.is_active() => match &*held {
                PointerConstraint::Locked(_) => Some(None),
                PointerConstraint::Confined(_) => Some(Some(held.region().cloned())),
            },
            Some(_) | None => None,
        });
    // The surface's data is not held here: the room reads the surface tree.
    match held {
        None => Hold::Free,
        Some(None) => Hold::Locked(surface),
        Some(Some(region)) => match focus_origin::of(&surface).map(whole_of) {
            Some(origin) => {
                let confine = confine_of(&surface, origin, region);
                Hold::Confined(surface, confine)
            }
            None => Hold::Free,
        },
    }
}

/// The surface a confined pointer stays on, as a focus.
pub(super) fn confined_focus(hold: &Hold<WlSurface>) -> Option<SurfaceAt> {
    match hold {
        Hold::Confined(surface, confine) => Some(SurfaceAt {
            surface: surface.clone(),
            origin: LogicalPoint {
                x: confine.origin.x.round() as i32,
                y: confine.origin.y.round() as i32,
            },
        }),
        Hold::Free | Hold::Locked(_) => None,
    }
}

/// The room a confinement leaves: the surface tree's extent, cut to the bounds of the region's
/// added rectangles (the whole surface without a region).
fn confine_of(
    surface: &WlSurface,
    origin: Point<f64, Logical>,
    region: Option<RegionAttributes>,
) -> Confine {
    let extent = bbox_from_surface_tree(surface, (0, 0));
    let room = region
        .as_ref()
        .and_then(|region| {
            region
                .rects
                .iter()
                .filter(|(kind, _)| matches!(kind, RectangleKind::Add))
                .map(|(_, rect)| *rect)
                .reduce(|a, b| a.merge(b))
        })
        .and_then(|bounds| bounds.intersection(extent))
        .unwrap_or(extent);
    Confine {
        origin,
        room,
        region,
    }
}

/// `wanted` held to the room, its last pixel being the surface's last (a confined pointer that is
/// pushed past an edge stops at it).
fn pinned(confine: &Confine, wanted: Point<f64, Logical>) -> Point<f64, Logical> {
    let (left, top) = (f64::from(confine.room.loc.x), f64::from(confine.room.loc.y));
    let (right, bottom) = (
        left + f64::from((confine.room.size.w - 1).max(0)),
        top + f64::from((confine.room.size.h - 1).max(0)),
    );
    let local = wanted - confine.origin;
    let held = Point::from((local.x.clamp(left, right), local.y.clamp(top, bottom)));
    confine.origin + held
}

/// Whether `at` (global) is inside `region` of the surface whose origin is `origin`.
fn inside(
    region: Option<&RegionAttributes>,
    origin: Point<f64, Logical>,
    at: Point<f64, Logical>,
) -> bool {
    let local = at - origin;
    region.is_none_or(|region| region.contains(local.to_i32_floor()))
}

/// Where a motion to `wanted` over `over` (a surface and its origin) ends up under `hold`:
/// `None` when the pointer may not move at all (locked, or confined with no place in the region
/// to stop at), else where it goes: `wanted`, or for a confined pointer the nearest place the
/// confinement allows.
pub(super) fn settled<S: PartialEq>(
    hold: &Hold<S>,
    over: Option<(&S, Point<f64, Logical>)>,
    wanted: Point<f64, Logical>,
) -> Option<Point<f64, Logical>> {
    let origin_over = |held: &S| {
        over.filter(|(surface, _)| *surface == held)
            .map(|(_, at)| at)
    };
    match hold {
        Hold::Free => Some(wanted),
        Hold::Locked(held) => match origin_over(held) {
            Some(_) => None,
            None => Some(wanted),
        },
        Hold::Confined(_, confine) => {
            let at = pinned(confine, wanted);
            inside(confine.region.as_ref(), confine.origin, at).then_some(at)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use smithay::utils::Rectangle;
    use smithay::wayland::compositor::RectangleKind;

    fn at(x: f64, y: f64) -> Point<f64, Logical> {
        Point::from((x, y))
    }

    fn region(width: i32, height: i32) -> RegionAttributes {
        RegionAttributes {
            rects: vec![(
                RectangleKind::Add,
                Rectangle::new((0, 0).into(), (width, height).into()),
            )],
        }
    }

    const HERE: u32 = 7;
    const ELSEWHERE: u32 = 8;

    #[test]
    fn a_free_pointer_goes_where_it_is_sent() {
        let moved = settled(&Hold::Free, Some((&HERE, at(0.0, 0.0))), at(5.0, 5.0));
        assert_eq!(moved, Some(at(5.0, 5.0)));
    }

    #[test]
    fn a_locked_pointer_stays_put_over_its_surface_and_moves_off_it() {
        let hold = Hold::Locked(HERE);
        let over_it = settled(&hold, Some((&HERE, at(0.0, 0.0))), at(5.0, 5.0));
        assert_eq!(over_it, None);
        let off_it = settled(&hold, Some((&ELSEWHERE, at(0.0, 0.0))), at(5.0, 5.0));
        assert_eq!(off_it, Some(at(5.0, 5.0)));
        assert_eq!(settled(&hold, None, at(5.0, 5.0)), Some(at(5.0, 5.0)));
    }

    fn confined(region: Option<RegionAttributes>, origin: Point<f64, Logical>) -> Hold<u32> {
        Hold::Confined(
            HERE,
            Confine {
                origin,
                room: Rectangle::new((0, 0).into(), (100, 100).into()),
                region,
            },
        )
    }

    #[test]
    fn a_confined_pointer_is_held_at_the_edge_of_its_room() {
        let hold = confined(Some(region(100, 100)), at(10.0, 10.0));
        // (50, 50) is (40, 40) inside the room: it goes there. Past the right and bottom edges
        // it stops on the last pixel; past the left and top it stops on the first.
        assert_eq!(settled(&hold, None, at(50.0, 50.0)), Some(at(50.0, 50.0)));
        assert_eq!(
            settled(&hold, None, at(900.0, 900.0)),
            Some(at(109.0, 109.0))
        );
        assert_eq!(settled(&hold, None, at(-9.0, 50.0)), Some(at(10.0, 50.0)));
    }

    #[test]
    fn a_confinement_without_a_region_covers_the_whole_surface() {
        let hold = confined(None, at(0.0, 0.0));
        assert_eq!(settled(&hold, None, at(40.0, 40.0)), Some(at(40.0, 40.0)));
        assert_eq!(settled(&hold, None, at(900.0, -900.0)), Some(at(99.0, 0.0)));
    }

    #[test]
    fn a_confinement_to_a_hole_in_the_region_goes_nowhere() {
        let mut holey = region(100, 100);
        holey.rects.push((
            RectangleKind::Subtract,
            Rectangle::new((40, 40).into(), (20, 20).into()),
        ));
        let hold = confined(Some(holey), at(0.0, 0.0));
        assert_eq!(settled(&hold, None, at(50.0, 50.0)), None);
        assert_eq!(settled(&hold, None, at(10.0, 10.0)), Some(at(10.0, 10.0)));
    }
}
