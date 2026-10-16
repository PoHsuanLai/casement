//! A gamma ramp (`zwlr_gamma_control_v1`): the colour curve an output is driven with.

use super::night_light::Tint;
use crate::error::CoreError;

/// The share of the identity curve ([`GammaTable::floored`]) a client's ramp may not go under,
/// as a divisor (an eighth): dim, but a screen a person can still read to put right.
const FLOOR_DIVISOR: u64 = 8;

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

    /// A linear ramp of `size` steps with each channel scaled by `tint` (thousandths): the
    /// identity curve, warmed. A size under two cannot hold a ramp.
    pub fn tinted(size: u32, tint: Tint) -> Result<GammaTable, CoreError> {
        let last = size
            .checked_sub(1)
            .filter(|last| *last > 0)
            .ok_or_else(|| CoreError::Malformed {
                what: "gamma table",
                text: format!("{size} steps"),
            })?;
        let channel = |share: u16| -> Vec<u16> {
            (0..size)
                .map(|step| {
                    let level = u64::from(step) * 65535 / u64::from(last);
                    u16::try_from(level * u64::from(share) / 1000).unwrap_or(u16::MAX)
                })
                .collect()
        };
        GammaTable::new(channel(tint.0[0]), channel(tint.0[1]), channel(tint.0[2]))
    }

    /// The table with every step raised to at least an eighth of the identity curve's: a ramp
    /// from a client can warm or dim a display, never black it out (CASW-18). A table of one
    /// step has no curve to compare with and is left as it is.
    #[must_use]
    pub fn floored(self) -> GammaTable {
        let size = self.size();
        let last = u64::try_from(size.saturating_sub(1)).unwrap_or(0);
        let lift = |channel: Vec<u16>| -> Vec<u16> {
            channel
                .into_iter()
                .enumerate()
                .map(|(step, level)| {
                    let step = u64::try_from(step).unwrap_or(0);
                    let floor = match last {
                        0 => 0,
                        last => step * 65535 / last / FLOOR_DIVISOR,
                    };
                    level.max(u16::try_from(floor).unwrap_or(u16::MAX))
                })
                .collect()
        };
        GammaTable {
            red: lift(self.red),
            green: lift(self.green),
            blue: lift(self.blue),
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
