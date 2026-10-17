//! `wp_security_context_manager_v1`: a sandbox engine (Flatpak) makes a listening socket whose
//! clients carry the engine's facts; they connect as `ConnectionRole::Sandboxed` and see less
//! (I7). Hidden from sandboxed clients themselves.

use crate::states::GlobalFilter;
use casement_core::Event;
use casement_core::policy::GlobalKind;
use porter_core::{PeerIdentity, SandboxFacts};
use smithay::reexports::wayland_protocols::wp::security_context::v1::server::{
    wp_security_context_manager_v1::WpSecurityContextManagerV1,
    wp_security_context_v1::WpSecurityContextV1,
};
use smithay::reexports::wayland_server::{Dispatch, DisplayHandle, GlobalDispatch};
use smithay::wayland::GlobalData;
use smithay::wayland::security_context::{
    SecurityContext, SecurityContextState, SecurityContextUserData,
};

/// `wp_security_context_manager_v1`.
#[derive(Debug)]
pub struct SecurityContextPart {
    /// smithay's state.
    pub state: SecurityContextState,
}

impl SecurityContextPart {
    /// Creates the global (version 1), filtered by the I7 table.
    pub fn new<D>(display: &DisplayHandle, role_filter: GlobalFilter) -> SecurityContextPart
    where
        D: GlobalDispatch<
                WpSecurityContextManagerV1,
                smithay::wayland::security_context::SecurityContextGlobalData,
            > + Dispatch<WpSecurityContextManagerV1, GlobalData>
            + Dispatch<WpSecurityContextV1, SecurityContextUserData>
            + 'static,
    {
        SecurityContextPart {
            state: SecurityContextState::new::<D, _>(
                display,
                role_filter.for_global(GlobalKind::SecurityContext),
            ),
        }
    }
}

/// The sandbox facts a security context carries (engine, app id, instance), as porter reads
/// them; clients of its listener connect as `Sandboxed(facts)`. The handler inserts the
/// listener into the event loop. A field the sandbox engine did not set is the empty text,
/// which porter's `identity_of` never takes for a name, so the clients stay unproven.
///
/// No event: a listener is not a client, and the model first hears of a sandboxed client when
/// it connects (`Event::ClientConnected` carries the role).
pub fn on_create_listener(
    part: &mut SecurityContextPart,
    context: &SecurityContext,
) -> (SandboxFacts, Vec<Event>) {
    let _ = part;
    let text = |field: &Option<String>| field.clone().unwrap_or_default();
    let facts = SandboxFacts::Engine {
        engine: text(&context.sandbox_engine),
        app: text(&context.app_id),
        instance: text(&context.instance_id),
    };
    (facts, Vec::new())
}

/// Who made a security context, as far as the desktop's caller table shows: the one program
/// it lets create sandboxes, or any other program.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Maker {
    /// The image of the table's `sandbox_launcher` row (`/usr/bin/flatpak`), root-owned.
    SandboxLauncher,
    /// Every other program, proven or not.
    #[default]
    Other,
}

/// What the clients of a context are known as. A context is its creator's claim about another
/// process (any public client may make one, and the engine's name, app id and instance id are
/// whatever it wrote), so only the known sandbox launcher can give its clients an app: facts
/// that would prove an app (`facts_prove`) count only when the `maker` is the launcher and is
/// itself proven. A context made by any other program, however well that program is proven
/// (a root-owned image in a scope it named itself is proven), leaves every client of its
/// socket with no app, an unknown one (CASW-04, owner decision 2026-10-17). The clients' powers
/// do not depend on this: they are sandboxed whoever made the context.
pub fn identity_for_clients(
    creator: &PeerIdentity,
    maker: Maker,
    facts_prove: PeerIdentity,
) -> PeerIdentity {
    match (maker, creator) {
        (Maker::SandboxLauncher, PeerIdentity::Proven(_)) => facts_prove,
        _ => PeerIdentity::Unproven,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use porter_core::{AppId, AppName, Isolation};

    fn proven(name: &str, isolation: Isolation) -> PeerIdentity {
        PeerIdentity::Proven(AppId {
            name: AppName::parse(name).expect("an app name"),
            isolation,
        })
    }

    /// (case, creator's proof, who made it, what the facts prove, what the clients are)
    #[test]
    fn only_the_sandbox_launcher_gives_a_contexts_clients_an_app() {
        let firefox = proven("org.mozilla.firefox", Isolation::Flatpak);
        let flatpak = proven("org.quire.Flatpak", Isolation::Unsandboxed);
        let other = proven("org.example.Tool", Isolation::Unsandboxed);
        let cases = [
            (
                "the launcher, facts prove an app",
                flatpak.clone(),
                Maker::SandboxLauncher,
                firefox.clone(),
                firefox.clone(),
            ),
            (
                "the launcher, facts prove nothing",
                flatpak,
                Maker::SandboxLauncher,
                PeerIdentity::Unproven,
                PeerIdentity::Unproven,
            ),
            (
                "the launcher row but the creator is not proven",
                PeerIdentity::Unproven,
                Maker::SandboxLauncher,
                firefox.clone(),
                PeerIdentity::Unproven,
            ),
            (
                "another root-owned proven program, facts prove an app",
                other.clone(),
                Maker::Other,
                firefox.clone(),
                PeerIdentity::Unproven,
            ),
            (
                "another proven program, facts prove nothing",
                other,
                Maker::Other,
                PeerIdentity::Unproven,
                PeerIdentity::Unproven,
            ),
            (
                "an unproven creator, facts prove an app",
                PeerIdentity::Unproven,
                Maker::Other,
                firefox,
                PeerIdentity::Unproven,
            ),
        ];
        for (name, creator, maker, proves, want) in cases {
            assert_eq!(
                identity_for_clients(&creator, maker, proves),
                want,
                "{name}"
            );
        }
    }
}
