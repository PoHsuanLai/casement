//! Requests from window-list clients (`zwlr_foreign_toplevel_handle_v1`).

use super::Model;
use super::moved::Moved;
use crate::cause::MinimiseTarget;
use crate::effect::{CoreNote, Effect};
use crate::event::ToplevelRequest;
use crate::focus::TokenCheck;
use crate::geometry::LogicalRect;
use crate::ids::WindowId;
use crate::window::{WindowState, WlrToplevelRequest};
use casement_protocols::vocab::Toggle;

impl Model {
    /// A window-list client asked something of `window`: the same transitions the window's own
    /// requests take (`request`), plus the minimise rectangle, which is stored for the genie.
    ///
    /// A dock or switcher is the person's own hand, so its activation counts as a valid one;
    /// closing asks the window to close and leaves the answer to the client.
    pub(super) fn wlr_request(self, window: WindowId, request: WlrToplevelRequest) -> Moved {
        if !self.windows.contains_key(&window) {
            return Moved::of(self).with(Effect::Note(CoreNote::UnknownWindow(window)));
        }
        match request {
            WlrToplevelRequest::Maximise(toggle) => {
                self.request(window, ToplevelRequest::Maximise(toggle))
            }
            WlrToplevelRequest::Minimise(Toggle::On) => {
                self.request(window, ToplevelRequest::Minimise)
            }
            WlrToplevelRequest::Minimise(Toggle::Off) => self.unminimised(window),
            WlrToplevelRequest::FullScreen { toggle, output } => {
                self.request(window, ToplevelRequest::FullScreen { toggle, output })
            }
            WlrToplevelRequest::Activate => {
                self.request(window, ToplevelRequest::Activate(TokenCheck::Valid))
            }
            WlrToplevelRequest::Close => Moved::of(self).with(Effect::SendClose { window }),
            WlrToplevelRequest::MinimiseRect(rect) => {
                Moved::of(self.with_minimise_rect(window, rect))
            }
        }
    }

    /// A minimised window comes back from the dock (and is activated, as a click on its tile
    /// does); any other window is left alone.
    fn unminimised(self, window: WindowId) -> Moved {
        let minimised = self
            .windows
            .get(&window)
            .is_some_and(|held| matches!(held.state, WindowState::Minimised { .. }));
        match minimised {
            true => self.request(window, ToplevelRequest::Activate(TokenCheck::Valid)),
            false => Moved::of(self),
        }
    }

    /// `rect` (global logical coordinates) is where `window` minimises to, on the output it
    /// lies on (else the window's own). An empty rectangle forgets the one set before.
    pub(super) fn with_minimise_rect(self, window: WindowId, rect: LogicalRect) -> Model {
        let mut model = self;
        if rect.size.w == 0 || rect.size.h == 0 {
            model.minimise_rects.remove(&window);
            return model;
        }
        let lying_on = model
            .outputs
            .0
            .iter()
            .find(|(_, state)| state.config.logical_rect().contains(rect.origin))
            .map(|(key, _)| key.clone());
        let own = model
            .windows
            .get(&window)
            .and_then(|held| model.output_of(held));
        if let Some(output) = lying_on.or(own) {
            model
                .minimise_rects
                .insert(window, MinimiseTarget { output, rect });
        }
        model
    }

    /// Where `window` minimises to, when its client set a rectangle on an output still plugged.
    pub(super) fn set_minimise_target(&self, window: WindowId) -> Option<MinimiseTarget> {
        self.minimise_rects
            .get(&window)
            .filter(|target| self.outputs.0.contains_key(&target.output))
            .cloned()
    }
}
