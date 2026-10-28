//! Requests from the shell (`quire_shell_v1`; P2-Z): which layer surface is the bar and the
//! dock, and where windows minimise to.

use super::Model;
use super::moved::Moved;
use crate::cause::Cause;
use crate::effect::Effect;
use crate::geometry::{LogicalPoint, LogicalRect, LogicalSize};
use crate::ids::{LayerKey, OutputKey, WindowId};
use crate::layer::ShellRoleTag;
use crate::reveal::FullScreenBar;
use crate::shell::{ShellNote, ShellRequest};
use casement_protocols::vocab::{Motion, ShellRole};

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
            ShellRequest::WindowMenuPick { window, entry } => self.window_menu_pick(window, entry),
            ShellRequest::SplitPick { window } => self.split_pick(window),
            ShellRequest::SplitCancel => self.split_cancel(),
            ShellRequest::SplitDivide { window, share } => self.split_divide(window, share),
            ShellRequest::Overview { mode, toggle } => self.overview_asked(mode, toggle),
            ShellRequest::FullScreenBar(mode) => self.with_full_screen_bar(mode),
            ShellRequest::Motion(motion) => Moved::of(Model {
                reduced_motion: matches!(motion, Motion::Reduced),
                ..self
            }),
        }
    }

    /// The bar does `mode` in full screen from now. A change lays the scene out again (the
    /// full-screen windows are sized to it) and asks every output for a frame.
    fn with_full_screen_bar(self, mode: FullScreenBar) -> Moved {
        match self.full_screen_bar == mode {
            true => Moved::of(self),
            false => {
                let outputs: Vec<OutputKey> = self.outputs.0.keys().cloned().collect();
                Model {
                    full_screen_bar: mode,
                    ..self
                }
                .redrawn(Cause::Configure, outputs)
            }
        }
    }

    /// `layer` holds `role` from now. A surface never mapped, or a role another surface holds
    /// (on its output, for a bar or a dock), changes nothing (the wire layer answered the client
    /// with an error).
    fn with_shell_role(self, layer: LayerKey, role: ShellRole) -> Moved {
        let free =
            |spec: &crate::layer::LayerSpec| !self.layers.role_taken(role, layer, &spec.output);
        let Some(spec) = self.layers.get(&layer).cloned().filter(free) else {
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
    /// (the shell's target wins whichever asked last); with no dock it is the only source. The
    /// dock is the one on the window's output, else any.
    pub(super) fn with_window_list_rect(self, window: WindowId, rect: LogicalRect) -> Model {
        match self.dock_for(window) {
            Some(_) => self,
            None => self.with_minimise_rect(window, rect),
        }
    }

    /// The dock `window` minimises into: the one on the window's output, else any dock.
    fn dock_for(&self, window: WindowId) -> Option<LayerKey> {
        let home = self
            .windows
            .get(&window)
            .and_then(|window| window.home.clone());
        match home {
            Some(output) => self.layers.role_holder_near(ShellRole::Dock, &output),
            None => self.layers.role_holder(ShellRole::Dock),
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
