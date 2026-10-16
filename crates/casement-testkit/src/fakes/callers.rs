//! Callers of `org.quire.Compositor1` as the bus thread proved them: the app and the role of
//! the caller-table row they matched. A test says who is calling instead of building a process
//! table; the proof itself (the process, its image, its unit) is `casement-ipc`'s tests.

use casement_ipc::dbus::Proven;
use casement_ipc::dbus::peer::{AppKey, RoleName, roles};

/// The caller `app` holding `role`.
pub fn proven(app: &str, role: &str) -> Proven {
    Proven::new(
        AppKey::new(app).expect("an app key"),
        RoleName::new(role).expect("a role name"),
    )
}

/// The shell (sill).
pub fn shell() -> Proven {
    Proven::new(
        AppKey::new("org.quire.Shell").expect("an app key"),
        roles::SHELL,
    )
}

/// The agent tier (cuad).
pub fn agent_tier() -> Proven {
    Proven::new(
        AppKey::new("org.quire.Agent").expect("an app key"),
        roles::AGENT_TIER,
    )
}

/// The portal backend (transom).
pub fn portal() -> Proven {
    Proven::new(
        AppKey::new("org.quire.Portal").expect("an app key"),
        roles::PORTAL,
    )
}

/// Some other desktop program (a role that is no connection's).
pub fn companion() -> Proven {
    Proven::new(
        AppKey::new("org.quire.Companion").expect("an app key"),
        roles::COMPANION,
    )
}
