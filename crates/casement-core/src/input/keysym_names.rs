//! XKB keysym names, for the keys a binding can name. The compositor reads keysyms from
//! libxkbcommon; this pure table is only for the settings text, so it lists the keys people
//! bind (letters, digits, punctuation, the editing and function keys, the media keys) and
//! reaches any other keysym by its code: `0x1008ff13`, or `U20AC` for a Unicode character.

use super::Keysym;

/// The first keysym of the Unicode range: `0x0100_0000 + code point`.
const UNICODE_BASE: u32 = 0x0100_0000;
/// `F1`'s keysym; `F2` and the rest follow.
const F1: u32 = 0xffbe;
/// How many function keys have names (`F1` to `F24`).
const FUNCTION_KEYS: u32 = 24;

/// Names that are not their own character: ASCII punctuation, the editing keys, the media keys.
const NAMED: &[(&str, u32)] = &[
    ("space", 0x20),
    ("exclam", 0x21),
    ("quotedbl", 0x22),
    ("numbersign", 0x23),
    ("dollar", 0x24),
    ("percent", 0x25),
    ("ampersand", 0x26),
    ("apostrophe", 0x27),
    ("parenleft", 0x28),
    ("parenright", 0x29),
    ("asterisk", 0x2a),
    ("plus", 0x2b),
    ("comma", 0x2c),
    ("minus", 0x2d),
    ("period", 0x2e),
    ("slash", 0x2f),
    ("colon", 0x3a),
    ("semicolon", 0x3b),
    ("less", 0x3c),
    ("equal", 0x3d),
    ("greater", 0x3e),
    ("question", 0x3f),
    ("at", 0x40),
    ("bracketleft", 0x5b),
    ("backslash", 0x5c),
    ("bracketright", 0x5d),
    ("asciicircum", 0x5e),
    ("underscore", 0x5f),
    ("grave", 0x60),
    ("braceleft", 0x7b),
    ("bar", 0x7c),
    ("braceright", 0x7d),
    ("asciitilde", 0x7e),
    ("BackSpace", 0xff08),
    ("Tab", 0xff09),
    ("ISO_Left_Tab", 0xfe20),
    ("Return", 0xff0d),
    ("Escape", 0xff1b),
    ("Home", 0xff50),
    ("Left", 0xff51),
    ("Up", 0xff52),
    ("Right", 0xff53),
    ("Down", 0xff54),
    ("Page_Up", 0xff55),
    ("Page_Down", 0xff56),
    ("End", 0xff57),
    ("Print", 0xff61),
    ("Insert", 0xff63),
    ("Delete", 0xffff),
    ("XF86MonBrightnessUp", 0x1008_ff02),
    ("XF86MonBrightnessDown", 0x1008_ff03),
    ("XF86AudioLowerVolume", 0x1008_ff11),
    ("XF86AudioMute", 0x1008_ff12),
    ("XF86AudioRaiseVolume", 0x1008_ff13),
    ("XF86AudioPlay", 0x1008_ff14),
    ("XF86AudioStop", 0x1008_ff15),
    ("XF86AudioPrev", 0x1008_ff16),
    ("XF86AudioNext", 0x1008_ff17),
];

impl Keysym {
    /// The keysym named `name`: `a`, `4`, `space`, `Tab`, `F5`, `U20AC` or `0xff09`.
    /// `None` when the name is none of these.
    pub fn named(name: &str) -> Option<Keysym> {
        single_char(name)
            .or_else(|| table(name))
            .or_else(|| function_key(name))
            .or_else(|| unicode(name))
            .or_else(|| hex(name))
            .map(Keysym)
    }

    /// The keysym's name as `named` reads it back: its own character, its table name, `F<n>`,
    /// `U<hex>`, else `0x<hex>`.
    pub fn name(&self) -> String {
        let code = self.0;
        char_name(code)
            .or_else(|| {
                NAMED
                    .iter()
                    .find(|(_, sym)| *sym == code)
                    .map(|(name, _)| (*name).to_owned())
            })
            .or_else(|| function_name(code))
            .or_else(|| unicode_name(code))
            .unwrap_or_else(|| format!("{code:#x}"))
    }
}

/// A letter or a digit is its own name (`a` is `0x61`, `A` is `0x41`).
fn single_char(name: &str) -> Option<u32> {
    let mut chars = name.chars();
    let (Some(c), None) = (chars.next(), chars.next()) else {
        return None;
    };
    c.is_ascii_alphanumeric().then_some(u32::from(c))
}

fn char_name(code: u32) -> Option<String> {
    char::from_u32(code)
        .filter(char::is_ascii_alphanumeric)
        .map(String::from)
}

fn table(name: &str) -> Option<u32> {
    NAMED.iter().find(|(n, _)| *n == name).map(|(_, sym)| *sym)
}

fn function_key(name: &str) -> Option<u32> {
    let n: u32 = name.strip_prefix('F')?.parse().ok()?;
    let canonical = name == format!("F{n}");
    (canonical && (1..=FUNCTION_KEYS).contains(&n)).then(|| F1 + n - 1)
}

fn function_name(code: u32) -> Option<String> {
    let n = code.checked_sub(F1)? + 1;
    (1..=FUNCTION_KEYS).contains(&n).then(|| format!("F{n}"))
}

/// `U20AC`: a Unicode character above Latin-1's keysym range (those keysyms are the code point
/// itself, which the table and the single characters already cover).
fn unicode(name: &str) -> Option<u32> {
    let point = hex_digits(name.strip_prefix('U')?)?;
    (char::from_u32(point).is_some() && point >= 0x100).then_some(UNICODE_BASE + point)
}

fn unicode_name(code: u32) -> Option<String> {
    let point = code.checked_sub(UNICODE_BASE)?;
    (point >= 0x100 && char::from_u32(point).is_some()).then(|| format!("U{point:04X}"))
}

fn hex(name: &str) -> Option<u32> {
    hex_digits(name.strip_prefix("0x")?)
}

/// Hexadecimal digits only: `from_str_radix` alone would also take a leading `+`.
fn hex_digits(digits: &str) -> Option<u32> {
    let only_digits = !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_hexdigit());
    only_digits
        .then(|| u32::from_str_radix(digits, 16).ok())
        .flatten()
}
