//! `wl_output` (P0-H1): globals are created and withdrawn by exec as connectors come and go.

use crate::state::State;
use smithay::wayland::output::OutputHandler;

impl OutputHandler for State {}
