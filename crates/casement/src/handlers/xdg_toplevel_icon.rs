//! `xdg_toplevel_icon_manager_v1` (P0-H2).

use crate::state::State;
use smithay::wayland::xdg_toplevel_icon::XdgToplevelIconHandler;

impl XdgToplevelIconHandler for State {}
