//! `ext_idle_notifier_v1` (P0-H8). The protocol is served by `casement_wayland::idle_notify`
//! (smithay's notifier times on the wall clock, A36); this file gives it the root's part, the
//! session clock and `exec`, and keeps smithay's handler trait satisfied for the aggregate.

use crate::exec::exec;
use crate::state::State;
use casement_core::Effect;
use casement_core::time::Mono;
use casement_wayland::idle_notify::{IdleNotifyHandler, IdleNotifyPart};
use smithay::wayland::idle_notify::{IdleNotifierHandler, IdleNotifierState};

impl IdleNotifierHandler for State {
    fn idle_notifier_state(&mut self) -> &mut IdleNotifierState<State> {
        &mut self.wl.idle_notify.state
    }
}

impl IdleNotifyHandler for State {
    fn idle_notify_part(&mut self) -> &mut IdleNotifyPart<State> {
        &mut self.wl.idle_notify
    }

    fn session_now(&self) -> Mono {
        self.clock.now()
    }

    fn idle_effects(&mut self, effects: Vec<Effect>) {
        exec(self, effects);
    }
}
