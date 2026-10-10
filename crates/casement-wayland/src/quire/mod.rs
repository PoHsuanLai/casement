//! The private `quire_*` protocols served to the shell and the agent tier (P2-Z; behind the
//! crate's `quire` feature, A8): `quire_desktop_manager_v1` (the desktop model of apps,
//! windows and Spaces) and `quire_shell_v1` (the shell's surface roles and the events only the
//! compositor sees). `quire_window_manager_v1`, which every app may use, stays beside the
//! standard protocols in [`crate::quire_window`].
//!
//! Neither global is created yet: each `Part::new` is written, behind a switch
//! ([`Advertised`]) its lane flips when its bodies are filled, so no client sees a protocol
//! whose requests would reach a `todo!()`. The I7 rows are in force already
//! (`casement_core::policy::advertised`): the shell sees both, the agent tier sees the
//! desktop manager (and is refused its management requests), the public socket and
//! sandboxes see neither.

pub mod desktop;
pub mod shell;

use smithay::reexports::wayland_server::WEnum;

/// Whether a protocol's global is created yet. The freeze leaves both `quire` protocols
/// [`Advertised::Not`]; the lane that fills one changes its constant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Advertised {
    /// `Part::new` creates no global.
    Not,
    /// `Part::new` creates the global behind the I7 filter.
    Yes,
}

/// The wire code of a request's enum argument (`None` for a value the XML does not name).
pub(crate) fn code_of<T: Into<u32>>(value: WEnum<T>) -> Option<u32> {
    match value {
        WEnum::Value(value) => Some(value.into()),
        WEnum::Unknown(_) => None,
    }
}
