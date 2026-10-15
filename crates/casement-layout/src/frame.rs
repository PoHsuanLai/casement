//! The window frame casement draws for a client that asked for a server-side one, read from
//! quire's tokens: the chrome lengths, the colour table, `light_state`'s reveal rule. Pure; the
//! renderer draws the [`FrameStyle`] this makes.
//!
//! Three values are not reachable from quire's pure crates as numbers yet, so they are written
//! here with the token they mirror and an ask for quire (FINDINGS "wave-2026-10-15/ssd"): the
//! window radius (`--r-window`, a CSS string), the shadow (`--shadow-window`, a CSS string) and
//! the three light hues (stylesheet-only candy colours).

use casement_core::window::{
    FrameColour, FrameLook, FrameMetrics, FrameStyle, Light, LightDraw, ShadowLayer,
};
use ds_core::vocab::{Activity, Shown};
use ds_style::appearance::theme::{Scheme, Theme};
use ds_style::tokens::chrome::{ChromeToken, GroupHover, LightFill, LightState, light_state};
use ds_style::tokens::colour::ColourToken;
use ds_style::tokens::hex::Colour;

/// `--r-window` for the Sonoma and Sequoia window (quire's batch moves the token to 10).
pub const WINDOW_RADIUS: u32 = 10;

/// The close light, minimise light and zoom light hues, and the ink of their marks (the macOS
/// values the candy shelf holds).
const HUES: [(u32, u32); 3] = [
    (0xFF5F57, 0x4D0000),
    (0xFEBC2E, 0x985600),
    (0x28C840, 0x0A6400),
];

/// The frame's lengths from the chrome tokens.
pub fn metrics() -> FrameMetrics {
    let of = |token: ChromeToken| u32::from(token.length().0);
    FrameMetrics::new(
        of(ChromeToken::TitlebarHeight),
        of(ChromeToken::LightSize),
        of(ChromeToken::LightGap),
        of(ChromeToken::LightInset),
        of(ChromeToken::Edge),
        of(ChromeToken::EdgeCorner),
    )
}

/// The frame as it looks in `scheme` for a window in `activity` with the pointer as `hover`
/// says.
pub fn frame_style(scheme: Scheme, activity: Activity, hover: GroupHover) -> FrameStyle {
    let state = light_state(activity, hover);
    let token = |token: ColourToken| colour(token.value(scheme));
    let ground = match activity {
        Activity::Active => ColourToken::Paper,
        Activity::Inactive => ColourToken::Surface2,
    };
    FrameStyle {
        metrics: metrics(),
        look: FrameLook {
            radius: WINDOW_RADIUS,
            titlebar: token(ground),
            rule: token(ColourToken::Line),
            lights: HUES.map(|(hue, ink)| light(state, hue, ink, scheme)),
            shadow: shadow(scheme),
        },
    }
}

/// The scheme a person's `appearance.theme` paints. `System` would follow the desktop's
/// preference, which casement does not read (the settings portal is not built in), so it paints
/// Light, the scheme the desktop starts in.
///
/// ```
/// use casement_layout::{Scheme, Theme, scheme_of};
///
/// assert_eq!(scheme_of(Theme::Dark), Scheme::Dark);
/// assert_eq!(scheme_of(Theme::System), Scheme::Light);
/// ```
pub fn scheme_of(theme: Theme) -> Scheme {
    match theme {
        Theme::Dark => Scheme::Dark,
        _ => Scheme::Light,
    }
}

/// `style` with `light` drawn pressed: its hue a shade darker, its mark showing, as the Mac
/// draws a light under the button.
///
/// ```
/// use casement_core::window::Light;
/// use casement_layout::{GroupHover, Scheme, frame_style, pressed};
/// use ds_core::vocab::Activity;
///
/// let rest = frame_style(Scheme::Light, Activity::Active, GroupHover::Away);
/// let down = pressed(rest, Light::Close);
/// assert!(down.look.light(Light::Close).mark.is_some());
/// assert_eq!(down.look.light(Light::Zoom), rest.look.light(Light::Zoom));
/// ```
pub fn pressed(style: FrameStyle, light: Light) -> FrameStyle {
    let slot = match light {
        Light::Close => 0,
        Light::Minimise => 1,
        Light::Zoom => 2,
    };
    let (hue, ink) = HUES[slot];
    let mut look = style.look;
    look.lights[slot] = LightDraw {
        fill: darker(rgb(hue)),
        ring: darker(darker(rgb(hue))),
        mark: Some(rgb(ink)),
    };
    FrameStyle { look, ..style }
}

/// One light: its hue or the grey pill, and its mark while the group shows marks.
fn light(state: LightState, hue: u32, ink: u32, scheme: Scheme) -> LightDraw {
    let (fill, ring) = match state.fill {
        LightFill::Colour => (rgb(hue), darker(rgb(hue))),
        LightFill::Grey => (
            colour(ColourToken::Line.value(scheme)),
            colour(ColourToken::InkFaint.value(scheme)),
        ),
    };
    let mark = match state.marks {
        Shown::Visible => Some(rgb(ink)),
        Shown::Hidden => None,
    };
    LightDraw { fill, ring, mark }
}

