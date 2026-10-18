//! `wp_color_manager_v1`: `casement_wayland::color_management` serves the protocol; this file
//! gives it the root's part and the output lookup. Nothing reaches the model: the tags are read
//! by the renderer (`color_management::surface_tag`) and the gamut table is filled where the
//! EDID is read.

use crate::state::State;
use casement_wayland::color_management::{ColorHandler, ColorPart};

impl ColorHandler for State {
    fn color_part(&mut self) -> &mut ColorPart {
        &mut self.wl.color_management
    }
}
