//! `xdg_system_bell_v1`: the alert sound, attributed to an app when a surface is given.

use crate::states::GlobalFilter;
use casement_core::app::AppKey;
use casement_core::{Event, Model, WindowId};
use smithay::reexports::wayland_protocols::xdg::system_bell::v1::server::xdg_system_bell_v1::XdgSystemBellV1;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::reexports::wayland_server::{DisplayHandle, GlobalDispatch};
use smithay::wayland::GlobalData;
use smithay::wayland::xdg_system_bell::{XdgSystemBellHandler, XdgSystemBellState};

/// `xdg_system_bell_v1`.
#[derive(Debug)]
pub struct SystemBellPart {
    /// smithay's state.
    pub state: XdgSystemBellState,
}

impl SystemBellPart {
    /// Creates the global (version 1). Every connection sees it.
    pub fn new<D>(display: &DisplayHandle, role_filter: GlobalFilter) -> SystemBellPart
    where
        D: XdgSystemBellHandler + GlobalDispatch<XdgSystemBellV1, GlobalData>,
    {
        let _ = role_filter;
        SystemBellPart {
            state: XdgSystemBellState::new::<D>(display),
        }
    }
}

/// The bell rang, for `surface`'s app when given. The model has no bell event: the effect
/// (`Effect::Bell`) is built by the handler from [`app_to_ring`], so this reports nothing.
pub fn on_bell(part: &mut SystemBellPart, surface: Option<&WlSurface>) -> Vec<Event> {
    let _ = (part, surface);
    Vec::new()
}

/// The app a bell is for: the app of `window` when the client named a surface of one of its
/// windows, else the active app (a bell with no surface is the app the person is in), else
/// nobody (a bell for no one is not rung).
pub fn app_to_ring(model: &Model, window: Option<WindowId>) -> Option<AppKey> {
    window
        .and_then(|id| model.windows().get(&id))
        .map(|window| window.app.clone())
        .or_else(|| model.focus().active.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use casement_core::focus::TokenCheck;
    use casement_core::geometry::LogicalSize;
    use casement_core::policy::ConnectionRole;
    use casement_core::settings::CasementSettings;
    use casement_core::space::SpaceSeed;
    use casement_core::time::Mono;
    use casement_core::window::{MaxSize, Parent, SizeLimits};
    use casement_core::{ClientKey, MappedToplevel};
    use casement_protocols::vocab::{Modality, WindowRole};
    use porter_core::{ClaimedId, PeerIdentity};
    use std::time::Duration;

    fn mapped(model: Model, id: u64, claimed: &str) -> Model {
        let client = ClientKey(id);
        let claimed = ClaimedId::parse(claimed).expect("a claimed id");
        let now = Mono(Duration::from_millis(10));
        let (model, _) = model.step(
            Event::ClientConnected {
                client,
                peer: PeerIdentity::Unproven,
                role: ConnectionRole::Public,
            },
            now,
        );
        let (model, _) = model.step(
            Event::ToplevelMapped(MappedToplevel {
                window: WindowId(id),
                client,
                app: AppKey::of(&PeerIdentity::Unproven, &claimed),
                claimed_id: claimed,
                role: WindowRole::Document,
                modality: Modality::None,
                parent: Parent::None,
                limits: SizeLimits {
                    min: LogicalSize { w: 0, h: 0 },
                    max: MaxSize::Unbounded,
                },
                first_size: LogicalSize { w: 640, h: 480 },
                activation: TokenCheck::Missing,
            }),
            now,
        );
        model
    }

    /// A surface's window gives its app; no surface (or none that is ours) gives the active app;
    /// with neither there is no one to ring for.
    #[test]
    fn the_bell_rings_for_the_surfaces_app_else_the_active_one() {
        let empty = Model::new(
            CasementSettings::defaults(),
            Mono(Duration::ZERO),
            SpaceSeed::from_entropy([7; 16]),
        );
        assert_eq!(app_to_ring(&empty, None), None);
        assert_eq!(app_to_ring(&empty, Some(WindowId(1))), None);

        let model = mapped(mapped(empty, 1, "org.example.One"), 2, "org.example.Two");
        let one = AppKey::of(
            &PeerIdentity::Unproven,
            &ClaimedId::parse("org.example.One").expect("id"),
        );
        assert_eq!(app_to_ring(&model, Some(WindowId(1))), Some(one));
        let two = AppKey::of(
            &PeerIdentity::Unproven,
            &ClaimedId::parse("org.example.Two").expect("id"),
        );
        assert_eq!(app_to_ring(&model, Some(WindowId(2))), Some(two));
        // An unknown surface falls back to whoever is active (a window mapped with no token
        // takes no focus, A7, so nobody is active here).
        assert_eq!(
            app_to_ring(&model, Some(WindowId(99))),
            model.focus().active.clone()
        );
    }
}
