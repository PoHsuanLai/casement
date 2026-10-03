//! `zwp_pointer_constraints_v1` (P0-H4).

use crate::state::State;
use smithay::wayland::pointer_constraints::PointerConstraintsHandler;

impl PointerConstraintsHandler for State {}
