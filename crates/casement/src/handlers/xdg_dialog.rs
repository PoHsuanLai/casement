//! `xdg_wm_dialog_v1` (P0-H2): a dialog or modal hint reaches the model when the window maps
//! (`xdg_shell::modality_of`); a change after that goes through `xdg_dialog::on_modal`.

use crate::core_loop;
use crate::state::State;
use casement_wayland::xdg_dialog::on_modal;
use casement_wayland::xdg_shell::modality_of;
use smithay::wayland::shell::xdg::ToplevelSurface;
use smithay::wayland::shell::xdg::dialog::{ToplevelDialogHint, XdgDialogHandler};

impl XdgDialogHandler for State {
    fn dialog_hint_changed(&mut self, toplevel: ToplevelSurface, hint: ToplevelDialogHint) {
        let events = on_modal(&mut self.wl.xdg_dialog, &toplevel, modality_of(hint));
        core_loop::feed(self, events);
    }
}
