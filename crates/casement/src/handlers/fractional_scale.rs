//! `wp_fractional_scale_manager_v1` (P0-H1): `new_fractional_scale` sends the preferred scale
//! before the first configure (the H1 lane overrides it).

use crate::state::State;
use smithay::wayland::fractional_scale::FractionalScaleHandler;

impl FractionalScaleHandler for State {}
