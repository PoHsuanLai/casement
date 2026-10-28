//! What `quire_window_v1` requests mean, as pure functions: the double-buffered values, the
//! role the client may ask for, the accessibility root, the first-mouse regions, what a
//! placement request does at each phase, and what destroying the object forgets.

use crate::convert::rect_of;
use casement_core::geometry::LogicalRect;
use casement_core::window::{A11yRoot, FrameHint};
use casement_core::{Event, InputSerial, ToplevelRequest, WindowHintChange, WindowId};
use casement_protocols::vocab::{Edited, FrameMode, PlaceTarget, WindowRole};
use smithay::wayland::compositor::{RectangleKind, RegionAttributes};

/// The longest D-Bus name or object path an accessibility root may carry (the D-Bus limit for
/// a name is 255; a path longer than that is not one we pass on).
pub const A11Y_MAX_LEN: usize = 255;

/// The most rectangles of a `wl_region` that are read for the first-mouse regions (CASC-16): the
/// subtraction test is every added rectangle against every subtracted one on the loop, so a
/// region of a million rectangles must not be read whole. Rectangles past it are ignored.
pub const MAX_FIRST_MOUSE_RECTS: usize = 64;

/// Values `set_first_mouse` and `set_frame` staged since the last commit: they are
/// double-buffered, so the model hears them when the toplevel commits.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PendingHints {
    first_mouse: Option<Vec<LogicalRect>>,
    frame: Option<FrameHint>,
}

impl PendingHints {
    /// Nothing staged.
    pub fn none() -> PendingHints {
        PendingHints::default()
    }

    /// With `first_mouse` staged (a later request replaces an earlier one).
    pub fn with_first_mouse(self, first_mouse: Vec<LogicalRect>) -> PendingHints {
        PendingHints {
            first_mouse: Some(first_mouse),
            ..self
        }
    }

    /// With `frame` staged (a later request replaces an earlier one).
    pub fn with_frame(self, frame: FrameHint) -> PendingHints {
        PendingHints {
            frame: Some(frame),
            ..self
        }
    }

    /// What the commit applies, in a fixed order (first mouse, then frame).
    pub fn commit(self) -> Vec<WindowHintChange> {
        self.first_mouse
            .map(WindowHintChange::FirstMouse)
            .into_iter()
            .chain(self.frame.map(WindowHintChange::Frame))
            .collect()
    }
}

/// The first-mouse regions a `wl_region` describes: its added rectangles (surface-local). A
/// subtracted rectangle is not representable as a list of rectangles, so a region with any
/// subtraction keeps only the added rectangles no subtraction fully covers; an empty region
/// is none.
pub fn first_mouse_of(region: &RegionAttributes) -> Vec<LogicalRect> {
    let read: Vec<_> = region.rects.iter().take(MAX_FIRST_MOUSE_RECTS).collect();
    let subtracted: Vec<_> = read
        .iter()
        .filter(|(kind, _)| matches!(kind, RectangleKind::Subtract))
        .map(|(_, rect)| *rect)
        .collect();
    let kept: Vec<_> = read
        .iter()
        .filter(|(kind, rect)| {
            matches!(kind, RectangleKind::Add)
                && !subtracted.iter().any(|cut| cut.contains_rect(*rect))
        })
        .map(|(_, rect)| rect_of(*rect))
        .filter(|rect| rect.size.w > 0 && rect.size.h > 0)
        .collect();
    kept
}

/// The frame a `set_frame` asks for.
pub fn frame_of(mode: FrameMode, corner_radius: u32) -> FrameHint {
    FrameHint {
        mode,
        corner_radius,
    }
}

/// A role a client asked for that it may not have (`bad_role`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BadRole;

/// The role `set_role` may set: `document`, `utility` and `pip`; `dialog` and `shell` are known
/// from other protocols (xdg-dialog, quire_shell_v1) and a client cannot ask for them.
pub fn role_request(role: WindowRole) -> Result<WindowRole, BadRole> {
    match role {
        WindowRole::Document | WindowRole::Utility | WindowRole::Pip => Ok(role),
        WindowRole::Dialog | WindowRole::Shell => Err(BadRole),
    }
}

