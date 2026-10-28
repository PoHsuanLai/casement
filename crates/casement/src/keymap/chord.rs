//! A chordkit chord as a casement chord (pure).

use casement_core::input::{Chord, Keysym};
use casement_protocols::vocab::HeldMods;
use chordkit::{Key, Modifier, Modifiers, NamedKey};

/// `F1`'s keysym; the function keys follow in order.
const F1: u32 = 0xffbe;
/// The first keysym of the Unicode range: `0x0100_0000 + code point`.
const UNICODE_BASE: u32 = 0x0100_0000;

/// The casement chord that `chord` means: Super is Command (the key arrives as the logo key),
/// Alt is Option. `None` for a chord casement cannot see as a key press (a tapped modifier,
/// or a function key past F24).
///
/// chordkit writes `Shift+=` as `+` and `Shift+/` as `?`; the keyboard reports the unshifted
/// key with Shift held, so those two are written back that way.
pub fn chord_of(chord: chordkit::Chord) -> Option<Chord> {
    let (key, shifted) = match chord.key() {
        Key::Char('+') => (Keysym(0x3d), true),
        Key::Char('?') => (Keysym(0x2f), true),
        other => (keysym_of(other)?, false),
    };
    let mods = mods_of(chord.modifiers());
    Some(Chord {
        mods: match shifted {
            true => mods.with(HeldMods::SHIFT),
            false => mods,
        },
        key,
    })
}

fn mods_of(mods: Modifiers) -> HeldMods {
    mods.iter().fold(HeldMods::NONE, |held, one| {
        held.with(match one {
            Modifier::Ctrl => HeldMods::CONTROL,
            Modifier::Alt => HeldMods::OPTION,
            Modifier::Shift => HeldMods::SHIFT,
            Modifier::Super | Modifier::Meta => HeldMods::COMMAND,
        })
    })
}

fn keysym_of(key: Key) -> Option<Keysym> {
    match key {
        Key::Char(c) => Some(Keysym(match u32::from(c) {
            code @ (0x20..=0x7e | 0xa0..=0xff) => code,
            code => UNICODE_BASE + code,
        })),
        Key::Named(named) => named_keysym(named).map(Keysym),
        Key::Tap(_) => None,
    }
}

fn named_keysym(key: NamedKey) -> Option<u32> {
    match key {
        NamedKey::Space => Some(0x20),
        NamedKey::Tab => Some(0xff09),
        NamedKey::Enter => Some(0xff0d),
        NamedKey::Escape => Some(0xff1b),
        NamedKey::Backspace => Some(0xff08),
        NamedKey::Delete => Some(0xffff),
        NamedKey::Insert => Some(0xff63),
        NamedKey::Home => Some(0xff50),
        NamedKey::End => Some(0xff57),
        NamedKey::PageUp => Some(0xff55),
        NamedKey::PageDown => Some(0xff56),
        NamedKey::Up => Some(0xff52),
        NamedKey::Down => Some(0xff54),
        NamedKey::Left => Some(0xff51),
        NamedKey::Right => Some(0xff53),
        NamedKey::PrintScreen => Some(0xff61),
        NamedKey::F(n) => (1..=24).contains(&n).then(|| F1 + u32::from(n) - 1),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type Case = (&'static str, &'static str, Option<&'static str>);

    /// (case, chordkit text, casement text)
    const CASES: &[Case] = &[
        ("super is command", "Super+W", Some("cmd+w")),
        ("alt is option", "Alt+Super+H", Some("option+cmd+h")),
        (
            "ctrl and shift",
            "Ctrl+Shift+Super+F",
            Some("ctrl+shift+cmd+f"),
        ),
        ("a digit", "Shift+Super+3", Some("shift+cmd+3")),
        ("space", "Super+Space", Some("cmd+space")),
        ("tab", "Shift+Super+Tab", Some("shift+cmd+Tab")),
        ("escape", "Alt+Super+Escape", Some("option+cmd+Escape")),
        ("an arrow", "Ctrl+Left", Some("ctrl+Left")),
        ("print", "Print", Some("Print")),
        (
            "a function key",
            "Ctrl+Alt+Shift+Super+F19",
            Some("ctrl+option+shift+cmd+F19"),
        ),
        ("a modifier tapped alone", "Super", None),
    ];

    #[test]
    fn chords_read_as_casements_own() {
        for (case, written, want) in CASES {
            let chord: chordkit::Chord = written.parse().expect(case);
            let want = want.map(|text| Chord::parse(text).expect(case));
            assert_eq!(chord_of(chord), want, "{case}");
        }
    }

    #[test]
    fn the_symbols_shift_makes_come_back_as_shifted_keys() {
        let plus = chordkit::Chord::new(Modifiers::of(Modifier::Super), Key::Char('+'));
        let question = chordkit::Chord::new(Modifiers::of(Modifier::Super), Key::Char('?'));
        let want = |text| Some(Chord::parse(text).expect("a chord"));
        assert_eq!(chord_of(plus), want("shift+cmd+equal"));
        assert_eq!(chord_of(question), want("shift+cmd+slash"));
    }

    #[test]
    fn a_character_outside_latin_one_is_a_unicode_keysym() {
        let euro = chordkit::Chord::new(Modifiers::NONE, Key::Char('\u{20ac}'));
        assert_eq!(
            chord_of(euro).map(|chord| chord.key),
            Some(Keysym(0x0100_20ac))
        );
    }
}
