//! `casement --write-schema DIR`: the schema detent and the Settings app read.

use super::ConfigError;
use super::sections::{
    GesturesSection, GpuSection, KeyboardSection, NightLightSection, OutputsSection,
    PointerSection, SpacesSection, TouchpadSection, WindowsSection, XwaylandSection,
};
use ds_settings::schema::{AppId, FilePath, Schema, SettingsSchema};
use std::path::Path;

/// The schema's version: bumped when a key's meaning changes, never for a new key.
const VERSION: u16 = 1;

/// Every key the document reads, as `<app-id>.settings.toml` ships it. `[bindings]` is not in
/// it: a chord table has no fixed keys to draw.
pub fn schema() -> Schema {
    Schema {
        app: AppId("casement".to_owned()),
        file: FilePath("casement/settings.toml".to_owned()),
        version: VERSION,
        foreign: Default::default(),
        key: [
            WindowsSection::schema(),
            SpacesSection::schema(),
            KeyboardSection::schema(),
            OutputsSection::schema(),
            PointerSection::schema(),
            TouchpadSection::schema(),
            GesturesSection::schema(),
            GpuSection::schema(),
            NightLightSection::schema(),
            XwaylandSection::schema(),
        ]
        .into_iter()
        .flat_map(|schema| schema.key)
        .collect(),
    }
}

/// Writes `casement.settings.toml` into `to` (created if needed, replaced atomically).
pub fn write_schema(to: &Path) -> Result<(), ConfigError> {
    schema().write_to(to)?;
    Ok(())
}
