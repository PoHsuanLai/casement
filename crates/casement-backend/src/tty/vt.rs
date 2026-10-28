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

/// The VT a key press asks for, from the symbols the keymap gives it with the modifiers
/// applied (`modified`) and without (`raw`). `ctrl_alt` is whether Control and Alt are both held,
/// the only modifiers a VT switch cares about.
///
/// The keymap's own `XF86Switch_VT_n` wins. Where the keymap has none (a layout without the
/// server-control group, or a keyboard whose F-row sends media keys until Fn is held), plain
/// `Ctrl+Alt+Fn` still asks for VT n, read from the F1 to F12 symbols.
///
/// ```
/// use casement_backend::tty::{VtNumber, vt_of_chord};
/// use casement_core::input::Keysym;
///
/// let f2 = [Keysym(0xffbf)];
/// assert_eq!(vt_of_chord(true, &[], &f2), Some(VtNumber(2)));
/// assert_eq!(vt_of_chord(false, &f2, &f2), None);
/// ```
pub fn vt_of_chord(ctrl_alt: bool, modified: &[Keysym], raw: &[Keysym]) -> Option<VtNumber> {
    let named = modified.iter().find_map(|sym| vt_of_keysym(*sym));
    match ctrl_alt {
        true => named.or_else(|| {
            modified
                .iter()
                .chain(raw)
                .find_map(|sym| vt_of_function_key(*sym))
        }),
        false => named,
    }
}

/// The VT an `F1` to `F12` symbol stands for.
fn vt_of_function_key(sym: Keysym) -> Option<VtNumber> {
    match sym.0 {
        raw @ FIRST_FUNCTION_SYM..=LAST_FUNCTION_SYM => {
            Some(VtNumber(raw - FIRST_FUNCTION_SYM + 1))
        }
        _ => None,
    }
}

/// `XK_F1`.
const FIRST_FUNCTION_SYM: u32 = 0xffbe;

/// `XK_F12`.
const LAST_FUNCTION_SYM: u32 = 0xffc9;
