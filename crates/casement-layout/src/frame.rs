//! The window frame casement draws for a client that asked for a server-side one, read from
//! quire's tokens: the chrome lengths, the colour table, `light_state`'s reveal rule. Pure; the
//! renderer draws the [`FrameStyle`] this makes.
//!
//! Three values are not reachable from quire's pure crates as numbers yet, so they are written
//! here with the token they mirror and an ask for quire (FINDINGS "wave-2026-10-15/ssd"): the
//! window radius (`--r-window`, a CSS string), the shadow (`--shadow-window`, a CSS string) and
//! the three light hues (stylesheet-only candy colours).

use casement_core::window::{
    FrameColour, FrameLook, FrameMetrics, FrameStyle, LightDraw, ShadowLayer,
};
use ds_core::vocab::{Activity, Shown};
use ds_style::appearance::theme::Scheme;
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
    fn an_inactive_titlebar_is_lighter() {
        let key = frame_style(Scheme::Light, Activity::Active, GroupHover::Away);
        let idle = frame_style(Scheme::Light, Activity::Inactive, GroupHover::Away);
        assert!(idle.look.titlebar.r > key.look.titlebar.r);
    }
}
