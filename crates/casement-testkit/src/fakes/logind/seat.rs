//! `org.freedesktop.login1.Seat` (P1-F2): `SwitchTo`.

use super::LogindError;

/// The seat object's methods. P1-F2 serves them as a `#[zbus::interface]`.
#[derive(Debug, Default)]
pub struct Seat;

impl Seat {
    /// `SwitchTo(vt)`, recorded.
    pub fn switch_to(&mut self, vt: u32) -> Result<(), LogindError> {
        let _ = vt;
        todo!("P1-F2: Seat::switch_to")
    }
}
