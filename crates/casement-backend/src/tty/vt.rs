//! VT switching as ordinary code (P1-Z; filled by P1-D1): the chord that asks for another VT.

use casement_core::input::Keysym;

/// A virtual terminal number, from 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct VtNumber(pub u32);

/// The VT a key symbol asks for: `XF86Switch_VT_1` to `XF86Switch_VT_12` are VT 1 to 12 (xkb
/// turns `Ctrl+Alt+Fn` into them).
///
/// ```
/// use casement_backend::tty::{VtNumber, vt_of_keysym};
/// use casement_core::input::Keysym;
///
/// assert_eq!(vt_of_keysym(Keysym(0x1008fe01)), Some(VtNumber(1)));
/// assert_eq!(vt_of_keysym(Keysym(0x20)), None);
/// ```
pub fn vt_of_keysym(sym: Keysym) -> Option<VtNumber> {
    match sym.0 {
        raw @ FIRST_SWITCH_SYM..=LAST_SWITCH_SYM => Some(VtNumber(raw - FIRST_SWITCH_SYM + 1)),
        _ => None,
    }
}

/// `XF86Switch_VT_1`.
const FIRST_SWITCH_SYM: u32 = 0x1008_fe01;

/// `XF86Switch_VT_12`.
const LAST_SWITCH_SYM: u32 = 0x1008_fe0c;
