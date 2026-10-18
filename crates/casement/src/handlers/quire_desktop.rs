//! `quire_desktop_manager_v1` (P2-Z; filled by P2-Q1): the wire glue lives in
//! `casement_wayland::quire::desktop`; this implements what it asks of the state.

use crate::state::State;
use casement_wayland::quire::desktop::{DesktopParts, DesktopView, QuireDesktopHandler};

impl QuireDesktopHandler for State {
    fn quire_desktop_parts(&mut self) -> DesktopParts<'_> {
        DesktopParts {
            part: &mut self.wl.quire_desktop,
            view: DesktopView {
                model: &self.core.model,
                outputs: &self.wl.output.outputs,
                rects: self
                    .core
                    .arrangement
                    .windows
                    .iter()
                    .map(|target| (target.window, target.rect))
                    .collect(),
            },
        }
    }
}
