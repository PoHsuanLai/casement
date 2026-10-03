//! `wp_security_context_manager_v1` (P0-H8).

use crate::state::State;
use smithay::wayland::security_context::{
    SecurityContext, SecurityContextHandler, SecurityContextListenerSource,
};

impl SecurityContextHandler for State {
    fn context_created(&mut self, source: SecurityContextListenerSource, context: SecurityContext) {
        let _ = (source, context);
        todo!("P0-H8: SecurityContextHandler::context_created")
    }
}
