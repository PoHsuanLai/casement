//! `casement --write-schema DIR`: the schema detent and the Settings app read.

use super::ConfigError;
use std::path::Path;

/// Writes `casement.settings.toml` into `to`.
pub fn write_schema(to: &Path) -> Result<(), ConfigError> {
    let _ = to;
    todo!("P0-F1-output: config::write_schema")
}
