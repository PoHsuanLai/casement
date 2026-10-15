//! The document.

use super::ConfigError;
use super::sections::{
    BindingsSection, GpuSection, KeyboardSection, NightLightSection, OutputsSection,
    PointerSection, SpacesSection, TouchpadSection, WindowsSection,
};
use super::version::Version;
use crate::cli::ConfigHome;
use ds_settings::{AppName, ConfigRoot, FileName, Format, SettingsDoc, Store, Watch};
use serde::{Deserialize, Serialize};

/// `casement/settings.toml`: one section per `CasementSettings` part. A key it does not read
/// is reported on load and dropped on save (ds-settings).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct CasementDoc {
    /// `version`: which defaults the file was written under (see [`Version`]); none in a file
    /// written before the file had one. [`load`] brings it to the current version.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<Version>,
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
    /// `pointer.*` (mice and trackballs).
    pub pointer: PointerSection,
    /// `touchpad.*`.
    pub touchpad: TouchpadSection,
    /// `gpu.*`.
    pub gpu: GpuSection,
    /// `night_light.*`.
    pub night_light: NightLightSection,
}

impl SettingsDoc for CasementDoc {
    const FILE: FileName = FileName("settings.toml");
    const FORMAT: Format = Format::Toml;
}

/// The program's directory name under the config home.
const APP: AppName = AppName("casement");

/// The document under `home` (a missing file: the defaults). A key the file sets that the
/// document does not read, or whose value it refuses, is reported in the log and costs only
/// itself.
///
/// `ConfigHome::At(dir)` stands in for `$XDG_CONFIG_HOME`, so the file is `dir/casement/settings.toml`
/// (ds-settings' scratch root); `ConfigHome::Xdg` reads the environment.
pub fn load(home: &ConfigHome) -> Result<CasementDoc, ConfigError> {
    let loaded = store(home).load::<CasementDoc>();
    for key in &loaded.unknown {
        tracing::warn!("settings.toml: {key}");
    }
    for key in &loaded.invalid {
        tracing::warn!("settings.toml: {key}");
    }
    Ok(loaded.value.migrated())
}

/// `home` as the ds-settings store it names.
pub(crate) fn store(home: &ConfigHome) -> Store {
    let root = match home {
        ConfigHome::Xdg => ConfigRoot::Xdg,
        ConfigHome::At(dir) => ConfigRoot::Scratch(dir.clone()),
    };
    Store::new(root, APP)
}

/// A watch on `home`'s document: every settled change of the file arrives whole (ds-settings,
/// debounced). It runs as a task on `spawn`.
pub(crate) fn watch(
    home: &ConfigHome,
    spawn: &dyn ds_core::spawner::Spawner,
) -> Watch<CasementDoc> {
    store(home).watch::<CasementDoc>(spawn)
}
