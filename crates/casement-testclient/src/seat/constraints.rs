//! zwp_pointer_constraints_v1: lock and confine. Filled by P0-H4.
//!
//! The constraint objects' events (`locked`, `unlocked`, `confined`, `unconfined`) reach the
//! transcript through the connection's tap.

use crate::conn::{ClientError, ClientState, TestClient};
use crate::seat::pointer::Pointer;
use wayland_client::protocol::wl_surface::WlSurface;
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle};
use wayland_protocols::wp::pointer_constraints::zv1::client::zwp_confined_pointer_v1::{
    self, ZwpConfinedPointerV1,
};
use wayland_protocols::wp::pointer_constraints::zv1::client::zwp_locked_pointer_v1::{
    self, ZwpLockedPointerV1,
};
use wayland_protocols::wp::pointer_constraints::zv1::client::zwp_pointer_constraints_v1::{
    Lifetime, ZwpPointerConstraintsV1,
};

/// The user data of the objects here: nothing.
#[derive(Debug, Clone, Copy, Default)]
pub struct ConstraintPlain;

/// A lock or a confinement the client asked for.
#[derive(Debug)]
pub enum Constraint {
    /// The pointer is locked in place.
    Locked(ZwpLockedPointerV1),
    /// The pointer is confined to the surface.
    Confined(ZwpConfinedPointerV1),
}

impl Constraint {
    /// Asks to lock `pointer` while it is over `surface`, for as long as the object lives.
    pub fn lock(
        client: &TestClient,
        surface: &WlSurface,
        pointer: &Pointer,
    ) -> Result<Constraint, ClientError> {
        let manager: ZwpPointerConstraintsV1 = client.bind(1..=1, ConstraintPlain)?;
        let locked = manager.lock_pointer(
            surface,
            pointer.pointer(),
            None,
            Lifetime::Persistent,
            &client.qh(),
            ConstraintPlain,
        );
        let _ = client.connection().flush();
        Ok(Constraint::Locked(locked))
    }

    /// Asks to confine `pointer` to `surface`.
    pub fn confine(
        client: &TestClient,
        surface: &WlSurface,
        pointer: &Pointer,
    ) -> Result<Constraint, ClientError> {
        let manager: ZwpPointerConstraintsV1 = client.bind(1..=1, ConstraintPlain)?;
        let confined = manager.confine_pointer(
            surface,
            pointer.pointer(),
            None,
            Lifetime::Persistent,
            &client.qh(),
            ConstraintPlain,
        );
        let _ = client.connection().flush();
        Ok(Constraint::Confined(confined))
    }
}

impl Dispatch<ZwpPointerConstraintsV1, ConstraintPlain> for ClientState {
    fn event(
        _: &mut Self,
        _: &ZwpPointerConstraintsV1,
        _: <ZwpPointerConstraintsV1 as Proxy>::Event,
        _: &ConstraintPlain,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ZwpLockedPointerV1, ConstraintPlain> for ClientState {
    fn event(
        _: &mut Self,
        _: &ZwpLockedPointerV1,
        _: zwp_locked_pointer_v1::Event,
        _: &ConstraintPlain,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ZwpConfinedPointerV1, ConstraintPlain> for ClientState {
    fn event(
        _: &mut Self,
        _: &ZwpConfinedPointerV1,
        _: zwp_confined_pointer_v1::Event,
        _: &ConstraintPlain,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}
