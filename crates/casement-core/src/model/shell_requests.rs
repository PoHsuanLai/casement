//! Requests from the shell (`quire_shell_v1`; P2-Z): which layer surface is the bar and the
//! dock, and where windows minimise to.

use super::Model;
use super::moved::Moved;
use crate::effect::Effect;
use crate::geometry::{LogicalPoint, LogicalRect, LogicalSize};
use crate::ids::{LayerKey, WindowId};
use crate::layer::ShellRoleTag;
use crate::shell::{ShellNote, ShellRequest};
use casement_protocols::vocab::ShellRole;

impl Model {
    /// The shell said something.
    pub(super) fn shell_request(self, request: ShellRequest) -> Moved {
        match request {
            ShellRequest::Role { layer, role } => self.with_shell_role(layer, role),
            ShellRequest::MinimiseTarget {
                dock,
                window,
                local,
            } => self.with_dock_target(dock, window, local),
            ShellRequest::ClearMinimiseTarget { window } => {
                let none = LogicalRect::new(LogicalPoint { x: 0, y: 0 }, LogicalSize::ZERO);
                Moved::of(self.with_minimise_rect(window, none))
            }
        }
    }

    /// `layer` holds `role` from now. A surface never mapped, or a role another surface holds,
    /// changes nothing (the wire layer answered the client with an error).
    fn with_shell_role(self, layer: LayerKey, role: ShellRole) -> Moved {
        let taken = self
            .layers
            .role_holder(role)
            .is_some_and(|held| held != layer);
        let Some(spec) = self.layers.get(&layer).cloned().filter(|_| !taken) else {
            return Moved::of(self);
        };
        let spec = crate::layer::LayerSpec {
            role: ShellRoleTag::Assigned(role),
            ..spec
        };
        let model = Model {
            layers: self.layers.with_layer(layer, spec),
            ..self
        };
        Moved::of(model).with(Effect::Shell(ShellNote::Role(layer, role)))
    }

    /// `window` minimises into `local`, a rectangle in the coordinates of the `dock` surface.
    fn with_dock_target(self, dock: LayerKey, window: WindowId, local: LogicalRect) -> Moved {
        let Some(origin) = self.layer_origin(dock) else {
            return Moved::of(self);
        };
        let global = LogicalRect::new(
            LogicalPoint {
                x: origin.x.saturating_add(local.origin.x),
                y: origin.y.saturating_add(local.origin.y),
            },
            local.size,
        );
        let model = self.with_minimise_rect(window, global);
        let stored = model.minimise_target(window).map(|target| target.rect);
        Moved::of(model)
            .with_all(stored.map(|rect| Effect::Shell(ShellNote::MinimiseTarget(window, rect))))
    }

    /// A window-list client's `set_rectangle` for `window`. While a dock holds a role the dock's
    /// tiles are where windows minimise to, so the window-list client's rectangle is ignored
    /// (the shell's target wins whichever asked last); with no dock it is the only source.
    pub(super) fn with_window_list_rect(self, window: WindowId, rect: LogicalRect) -> Model {
        match self.layers.role_holder(ShellRole::Dock) {
            Some(_) => self,
            None => self.with_minimise_rect(window, rect),
        }
    }

    /// Where `layer` sits: its pin, else the corner its anchors and configured size give it.
    fn layer_origin(&self, layer: LayerKey) -> Option<LogicalPoint> {
        if let Some(pin) = self.layers.pin(&layer) {
            return Some(pin);
        }
        let spec = self.layers.get(&layer)?;
        let full = self.output_rect(&spec.output)?;
        let arranged = self.layers.arrange(&spec.output, full);
        let size = arranged.configure_size(&layer)?;
        arranged.place(&layer, size).map(|rect| rect.origin)
    }
}