/// The accessibility root a `set_a11y_root` names: an empty name clears it, and one too long to
/// be a D-Bus name or path is not passed on.
pub fn a11y_of(bus_name: &str, object_path: &str) -> Result<Option<A11yRoot>, A11yTooLong> {
    if bus_name.is_empty() {
        return Ok(None);
    }
    if bus_name.len() > A11Y_MAX_LEN || object_path.len() > A11Y_MAX_LEN {
        return Err(A11yTooLong);
    }
    Ok(Some(A11yRoot {
        bus_name: bus_name.to_owned(),
        object_path: object_path.to_owned(),
    }))
}

/// An accessibility root whose name or path is too long to be a D-Bus one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct A11yTooLong;

/// What a proven `request_placement` does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PlacementPolicy {
    /// It is stored and not acted on (P0: windows only float).
    StoreOnly,
    /// The model places the window (P2, the policy in force).
    Act,
}

/// The policy this phase runs: from P2 a proven placement request is acted on (the model's
/// `place`), and the object is told the window's new mark when its state changes.
pub const PLACEMENT_POLICY: PlacementPolicy = PlacementPolicy::Act;

/// The events a proven placement request is under `policy`.
pub fn placement_events(
    policy: PlacementPolicy,
    window: WindowId,
    target: PlaceTarget,
    serial: InputSerial,
) -> Vec<Event> {
    match policy {
        PlacementPolicy::StoreOnly => Vec::new(),
        PlacementPolicy::Act => vec![Event::ToplevelRequest {
            window,
            request: ToplevelRequest::Place { target, serial },
        }],
    }
}

