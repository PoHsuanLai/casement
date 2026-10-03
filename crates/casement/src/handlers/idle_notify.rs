//! `ext_idle_notifier_v1` (P0-H8).

use crate::state::State;
use smithay::wayland::idle_notify::{IdleNotifierHandler, IdleNotifierState};

impl IdleNotifierHandler for State {
    fn idle_notifier_state(&mut self) -> &mut IdleNotifierState<State> {
        &mut self.wl.idle_notify.state
    }
}
