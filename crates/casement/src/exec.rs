//! The one place effects happen: each `Effect` reaches the protocol module, the backend, the
//! renderer or the clock that carries it out. A failure is logged and degrades; it never
//! panics the loop (CONVENTIONS addendum 2).

use crate::state::State;
use casement_core::Effect;

/// Carries out `effects` in order.
pub fn exec(state: &mut State, effects: Vec<Effect>) {
    let _ = (state, effects);
    todo!("P0-B-loop: exec")
}
