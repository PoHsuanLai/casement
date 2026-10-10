//! A gamma ramp (`zwlr_gamma_control_v1`): the colour curve an output is driven with.

use crate::error::CoreError;

/// A ramp's three channels, each a table of 16-bit steps, all the same length.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GammaTable {
    red: Vec<u16>,
    green: Vec<u16>,
    blue: Vec<u16>,
}

impl GammaTable {
    /// A table from its channels, which must be the same non-zero length.
    pub fn new(red: Vec<u16>, green: Vec<u16>, blue: Vec<u16>) -> Result<GammaTable, CoreError> {
        let size = red.len();
        match size != 0 && green.len() == size && blue.len() == size {
            true => Ok(GammaTable { red, green, blue }),
            false => Err(CoreError::Malformed {
                what: "gamma table",
                text: format!("{}/{}/{} steps", red.len(), green.len(), blue.len()),
            }),
        }
    }

    /// How many steps each channel has.
    pub fn size(&self) -> usize {
        self.red.len()
    }

    /// The red channel.
    pub fn red(&self) -> &[u16] {
        &self.red
    }

    /// The green channel.
    pub fn green(&self) -> &[u16] {
        &self.green
    }

    /// The blue channel.
    pub fn blue(&self) -> &[u16] {
        &self.blue
    }
}

/// What an output's gamma should be.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum GammaRamp {
    /// The identity curve (what the output has when no client controls its gamma).
    Identity,
    /// This table (night light, a colour profile).
    Table(GammaTable),
}
