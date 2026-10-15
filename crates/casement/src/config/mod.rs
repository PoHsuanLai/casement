//! The stored settings: `casement/settings.toml` through ds-settings (lenient read, atomic
//! write, a debounced watch), its schema for detent (`casement --write-schema`), and the one
//! conversion into the model's `CasementSettings` (ARCHITECTURE.md, "Add a settings key").

mod action;
mod convert;
mod doc;
mod schema;
mod sections;
mod version;

pub use action::ActionWords;
pub use convert::to_core;
pub(crate) use doc::watch;
pub use doc::{CasementDoc, load, xwayland_of};
pub use schema::{schema, write_schema};
pub use sections::{
    BindingsSection, FirstSight, GesturesSection, GpuSection, KeyboardSection, NightLightSection,
    OutputsSection, PointerSection, SpacesSection, TouchpadSection, WindowsSection,
    XwaylandSection, XwaylandUse,
};
pub use version::{Stamped, Version, stamp_version};

/// Why the settings could not be read or the schema written.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ConfigError {
    /// ds-settings refused.
    #[error("settings: {0}")]
    Settings(String),
    /// The schema could not be written.
    #[error("schema: {0}")]
    Schema(#[from] std::io::Error),
}
