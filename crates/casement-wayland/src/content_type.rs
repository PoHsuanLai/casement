//! `wp_content_type_v1`: what a surface shows (photo, video, game), read from its cached
//! state on commit; a hint for scheduling and, from P5, VRR.

use crate::states::GlobalFilter;
use casement_core::Event;
use smithay::reexports::wayland_protocols::wp::content_type::v1::server::wp_content_type_manager_v1::WpContentTypeManagerV1;
use smithay::reexports::wayland_protocols::wp::content_type::v1::server::wp_content_type_v1::Type;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::reexports::wayland_server::{DisplayHandle, GlobalDispatch};
use smithay::wayland::GlobalData;
use smithay::wayland::compositor::with_states;
use smithay::wayland::content_type::{ContentTypeState, ContentTypeSurfaceCachedState};
use std::sync::Mutex;

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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ContentKind {
    /// No hint.
    #[default]
    None,
    /// Still images.
    Photo,
    /// Video.
    Video,
    /// A game.
    Game,
}

impl ContentKind {
    /// The wire value as a kind (the edge's conversion; a value this protocol version does not
    /// define reads as no hint).
    pub fn of_wire(wire: Type) -> ContentKind {
        match wire {
            Type::Photo => ContentKind::Photo,
            Type::Video => ContentKind::Video,
            Type::Game => ContentKind::Game,
            _ => ContentKind::None,
        }
    }
}

/// What a surface last committed, kept in the surface's own data (so it leaves with it).
#[derive(Debug, Default)]
struct Stored(Mutex<ContentKind>);

/// The kind `surface` committed with, from smithay's double-buffered state (the handler reads
/// it at every commit and passes it to [`on_content_type`]).
pub fn committed_kind(surface: &WlSurface) -> ContentKind {
    with_states(surface, |states| {
        ContentKind::of_wire(
            *states
                .cached_state
                .get::<ContentTypeSurfaceCachedState>()
                .current()
                .content_type(),
        )
    })
}

/// The kind `surface` was last stored with ([`ContentKind::None`] before any commit).
pub fn stored_kind(surface: &WlSurface) -> ContentKind {
    with_states(surface, |states| {
        states
            .data_map
            .get::<Stored>()
            .map(|stored| *stored.0.lock().unwrap_or_else(|poison| poison.into_inner()))
            .unwrap_or_default()
    })
}

/// `surface` committed `kind`: stored with the surface for the scheduler and, from P5, VRR.
/// No model event: nothing in the model depends on it yet.
pub fn on_content_type(
    part: &mut ContentTypePart,
    surface: &WlSurface,
    kind: ContentKind,
) -> Vec<Event> {
    let _ = part;
    with_states(surface, |states| {
        let stored = states.data_map.get_or_insert_threadsafe(Stored::default);
        *stored.0.lock().unwrap_or_else(|poison| poison.into_inner()) = kind;
    });
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wire_values_become_kinds() {
        assert_eq!(ContentKind::of_wire(Type::None), ContentKind::None);
        assert_eq!(ContentKind::of_wire(Type::Photo), ContentKind::Photo);
        assert_eq!(ContentKind::of_wire(Type::Video), ContentKind::Video);
        assert_eq!(ContentKind::of_wire(Type::Game), ContentKind::Game);
    }
}
