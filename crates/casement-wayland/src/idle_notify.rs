//! `ext_idle_notifier_v1`: idle and resumed notifications. smithay's notifier arms calloop
//! timers on the wall clock, which the virtual clock cannot stop; casement times idleness with
//! the model's `TimerKey::IdleNotify` instead and uses smithay's state for the objects only
//! (FINDINGS, A1).

use crate::states::{GlobalFilter, WaylandFault};
use casement_core::time::IdleNotifierId;
use casement_core::{ClientKey, Event};
use smithay::reexports::calloop::LoopHandle;
use smithay::reexports::wayland_protocols::ext::idle_notify::v1::server::ext_idle_notifier_v1::ExtIdleNotifierV1;
use smithay::reexports::wayland_server::{DisplayHandle, GlobalDispatch};
use smithay::wayland::GlobalData;
use smithay::wayland::idle_notify::{IdleNotifierHandler, IdleNotifierState};
use std::time::Duration;

/// `ext_idle_notifier_v1`.
#[derive(Debug)]
pub struct IdleNotifyPart<D: IdleNotifierHandler + 'static> {
    /// smithay's state.
    pub state: IdleNotifierState<D>,
}

impl<D: IdleNotifierHandler + 'static> IdleNotifyPart<D> {
    /// Creates the global (version 2). Every connection sees it.
    pub fn new(
        display: &DisplayHandle,
        role_filter: GlobalFilter,
        event_loop: LoopHandle<'static, D>,
    ) -> IdleNotifyPart<D>
    where
        D: GlobalDispatch<ExtIdleNotifierV1, GlobalData>,
    {
        let _ = role_filter;
        IdleNotifyPart {
            state: IdleNotifierState::new(display, event_loop),
        }
    }
}

/// `client` asked to be told after `timeout` of idleness (notifier `id`).
pub fn on_notification(
    part: &mut IdleNotifyPart<impl IdleNotifierHandler>,
    client: ClientKey,
    id: IdleNotifierId,
    timeout: Duration,
) -> Vec<Event> {
    let _ = (part, client, id, timeout);
    todo!("P0-H8: idle_notify::on_notification")
}

/// Sends `idled` to notifier `id`.
pub fn apply_idle(
    part: &mut IdleNotifyPart<impl IdleNotifierHandler>,
    id: IdleNotifierId,
) -> Result<(), WaylandFault> {
    let _ = (part, id);
    todo!("P0-H8: idle_notify::apply_idle")
}

/// Sends `resumed` to notifier `id`.
pub fn apply_resumed(
    part: &mut IdleNotifyPart<impl IdleNotifierHandler>,
    id: IdleNotifierId,
) -> Result<(), WaylandFault> {
    let _ = (part, id);
    todo!("P0-H8: idle_notify::apply_resumed")
}
