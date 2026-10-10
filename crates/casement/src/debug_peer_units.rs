//! Reads the test unit overrides from the environment (compiled only under `debug-inject`; a
//! build without that feature does not contain this file, so the variable has no effect there).
//!
//! `CASEMENT_DEBUG_PEER_UNITS="<program-or-pid>=<unit>[,...]"`, for example
//! `sill=sill.service`. See `casement_ipc::dbus::PeerUnitTable::parse` for the grammar.

use casement_ipc::dbus::{PeerUnitTable, PeerUnitsError};

/// The variable's name.
const VARIABLE: &str = "CASEMENT_DEBUG_PEER_UNITS";

/// The table the environment gives; empty when the variable is unset.
pub fn from_env() -> Result<PeerUnitTable, PeerUnitsError> {
    parse(std::env::var(VARIABLE).ok().as_deref())
}

fn parse(value: Option<&str>) -> Result<PeerUnitTable, PeerUnitsError> {
    PeerUnitTable::parse(value.unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_unset_variable_is_an_empty_table_and_a_set_one_is_parsed() {
        assert!(parse(None).expect("unset").rules().is_empty());
        let table = parse(Some("sill=sill.service")).expect("set");
        assert_eq!(table.rules().len(), 1);
        assert!(parse(Some("nonsense")).is_err());
    }
}
