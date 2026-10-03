//! The document.

use super::ConfigError;
use super::sections::{
    BindingsSection, KeyboardSection, OutputsSection, SpacesSection, WindowsSection,
};
use crate::cli::ConfigHome;
use ds_settings::{FileName, Format, SettingsDoc};
use serde::{Deserialize, Serialize};

/// `casement/settings.toml`: one section per `CasementSettings` part. A key it does not read
/// is reported on load and dropped on save (ds-settings).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct CasementDoc {
    /// `windows.*`.
    pub windows: WindowsSection,
    /// `spaces.*`.
    pub spaces: SpacesSection,
    /// `keyboard.*`.
    pub keyboard: KeyboardSection,
    /// `[bindings]` (C30: casement's document carries the defaults).
    pub bindings: BindingsSection,
    /// `outputs.*`.
    pub outputs: OutputsSection,
}

impl SettingsDoc for CasementDoc {
    const FILE: FileName = FileName("settings.toml");
    const FORMAT: Format = Format::Toml;
}

/// The document under `home` (missing file: the defaults).
pub fn load(home: &ConfigHome) -> Result<CasementDoc, ConfigError> {
    let _ = home;
    todo!("P0-F1-output: config::load")
}
