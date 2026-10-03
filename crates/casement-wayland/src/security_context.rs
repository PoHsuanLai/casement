//! `wp_security_context_manager_v1`: a sandbox engine (Flatpak) makes a listening socket whose
//! clients carry the engine's facts; they connect as `ConnectionRole::Sandboxed` and see less
//! (I7). Hidden from sandboxed clients themselves.

use crate::states::GlobalFilter;
use casement_core::Event;
use casement_core::policy::GlobalKind;
use porter_core::SandboxFacts;
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
