//! `xdg_wm_dialog_v1` (P0-H2): `dialog_hint_changed` reaches `casement_wayland::xdg_dialog::on_modal`
//! once the lane overrides it.

use crate::state::State;
use smithay::wayland::shell::xdg::dialog::XdgDialogHandler;

impl XdgDialogHandler for State {}
