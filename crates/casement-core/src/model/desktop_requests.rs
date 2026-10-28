//! Requests from the shell's desktop manager (`quire_desktop_manager_v1`; P2-Z): the same
//! transitions a window's own requests, a window-list client, a workspace client or a key
//! binding take, reached by name. Everything here is the person's: the wire layer refuses
//! management on an agent connection before an event is made.

use super::Model;
use super::moved::Moved;
use crate::app::AppKey;
use crate::desktop::{AppAsk, DesktopRequest, SpaceAsk, WindowAsk};
use crate::effect::{CoreNote, Effect};
use crate::ids::WindowId;
use crate::space::WorkspaceRequest;
use crate::window::WlrToplevelRequest;
use casement_protocols::vocab::{ActorKind, Toggle};
use porter_core::SpaceId;

impl Model {
    /// The shell asked something of the desktop.
    pub(super) fn desktop_request(self, request: DesktopRequest) -> Moved {
        match request {
            DesktopRequest::App { app, ask } => self.app_ask(app, ask),
            DesktopRequest::Window { window, ask } => self.window_ask(window, ask),
            DesktopRequest::Space { space, ask } => self.space_ask(space, ask),
            DesktopRequest::CreateSpace { output } => {
                self.workspace_request(WorkspaceRequest::Create {
                    output,
                    name: String::new(),
                })
            }
        }
    }

    /// An app. One the model does not know is left alone: the shell's view was a moment old.
    fn app_ask(self, app: AppKey, ask: AppAsk) -> Moved {
        if !self.apps.contains_key(&app) {
            return Moved::of(self);
        }
        match ask {
            AppAsk::Activate => self.activation(app, ActorKind::User),
            AppAsk::Hide => self.hide_app(&app),
            AppAsk::Unhide => self.unhide_app(&app),
            AppAsk::HideOthers => self.hide_others(Some(app)),
            AppAsk::Quit => self.quit_app(Some(app)),
            AppAsk::ForceQuit => self.force_quit_app(&app),
            AppAsk::Reopen => Moved::of(self).with(Effect::Note(CoreNote::ReopenDeferred)),
        }
    }

    /// A window.
    fn window_ask(self, window: WindowId, ask: WindowAsk) -> Moved {
        let Some(held) = self.windows.get(&window).cloned() else {
            return Moved::of(self).with(Effect::Note(CoreNote::UnknownWindow(window)));
        };
        match ask {
            WindowAsk::Activate => self.wlr_request(window, WlrToplevelRequest::Activate),
            WindowAsk::Close => self.wlr_request(window, WlrToplevelRequest::Close),
            WindowAsk::Minimise => {
                self.wlr_request(window, WlrToplevelRequest::Minimise(Toggle::On))
            }
            WindowAsk::Unminimise => {
                self.wlr_request(window, WlrToplevelRequest::Minimise(Toggle::Off))
            }
            WindowAsk::FullScreen(Toggle::On) => self
                .unminimised(window)
                .then(|model| model.full_screen_by_hand(window, Toggle::On)),
            WindowAsk::FullScreen(Toggle::Off) => self.full_screen_by_hand(window, Toggle::Off),
            WindowAsk::Place(target) => self.place(&held, target),
            WindowAsk::ToSpace(space) => self.window_to_space(window, &space),
            WindowAsk::ToNewSpace(output) => self.window_to_new_space(window, &output),
            WindowAsk::Assign(assign) => self.window_assigned(window, assign),
        }
    }

    /// A Space.
    fn space_ask(self, space: SpaceId, ask: SpaceAsk) -> Moved {
        match ask {
            SpaceAsk::Activate => self.workspace_request(WorkspaceRequest::Activate(space)),
            SpaceAsk::Remove => self.workspace_request(WorkspaceRequest::Remove(space)),
            SpaceAsk::MoveTo(index) => self.space_moved(&space, index),
        }
    }
}
