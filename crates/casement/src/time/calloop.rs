//! Arming timers on the event loop: a real calloop `Timer` on the wall clock, the virtual
//! clock's book otherwise.

use crate::state::State;
use casement_core::time::Due;
use smithay::reexports::calloop::LoopHandle;

/// Arms `due` (`Effect::ArmTimer`): when it fires, `Event::TimerFired(due.key)` is fed to the
/// core loop.
pub fn arm(handle: &LoopHandle<'static, State>, due: Due) {
    let _ = (handle, due);
    todo!("P0-B-loop: time::arm")
}
