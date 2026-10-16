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

/// What the clients of a context are known as. A context is its creator's claim about another
/// process (any public client may make one, and the engine's name, app id and instance id are
/// whatever it wrote), so it carries at most the creator's own proof: facts that would prove
/// an app (`facts_prove`) count only when the creator is itself proven, and a context made by
/// an unproven client leaves every client of its socket unproven (CASW-04).
pub fn identity_for_clients(creator: &PeerIdentity, facts_prove: PeerIdentity) -> PeerIdentity {
    match creator {
        PeerIdentity::Proven(_) => facts_prove,
        PeerIdentity::Unproven => PeerIdentity::Unproven,
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

    #[test]
    fn a_context_carries_at_most_its_creators_proof() {
        let firefox = proven("org.mozilla.firefox", Isolation::Flatpak);
        let launcher = proven("org.example.Launcher", Isolation::Unsandboxed);
        let cases = [
            (
                "unproven creator, facts prove an app",
                PeerIdentity::Unproven,
                firefox.clone(),
                PeerIdentity::Unproven,
            ),
            (
                "unproven creator, facts prove nothing",
                PeerIdentity::Unproven,
                PeerIdentity::Unproven,
                PeerIdentity::Unproven,
            ),
            (
                "proven creator, facts prove an app",
                launcher.clone(),
                firefox.clone(),
                firefox,
            ),
            (
                "proven creator, facts prove nothing",
                launcher,
                PeerIdentity::Unproven,
                PeerIdentity::Unproven,
            ),
        ];
        for (name, creator, proves, want) in cases {
            assert_eq!(identity_for_clients(&creator, proves), want, "{name}");
        }
    }
}
