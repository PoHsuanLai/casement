//! What a title raster depends on: the cache key.

use crate::place::device;
use ds_core::geometry::scale::Scale;
use ds_core::vocab::Activity;
use ds_style::appearance::theme::Scheme;

/// The most characters of a title that are ever shaped. A client's title is untrusted text of
/// any length; this bounds the work and the key's memory.
pub const TITLE_CHARS: usize = 256;

/// One title raster's inputs: the text, the titlebar's logical size, the output scale, the
/// colour scheme and whether the window is the key window.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct TitleKey {
    /// The title, tidied by [`tidy`].
    pub title: String,
    /// The titlebar's width in logical pixels.
    pub width: u32,
    /// The titlebar's height in logical pixels.
    pub height: u32,
    /// The output's scale.
    pub scale: Scale,
    /// The scheme the frame is drawn in.
    pub scheme: Scheme,
    /// Whether the window is the key window (primary ink) or not (secondary ink).
    pub activity: Activity,
}

impl TitleKey {
    /// A key for `title` in a `width` by `height` titlebar (logical pixels) at `scale`.
    ///
    /// ```
    /// use casement_chrome::TitleKey;
    /// use ds_core::geometry::scale::Scale;
    /// use ds_core::vocab::Activity;
    /// use ds_style::appearance::theme::Scheme;
    ///
    /// let key = TitleKey::new("a\nb", 400, 28, Scale(180), Scheme::Light, Activity::Active);
    /// assert_eq!(key.title, "a b");
    /// assert_eq!(key.device_size(), (600, 42));
    /// ```
    pub fn new(
        title: &str,
        width: u32,
        height: u32,
        scale: Scale,
        scheme: Scheme,
        activity: Activity,
    ) -> TitleKey {
        TitleKey {
            title: tidy(title),
            width,
            height,
            scale,
            scheme,
            activity,
        }
    }

    /// The raster's size in device pixels, rounded as the surface buffers are.
    pub fn device_size(&self) -> (u32, u32) {
        (
            device(self.width, self.scale),
            device(self.height, self.scale),
        )
    }
}

/// A client's title made fit to draw: control characters (a newline, a tab, an escape) become
/// spaces, the ends are trimmed, and at most [`TITLE_CHARS`] characters stay.
///
/// ```
/// use casement_chrome::key::tidy;
///
/// assert_eq!(tidy("  Notes\t- draft\n"), "Notes - draft");
/// ```
pub fn tidy(raw: &str) -> String {
    let spaced: String = raw
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .take(TITLE_CHARS)
        .collect();
    spaced.trim().to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_title_is_tidied_before_it_is_keyed() {
        let long = "x".repeat(TITLE_CHARS + 40);
        let capped = "x".repeat(TITLE_CHARS);
        let cases: &[(&str, &str, &str)] = &[
            ("plain", "Untitled", "Untitled"),
            ("trimmed", "  Untitled  ", "Untitled"),
            ("newline", "a\nb", "a b"),
            ("tab and escape", "a\t\u{1b}b", "a  b"),
            ("empty", "", ""),
            ("only controls", "\n\t", ""),
            ("capped", long.as_str(), capped.as_str()),
            (
                "wide characters",
                "\u{65e5}\u{672c}\u{8a9e}",
                "\u{65e5}\u{672c}\u{8a9e}",
            ),
        ];
        for (name, raw, want) in cases {
            assert_eq!(tidy(raw), *want, "{name}");
        }
    }

    #[test]
    fn the_device_size_rounds_as_the_buffers_do() {
        const CASES: &[(u32, u32, u32, u32, u32)] = &[
            (400, 28, 120, 400, 28),
            (400, 28, 150, 500, 35),
            (401, 28, 150, 501, 35),
            (400, 28, 180, 600, 42),
            (333, 28, 240, 666, 56),
        ];
        for (w, h, scale, want_w, want_h) in CASES {
            let key = TitleKey::new("t", *w, *h, Scale(*scale), Scheme::Light, Activity::Active);
            assert_eq!(key.device_size(), (*want_w, *want_h), "{w}x{h} at {scale}");
        }
    }

    #[test]
    fn every_field_is_part_of_the_key() {
        let base = TitleKey::new("t", 400, 28, Scale(120), Scheme::Light, Activity::Active);
        let others = [
            TitleKey::new("u", 400, 28, Scale(120), Scheme::Light, Activity::Active),
            TitleKey::new("t", 401, 28, Scale(120), Scheme::Light, Activity::Active),
            TitleKey::new("t", 400, 29, Scale(120), Scheme::Light, Activity::Active),
            TitleKey::new("t", 400, 28, Scale(180), Scheme::Light, Activity::Active),
            TitleKey::new("t", 400, 28, Scale(120), Scheme::Dark, Activity::Active),
            TitleKey::new("t", 400, 28, Scale(120), Scheme::Light, Activity::Inactive),
        ];
        for other in &others {
            assert_ne!(&base, other);
        }
        assert_eq!(
            base,
            TitleKey::new(" t\n", 400, 28, Scale(120), Scheme::Light, Activity::Active)
        );
    }
}
