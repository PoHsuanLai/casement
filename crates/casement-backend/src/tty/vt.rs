//! VT switching as ordinary code (P1-Z; filled by P1-D1): the chord that asks for another VT.

use casement_core::input::Keysym;

/// A virtual terminal number, from 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct VtNumber(pub u32);

/// The VT a key symbol asks for: `XF86Switch_VT_1` to `XF86Switch_VT_12` are VT 1 to 12 (xkb
/// turns `Ctrl+Alt+Fn` into them).
///
/// ```no_run
/// use casement_backend::tty::vt_of_keysym;
/// use casement_core::input::Keysym;
///
/// assert!(vt_of_keysym(Keysym(0x1008fe01)).is_some());
/// ```
pub fn vt_of_keysym(sym: Keysym) -> Option<VtNumber> {
    let _ = sym;
    todo!("P1-D1: vt_of_keysym")
}
