//! The title's ink: quire's primary ink for the key window, its secondary ink for the others,
//! in the scheme the frame is drawn in.

use ds_core::vocab::Activity;
use ds_style::appearance::theme::Scheme;
use ds_style::tokens::colour::ColourToken;
use ds_style::tokens::hex::Colour;

/// A colour with straight (not premultiplied) alpha.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Ink {
    /// Red.
    pub r: u8,
    /// Green.
    pub g: u8,
    /// Blue.
    pub b: u8,
    /// Alpha, 255 opaque.
    pub a: u8,
}

impl Ink {
    /// A token's value as ink.
    pub fn of(value: Colour) -> Ink {
        match value {
            Colour::Solid(hex) => Ink {
                r: hex.0[0],
                g: hex.0[1],
                b: hex.0[2],
                a: 255,
            },
            Colour::Alpha(hex, alpha) => Ink {
                r: hex.0[0],
                g: hex.0[1],
                b: hex.0[2],
                a: u8::try_from((u32::from(alpha.0.min(1000)) * 255 + 500) / 1000).unwrap_or(255),
            },
        }
    }
}

/// The title's ink in `scheme` for a window in `activity`: `--ink` while it is the key window,
/// `--ink-soft` otherwise.
///
/// ```
/// use casement_chrome::ink;
/// use ds_core::vocab::Activity;
/// use ds_style::appearance::theme::Scheme;
///
/// assert_ne!(
///     ink(Scheme::Light, Activity::Active),
///     ink(Scheme::Light, Activity::Inactive)
/// );
/// ```
pub fn ink(scheme: Scheme, activity: Activity) -> Ink {
    let token = match activity {
        Activity::Active => ColourToken::Ink,
        Activity::Inactive => ColourToken::InkSoft,
    };
    Ink::of(token.value(scheme))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ds_style::tokens::hex::{Alpha, Hex};

    #[test]
    fn a_token_value_becomes_straight_alpha_ink() {
        const CASES: &[(&str, Colour, Ink)] = &[
            (
                "solid",
                Colour::Solid(Hex([1, 2, 3])),
                Ink {
                    r: 1,
                    g: 2,
                    b: 3,
                    a: 255,
                },
            ),
            (
                "half",
                Colour::Alpha(Hex([10, 20, 30]), Alpha(500)),
                Ink {
                    r: 10,
                    g: 20,
                    b: 30,
                    a: 128,
                },
            ),
            (
                "clear",
                Colour::Alpha(Hex([0, 0, 0]), Alpha(0)),
                Ink {
                    r: 0,
                    g: 0,
                    b: 0,
                    a: 0,
                },
            ),
            (
                "over full is full",
                Colour::Alpha(Hex([0, 0, 0]), Alpha(2000)),
                Ink {
                    r: 0,
                    g: 0,
                    b: 0,
                    a: 255,
                },
            ),
        ];
        for (name, value, want) in CASES {
            assert_eq!(Ink::of(*value), *want, "{name}");
        }
    }

    #[test]
    fn the_ink_is_the_token_per_scheme_and_activity() {
        const CASES: &[(Scheme, Activity, ColourToken)] = &[
            (Scheme::Light, Activity::Active, ColourToken::Ink),
            (Scheme::Light, Activity::Inactive, ColourToken::InkSoft),
            (Scheme::Dark, Activity::Active, ColourToken::Ink),
            (Scheme::Dark, Activity::Inactive, ColourToken::InkSoft),
        ];
        for (scheme, activity, token) in CASES {
            assert_eq!(
                ink(*scheme, *activity),
                Ink::of(token.value(*scheme)),
                "{scheme:?} {activity:?}"
            );
        }
    }

    #[test]
    fn dark_ink_is_lighter_than_light_ink() {
        for activity in [Activity::Active, Activity::Inactive] {
            let (light, dark) = (ink(Scheme::Light, activity), ink(Scheme::Dark, activity));
            assert!(dark.r > light.r, "{activity:?}");
        }
    }
}
