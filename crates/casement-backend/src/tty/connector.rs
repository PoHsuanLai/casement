//! A KMS connector as casement's output facts (P1-Z; filled by P1-D2).

use super::kms::ConnectorFacts;
use casement_core::output::{EdidFacts, OutputFacts};

/// Why an EDID block could not be read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum EdidError {
    /// The block is shorter than the 128 bytes the base block takes.
    #[error("EDID block too short: {0} bytes")]
    TooShort(usize),
    /// The fixed header is wrong.
    #[error("EDID header is wrong")]
    BadHeader,
    /// The checksum does not add up.
    #[error("EDID checksum is wrong")]
    BadChecksum,
}

/// The make, model and serial an EDID base block reports.
///
/// ```no_run
/// use casement_backend::tty::parse_edid;
///
/// assert!(parse_edid(&[]).is_err());
/// ```
pub fn parse_edid(bytes: &[u8]) -> Result<EdidFacts, EdidError> {
    let _ = bytes;
    todo!("P1-D2: parse_edid")
}

/// The facts of an attached connector: its identity from the EDID (an unreadable one gives an
/// anonymous display keyed by its connector), its modes, physical size and variable-refresh
/// capability.
pub fn output_facts(connector: &ConnectorFacts) -> OutputFacts {
    let _ = connector;
    todo!("P1-D2: output_facts")
}