/// What the model goes back to when the extension object is destroyed (the compositor forgets
/// every value set through it): no first-mouse regions, a client frame with square corners that says nothing, a
/// saved document, no accessibility root, and the role the window had without the object.
pub fn forgotten(base_role: WindowRole) -> Vec<WindowHintChange> {
    vec![
        WindowHintChange::FirstMouse(Vec::new()),
        WindowHintChange::FrameForgotten,
        WindowHintChange::Role(base_role),
        WindowHintChange::A11y(None),
        WindowHintChange::Edited(Edited::Saved),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use casement_core::geometry::{LogicalPoint, LogicalRect, LogicalSize};
    use smithay::utils::Rectangle;

    /// A kind and a rectangle as x, y, width, height.
    type Piece = (RectangleKind, (i32, i32, i32, i32));

    fn region(rects: &[Piece]) -> RegionAttributes {
        RegionAttributes {
            rects: rects
                .iter()
                .map(|(kind, (x, y, w, h))| {
                    (*kind, Rectangle::new((*x, *y).into(), (*w, *h).into()))
                })
                .collect(),
        }
    }

    fn rect(x: i32, y: i32, w: u32, h: u32) -> LogicalRect {
        LogicalRect::new(LogicalPoint { x, y }, LogicalSize { w, h })
    }

    #[test]
    fn staged_hints_apply_in_a_fixed_order_and_the_last_request_wins() {
        let frame = frame_of(FrameMode::CompositorShadow, 10);
        let pending = PendingHints::none()
            .with_frame(frame_of(FrameMode::Client, 3))
            .with_first_mouse(vec![rect(0, 0, 10, 10)])
            .with_frame(frame);
        assert_eq!(
            pending.commit(),
            vec![
                WindowHintChange::FirstMouse(vec![rect(0, 0, 10, 10)]),
                WindowHintChange::Frame(frame),
            ]
        );
        assert_eq!(PendingHints::none().commit(), Vec::new());
    }

    #[test]
    fn regions_become_first_mouse_rectangles() {
        use RectangleKind::{Add, Subtract};
        let rows = [
            ("empty", region(&[]), Vec::new()),
            (
                "one add",
                region(&[(Add, (0, 0, 40, 20))]),
                vec![rect(0, 0, 40, 20)],
            ),
            (
                "two adds keep their order",
                region(&[(Add, (0, 0, 40, 20)), (Add, (50, 0, 10, 10))]),
                vec![rect(0, 0, 40, 20), rect(50, 0, 10, 10)],
            ),
            (
                "an add that a subtraction covers is dropped",
                region(&[(Add, (5, 5, 10, 10)), (Subtract, (0, 0, 100, 100))]),
                Vec::new(),
            ),
            (
                "an add a subtraction only clips is kept whole",
                region(&[(Add, (0, 0, 40, 20)), (Subtract, (30, 0, 50, 50))]),
                vec![rect(0, 0, 40, 20)],
            ),
            (
                "an empty rectangle is nothing",
                region(&[(Add, (0, 0, 0, 20))]),
                Vec::new(),
            ),
        ];
        for (case, attrs, want) in rows {
            assert_eq!(first_mouse_of(&attrs), want, "{case}");
        }
    }

    /// CASC-16: a region of far more rectangles than the cap is read up to the cap, so the
    /// subtraction test stays small and the hint a window carries stays small.
    #[test]
    fn a_huge_region_is_read_up_to_the_cap() {
        use RectangleKind::{Add, Subtract};
        let many: Vec<_> = (0..200_000_i32)
            .map(|n| (if n % 2 == 0 { Add } else { Subtract }, (n, 0, 10, 10)))
            .collect();
        let kept = first_mouse_of(&region(&many));
        assert!(kept.len() <= MAX_FIRST_MOUSE_RECTS, "{} kept", kept.len());
        let adds: Vec<_> = (0..200_000_i32).map(|n| (Add, (n, 0, 10, 10))).collect();
        let kept = first_mouse_of(&region(&adds));
        assert_eq!(kept.len(), MAX_FIRST_MOUSE_RECTS);
    }

    #[test]
    fn only_document_utility_and_pip_may_be_asked_for() {
        let rows = [
            (WindowRole::Document, Ok(WindowRole::Document)),
            (WindowRole::Utility, Ok(WindowRole::Utility)),
            (WindowRole::Pip, Ok(WindowRole::Pip)),
            (WindowRole::Dialog, Err(BadRole)),
            (WindowRole::Shell, Err(BadRole)),
        ];
        for (role, want) in rows {
            assert_eq!(role_request(role), want, "{role:?}");
        }
    }

    #[test]
    fn an_accessibility_root_is_cleared_set_or_refused() {
        let long = "x".repeat(A11Y_MAX_LEN + 1);
        assert_eq!(a11y_of("", ""), Ok(None));
        assert_eq!(a11y_of("", "/org/a11y/root"), Ok(None));
        assert_eq!(
            a11y_of(":1.42", "/org/a11y/atspi/accessible/root"),
            Ok(Some(A11yRoot {
                bus_name: ":1.42".to_owned(),
                object_path: "/org/a11y/atspi/accessible/root".to_owned()
            }))
        );
        assert_eq!(a11y_of(&long, "/p"), Err(A11yTooLong));
        assert_eq!(a11y_of(":1.1", &long), Err(A11yTooLong));
    }

    #[test]
    fn placement_is_acted_on_from_p2_and_stored_when_asked_to() {
        let (window, serial) = (WindowId(5), InputSerial(77));
        assert_eq!(PLACEMENT_POLICY, PlacementPolicy::Act);
        assert_eq!(
            placement_events(
                PlacementPolicy::StoreOnly,
                window,
                PlaceTarget::Left,
                serial
            ),
            Vec::new()
        );
        assert_eq!(
            placement_events(PlacementPolicy::Act, window, PlaceTarget::Left, serial),
            vec![Event::ToplevelRequest {
                window,
                request: ToplevelRequest::Place {
                    target: PlaceTarget::Left,
                    serial
                }
            }]
        );
    }

    #[test]
    fn destroying_the_object_forgets_all_five_hints() {
        let hints = forgotten(WindowRole::Dialog);
        assert_eq!(hints.len(), 5);
        assert!(hints.contains(&WindowHintChange::Role(WindowRole::Dialog)));
        assert!(hints.contains(&WindowHintChange::Edited(Edited::Saved)));
        assert!(hints.contains(&WindowHintChange::FirstMouse(Vec::new())));
    }
}
