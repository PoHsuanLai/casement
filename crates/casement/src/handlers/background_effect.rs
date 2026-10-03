//! `ext_background_effect_manager_v1` (P0-H8): `set_blur_region` and `unset_blur_region` reach
//! `casement_wayland::background_effect::on_blur_region` once the lane overrides them.

use crate::state::State;
use smithay::wayland::background_effect::ExtBackgroundEffectHandler;

impl ExtBackgroundEffectHandler for State {}