/// `--shadow-window`: `0 1px 3px rgba(0,0,0,.12), 0 24px 64px -16px rgba(0,0,0,.4)` (dark:
/// .4 and .66).
fn shadow(scheme: Scheme) -> [ShadowLayer; 2] {
    let (contact, ambient) = match scheme {
        Scheme::Light => (0.12, 0.4),
        Scheme::Dark => (0.4, 0.66),
    };
    let black = |alpha: f64| FrameColour {
        r: 0,
        g: 0,
        b: 0,
        a: (alpha * 255.0).round() as u8,
    };
    [
        ShadowLayer {
            dy: 1,
            blur: 3,
            spread: 0,
            colour: black(contact),
        },
        ShadowLayer {
            dy: 24,
            blur: 64,
            spread: -16,
            colour: black(ambient),
        },
    ]
}

/// A token's value as a straight-alpha colour.
fn colour(value: Colour) -> FrameColour {
    match value {
        Colour::Solid(hex) => FrameColour {
            r: hex.0[0],
            g: hex.0[1],
            b: hex.0[2],
            a: 255,
        },
        Colour::Alpha(hex, alpha) => FrameColour {
            r: hex.0[0],
            g: hex.0[1],
            b: hex.0[2],
            a: u8::try_from((u32::from(alpha.0.min(1000)) * 255 + 500) / 1000).unwrap_or(255),
        },
    }
}

/// `0xRRGGBB` opaque.
fn rgb(value: u32) -> FrameColour {
    FrameColour {
        r: (value >> 16) as u8,
        g: (value >> 8) as u8,
        b: value as u8,
        a: 255,
    }
}

/// A hue's hairline: the hue at about four fifths.
fn darker(of: FrameColour) -> FrameColour {
    let shade = |channel: u8| (u16::from(channel) * 4 / 5) as u8;
    FrameColour {
        r: shade(of.r),
        g: shade(of.g),
        b: shade(of.b),
        a: of.a,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use casement_core::window::Light;

    #[test]
    fn the_metrics_are_the_chrome_tokens() {
        let m = metrics();
        assert_eq!(
            (m.titlebar, m.light, m.gap, m.inset, m.edge, m.corner),
            (28, 12, 8, 13, 4, 12)
        );
    }

    #[test]
    fn the_lights_follow_activity_and_hover() {
        const CASES: &[(&str, Activity, GroupHover, bool, bool)] = &[
            (
                "key at rest",
                Activity::Active,
                GroupHover::Away,
                true,
                false,
            ),
            (
                "key hovered",
                Activity::Active,
                GroupHover::Over,
                true,
                true,
            ),
            (
                "inactive at rest",
                Activity::Inactive,
                GroupHover::Away,
                false,
                false,
            ),
            (
                "inactive hovered",
                Activity::Inactive,
                GroupHover::Over,
                true,
                true,
            ),
        ];
        for (name, activity, hover, coloured, marked) in CASES {
            let draw = frame_style(Scheme::Light, *activity, *hover);
            for light in Light::ALL {
                let look = draw.look.light(light);
                assert_eq!(look.mark.is_some(), *marked, "{name} mark");
            }
            let red = draw.look.light(Light::Close).fill;
            assert_eq!(red == rgb(0xFF5F57), *coloured, "{name} fill");
        }
    }

    #[test]
    fn a_pressed_light_is_darker_with_its_mark_and_the_others_stay() {
        let rest = frame_style(Scheme::Light, Activity::Inactive, GroupHover::Away);
        for light in Light::ALL {
            let down = pressed(rest, light);
            let (was, now) = (rest.look.light(light), down.look.light(light));
            assert!(now.mark.is_some(), "{light:?} mark");
            assert_ne!(now.fill, was.fill, "{light:?} fill");
            for other in Light::ALL.into_iter().filter(|other| *other != light) {
                assert_eq!(down.look.light(other), rest.look.light(other), "{other:?}");
            }
        }
    }

    #[test]
    fn the_theme_picks_the_scheme() {
        const CASES: &[(&str, Theme, Scheme)] = &[
            ("dark", Theme::Dark, Scheme::Dark),
            ("light", Theme::Light, Scheme::Light),
            ("system is light", Theme::System, Scheme::Light),
        ];
        for (name, theme, want) in CASES {
            assert_eq!(scheme_of(*theme), *want, "{name}");
        }
    }

    #[test]
    fn a_dark_frame_has_a_dark_titlebar() {
        let light = frame_style(Scheme::Light, Activity::Active, GroupHover::Away);
        let dark = frame_style(Scheme::Dark, Activity::Active, GroupHover::Away);
        assert!(dark.look.titlebar.r < light.look.titlebar.r);
    }

    #[test]
    fn an_inactive_titlebar_is_lighter() {
        let key = frame_style(Scheme::Light, Activity::Active, GroupHover::Away);
        let idle = frame_style(Scheme::Light, Activity::Inactive, GroupHover::Away);
        assert!(idle.look.titlebar.r > key.look.titlebar.r);
    }
}
