//! `xdg_wm_dialog_v1`: dialogs and modal dialogs.

use crate::states::GlobalFilter;
use casement_core::Event;
use casement_protocols::vocab::Modality;
use smithay::reexports::wayland_protocols::xdg::dialog::v1::server::xdg_wm_dialog_v1::XdgWmDialogV1;
use smithay::reexports::wayland_server::{Dispatch, DisplayHandle, GlobalDispatch};
use smithay::wayland::GlobalData;
use smithay::wayland::shell::xdg::ToplevelSurface;
use smithay::wayland::shell::xdg::dialog::XdgDialogState;

/// `xdg_wm_dialog_v1`.
#[derive(Debug)]
pub struct XdgDialogPart {
    /// smithay's state.
    pub state: XdgDialogState,
}

impl XdgDialogPart {
    /// Creates the global (version 1). Every connection sees it.
    pub fn new<D>(display: &DisplayHandle, role_filter: GlobalFilter) -> XdgDialogPart
    where
        D: GlobalDispatch<XdgWmDialogV1, GlobalData>
            + Dispatch<XdgWmDialogV1, GlobalData>
            + 'static,
    {
        let _ = role_filter;
        XdgDialogPart {
            state: XdgDialogState::new::<D>(display),
        }
    }
}

/// `toplevel` became (or stopped being) a dialog or a modal one.
///
/// smithay keeps the hint on the toplevel's role, and the model reads it when the window maps
/// (`xdg_shell::modality_of`, in `MappedToplevel.modality`), so a hint set before the first
/// buffer needs no event. The model has no event for a change after the map (an interface ask:
/// `Event::ToplevelModality`), so this reports nothing today.
pub fn on_modal(
    part: &mut XdgDialogPart,
    toplevel: &ToplevelSurface,
    modality: Modality,
) -> Vec<Event> {
    let _ = (part, toplevel, modality);
    Vec::new()
}
