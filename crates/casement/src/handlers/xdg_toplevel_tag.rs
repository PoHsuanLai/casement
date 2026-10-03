//! `xdg_toplevel_tag_manager_v1` (P0-H2).

use crate::state::State;
use smithay::wayland::xdg_toplevel_tag::XdgToplevelTagHandler;

impl XdgToplevelTagHandler for State {}
