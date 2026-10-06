//! `zwp_pointer_constraints_v1`: locked and confined pointers (games, remote desktops).

use crate::states::GlobalFilter;
use casement_core::Event;
use smithay::input::SeatHandler;
use smithay::input::pointer::PointerHandle;
use smithay::reexports::wayland_protocols::wp::pointer_constraints::zv1::server::{
    zwp_confined_pointer_v1::ZwpConfinedPointerV1, zwp_locked_pointer_v1::ZwpLockedPointerV1,
    zwp_pointer_constraints_v1::ZwpPointerConstraintsV1,
};
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::reexports::wayland_server::{Dispatch, DisplayHandle, GlobalDispatch};
use smithay::wayland::GlobalData;
use smithay::wayland::pointer_constraints::{
    PointerConstraintUserData, PointerConstraintsHandler, PointerConstraintsState,
    with_pointer_constraint,
};

/// `zwp_pointer_constraints_v1`.
#[derive(Debug)]
pub struct PointerConstraintsPart {
    /// smithay's state.
    pub state: PointerConstraintsState,
}

impl PointerConstraintsPart {
    /// Creates the global (version 1). Every connection sees it.
    pub fn new<D>(display: &DisplayHandle, role_filter: GlobalFilter) -> PointerConstraintsPart
    where
        D: GlobalDispatch<ZwpPointerConstraintsV1, GlobalData>
            + Dispatch<ZwpPointerConstraintsV1, GlobalData>
            + Dispatch<ZwpConfinedPointerV1, PointerConstraintUserData<D>>
            + Dispatch<ZwpLockedPointerV1, PointerConstraintUserData<D>>
            + SeatHandler
            + 'static,
    {
        let _ = role_filter;
        PointerConstraintsPart {
            state: PointerConstraintsState::new::<D>(display),
        }
    }
}

/// `surface` asked to lock or confine `pointer`; it activates when the surface has pointer
/// focus (otherwise it waits: `seat::apply_pointer` activates it when the pointer enters).
/// Nothing is reported to the model.
pub fn on_constraint<D>(
    part: &mut PointerConstraintsPart,
    surface: &WlSurface,
    pointer: &PointerHandle<D>,
) -> Vec<Event>
where
    D: PointerConstraintsHandler + 'static,
    D::PointerFocus: Into<Option<WlSurface>>,
{
    let _ = part;
    let focus: Option<WlSurface> = pointer.current_focus().and_then(Into::into);
    if focus.as_ref() == Some(surface) {
        with_pointer_constraint::<D, _, _>(surface, pointer, |constraint| {
            if let Some(constraint) = constraint {
                constraint.activate();
            }
        });
    }
    Vec::new()
}

/// The keyboard moved from `left` to `entered` (a window was activated): a constraint holds only
/// while its surface has the keyboard as well as the pointer, as the compositors wlcs was
/// written against decide ("when the compositor deems implementation-specific constraints are
/// satisfied", pointer-constraints-v1). The surface that lost it is released (a persistent
/// constraint waits, a one-shot one is over); the one that gained it, when the pointer is on it,
/// holds again.
pub fn on_keyboard_focus<D>(
    pointer: &PointerHandle<D>,
    left: Option<&WlSurface>,
    entered: Option<&WlSurface>,
) where
    D: PointerConstraintsHandler + 'static,
    D::PointerFocus: Into<Option<WlSurface>>,
{
    if let Some(left) = left.filter(|left| Some(*left) != entered) {
        with_pointer_constraint::<D, _, _>(left, pointer, |constraint| {
            if let Some(constraint) = constraint {
                constraint.deactivate();
            }
        });
    }
    let under: Option<WlSurface> = pointer.current_focus().and_then(Into::into);
    if let Some(entered) = entered.filter(|entered| under.as_ref() == Some(*entered)) {
        with_pointer_constraint::<D, _, _>(entered, pointer, |constraint| {
            if let Some(constraint) = constraint {
                constraint.activate();
            }
        });
    }
}
