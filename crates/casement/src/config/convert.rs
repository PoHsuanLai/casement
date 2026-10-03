//! The one conversion from the stored document to the model's settings.

use super::doc::CasementDoc;
use casement_core::settings::CasementSettings;

/// The settings `doc` says; every key it leaves out takes `CasementSettings::defaults`'s value.
pub fn to_core(doc: &CasementDoc) -> CasementSettings {
    let _ = doc;
    todo!("P0-F1-output: config::to_core")
}
