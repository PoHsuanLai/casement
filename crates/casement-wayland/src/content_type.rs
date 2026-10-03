//! `wp_content_type_v1`: what a surface shows (photo, video, game), read from its cached
//! state on commit; a hint for scheduling and, from P5, VRR.

use crate::states::GlobalFilter;
use casement_core::Event;
use smithay::reexports::wayland_protocols::wp::content_type::v1::server::wp_content_type_manager_v1::WpContentTypeManagerV1;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::reexports::wayland_server::{DisplayHandle, GlobalDispatch};
use smithay::wayland::GlobalData;
use smithay::wayland::content_type::ContentTypeState;

/// `wp_content_type_manager_v1`.
#[derive(Debug)]
pub struct ContentTypePart {
    /// smithay's state.
    pub state: ContentTypeState,
}

impl ContentTypePart {
    /// Creates the global (version 1). Every connection sees it.
    pub fn new<D>(display: &DisplayHandle, role_filter: GlobalFilter) -> ContentTypePart
    where
        D: GlobalDispatch<WpContentTypeManagerV1, GlobalData> + 'static,
    {
        let _ = role_filter;
        ContentTypePart {
            state: ContentTypeState::new::<D>(display),
        }
    }
}

/// What a surface says it shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ContentKind {
    /// No hint.
    None,
    /// Still images.
    Photo,
    /// Video.
    Video,
    /// A game.
    Game,
}

/// `surface` committed `kind`.
pub fn on_content_type(
    part: &mut ContentTypePart,
    surface: &WlSurface,
    kind: ContentKind,
) -> Vec<Event> {
    let _ = (part, surface, kind);
    todo!("P0-H1: content_type::on_content_type")
}
