//! The document's sections. Each key gets a `#[settings(...)]` row (ds-settings'
//! `SettingsSchema` derive) as P0-F1-output writes it; the sections are empty at the freeze.

use serde::{Deserialize, Serialize};

/// `windows.*` (03 §1.17).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct WindowsSection {}

/// `spaces.*`.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct SpacesSection {}

/// `keyboard.*`.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct KeyboardSection {}

/// `[bindings]`: chord text to action.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct BindingsSection {}

/// `outputs.*`.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct OutputsSection {}
