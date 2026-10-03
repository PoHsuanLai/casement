//! The default key bindings (C30: casement's settings document carries them; sill ships none).

use crate::input::Bindings;

/// The bindings a fresh settings document holds: none yet (FINDINGS: the default table).
pub(crate) fn defaults() -> Bindings {
    Bindings(Vec::new())
}
