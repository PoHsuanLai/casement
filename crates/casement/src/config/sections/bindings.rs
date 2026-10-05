//! `[bindings]`: chord text to action.

use crate::config::action::ActionWords;
use crate::config::convert;
use casement_core::settings::CasementSettings;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// `[bindings]`: each key is a chord in its text form (`"cmd+space"`), each value what it does, as a list of words (`action.rs`).
/// Not in the schema: a chord table has no fixed keys for the Settings app to draw. A chord
/// or an action that does not parse is left out of the model and logged when the document is converted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct BindingsSection(pub BTreeMap<String, ActionWords>);

impl Default for BindingsSection {
    fn default() -> Self {
        convert::bindings_doc(&CasementSettings::defaults().bindings)
    }
}
