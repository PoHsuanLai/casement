//! Connection policy: which globals each kind of connection sees (I7), and which of them
//! need the person's consent.

mod globals;
mod power;

pub use globals::{ConnectionRole, GlobalKind, advertised, advertised_to};
pub use power::{Power, PowerSet, power_of, trusted_powers};
