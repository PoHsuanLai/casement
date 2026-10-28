//! The engine: shape with parley, truncate, rasterise with vello_cpu.
//!
//! One [`FontContext`] lives as long as the [`TitleRaster`], so the system fonts are scanned once
//! (fontique reads them lazily, on the first title), never per frame. A raster is cached per
//! [`TitleKey`], a few at a time.

use crate::cache::Bounded;
use crate::error::TitleError;
use crate::ink::{Ink, ink};
use crate::key::TitleKey;
use crate::place::{Margins, left_edge, longest_fit, room};
use casement_core::raster::{RasterId, RasterImage};
use ds_style::tokens::chrome::{TitleFont, title_font};
use parley::{
    FontContext, FontFamily, FontWeight, Layout, LayoutContext, PositionedLayoutItem, StyleProperty,
};
use std::fmt;
use std::sync::Arc;
use vello_cpu::color::{AlphaColor, Srgb};
use vello_cpu::{Level, Pixmap, RenderContext, RenderSettings, Resources};

/// How many title rasters are kept: the titles of the windows on screen, with room for the
/// two ink states of each.
const KEPT: usize = 32;

/// What follows a title cut short.
const ELLIPSIS: char = '\u{2026}';

/// What is tried after the title's own family when it is not installed.
const FALLBACK: &str = "system-ui, sans-serif";

/// Lays out and rasterises window titles. Make one and keep it.
pub struct TitleRaster {
    fonts: FontContext,
    layouts: LayoutContext<()>,
    resources: Resources,
    level: Level,
    font: TitleFont,
    margins: Margins,
    kept: Bounded<TitleKey, RasterImage>,
    made: u64,
}

impl fmt::Debug for TitleRaster {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TitleRaster")
            .field("font", &self.font)
            .field("margins", &self.margins)
            .field("kept", &self.kept.len())
            .finish_non_exhaustive()
    }
}

impl TitleRaster {
    /// A rasteriser for quire's system title font, keeping `margins` clear on each side of the
    /// title.
    ///
    /// ```
    /// use casement_chrome::{Margins, TitleKey, TitleRaster};
    /// use ds_core::geometry::scale::Scale;
    /// use ds_core::vocab::Activity;
    /// use ds_style::appearance::theme::Scheme;
    ///
    /// let mut titles = TitleRaster::system(Margins { left: 73, right: 13 });
    /// let key = TitleKey::new("Notes", 400, 28, Scale(240), Scheme::Light, Activity::Active);
    /// let pixels = titles.render(&key).expect("a title fits");
    /// assert_eq!((pixels.width, pixels.height), (800, 56));
    /// assert_eq!(pixels.rgba.len(), 800 * 56 * 4);
    /// ```
    pub fn system(margins: Margins) -> TitleRaster {
        TitleRaster::with_font(title_font(), margins)
    }

    /// A rasteriser for `font` (a family, a size in logical pixels and a weight).
    pub fn with_font(font: TitleFont, margins: Margins) -> TitleRaster {
        TitleRaster {
            fonts: FontContext::new(),
            layouts: LayoutContext::new(),
            resources: Resources::new(),
            level: Level::try_detect().unwrap_or(Level::baseline()),
            font,
            margins,
            kept: Bounded::new(KEPT),
            made: 0,
        }
    }

    /// The pixels of the title `key` describes: the same ones, cheaply, while the key is kept.
    /// An error means the frame is drawn without a title.
    pub fn render(&mut self, key: &TitleKey) -> Result<RasterImage, TitleError> {
        if let Some(kept) = self.kept.get(key) {
            return Ok(kept);
        }
        let made = self.draw(key)?;
        self.kept.put(key.clone(), made.clone());
        Ok(made)
    }

    /// The text shaped in the title font at `scale`, on one line.
    fn shape(&mut self, text: &str, scale: f32) -> Layout<()> {
        let stack = format!("{}, {FALLBACK}", self.font.family);
        let mut builder = self
            .layouts
            .ranged_builder(&mut self.fonts, text, scale, true);
        builder.push_default(StyleProperty::FontSize(self.font.size_px));
        builder.push_default(StyleProperty::FontWeight(FontWeight::new(f32::from(
            self.font.weight,
        ))));
        builder.push_default(StyleProperty::FontFamily(FontFamily::Source(stack.into())));
        let mut layout = builder.build(text);
        layout.break_all_lines(None);
        layout
    }

    /// The title as it fits `avail` device pixels: whole, or the longest start of it with an
    /// ellipsis after.
    fn fitted(&mut self, title: &str, scale: f32, avail: f32) -> Layout<()> {
        let whole = self.shape(title, scale);
        if whole.width() <= avail {
            return whole;
        }
        let ends: Vec<usize> = title
            .char_indices()
            .map(|(at, _)| at)
            .chain([title.len()])
            .collect();
        let cut = |n: usize| format!("{}{ELLIPSIS}", title[..ends[n]].trim_end());
        let keep = longest_fit(ends.len() - 1, |n| {
            self.shape(&cut(n), scale).width() <= avail
        });
        self.shape(&cut(keep), scale)
    }

    fn draw(&mut self, key: &TitleKey) -> Result<RasterImage, TitleError> {
        let (w, h) = key.device_size();
        let too_large = TitleError::TooLarge {
            width: w,
            height: h,
        };
        let (Ok(pw), Ok(ph)) = (u16::try_from(w), u16::try_from(h)) else {
            return Err(too_large);
        };
        let (left, right) = self.margins.in_device(key.scale);
        let avail = room(w, left, right);
        if pw == 0 || ph == 0 || avail == 0 {
            return Err(TitleError::NoRoom);
        }
        if key.title.is_empty() {
            return Err(TitleError::Empty);
        }
        let scale = key.scale.as_f64() as f32;
        let layout = self.fitted(&key.title, scale, avail as f32);
        // Whole device pixels, so a given scale always rasters the same.
        let x = left_edge(layout.width().ceil() as u32, w, left, right) as f64;
        let y = ((f32::from(ph) - layout.height()) / 2.0).round() as f64;
        let Ink { r, g, b, a } = ink(key.scheme, key.activity);
        let mut ctx = RenderContext::new_with(
            pw,
            ph,
            RenderSettings {
                level: self.level,
                num_threads: 0,
            },
        );
        ctx.set_paint(AlphaColor::<Srgb>::from_rgba8(r, g, b, a));
        ctx.set_transform(vello_cpu::kurbo::Affine::translate((x, y)));
        for line in layout.lines() {
            for item in line.items() {
                let PositionedLayoutItem::GlyphRun(run) = item else {
                    continue;
                };
                let face = run.run();
                ctx.glyph_run(&mut self.resources, face.font())
                    .font_size(face.font_size())
                    .hint(false)
                    .normalized_coords(face.normalized_coords())
                    .fill_glyphs(run.positioned_glyphs().map(|g| vello_cpu::Glyph {
                        id: g.id,
                        x: g.x,
                        y: g.y,
                    }));
            }
        }
        ctx.flush();
        let mut pixmap = Pixmap::new(pw, ph);
        ctx.render(&mut pixmap, &mut self.resources);
        self.made += 1;
        Ok(RasterImage {
            id: RasterId(self.made),
            width: w,
            height: h,
            rgba: Arc::from(pixmap.data_as_u8_slice()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ds_core::geometry::scale::Scale;
    use ds_core::vocab::Activity;
    use ds_style::appearance::theme::Scheme;

    fn engine() -> TitleRaster {
        TitleRaster::system(Margins {
            left: 73,
            right: 13,
        })
    }

    fn key(title: &str, width: u32, scale: u32, activity: Activity) -> TitleKey {
        TitleKey::new(title, width, 28, Scale(scale), Scheme::Light, activity)
    }

    /// Every pixel is premultiplied: no channel above its alpha.
    fn premultiplied(pixels: &RasterImage) -> bool {
        pixels
            .rgba
            .as_chunks::<4>()
            .0
            .iter()
            .all(|px| px[..3].iter().all(|c| *c <= px[3]))
    }

    #[test]
    fn the_raster_is_the_titlebar_at_the_native_scale() {
        let mut titles = engine();
        for (width, scale, want) in [
            (400, 120, (400, 28)),
            (400, 150, (500, 35)),
            (400, 180, (600, 42)),
            (400, 240, (800, 56)),
        ] {
            let pixels = titles
                .render(&key("Notes", width, scale, Activity::Active))
                .expect("fits");
            assert_eq!((pixels.width, pixels.height), want, "at {scale}");
            assert_eq!(pixels.rgba.len(), (want.0 * want.1 * 4) as usize);
            assert!(premultiplied(&pixels), "at {scale}");
        }
    }

    #[test]
    fn a_title_that_cannot_be_drawn_says_why() {
        let mut titles = engine();
        let cases = [
            (
                "empty",
                key("", 400, 120, Activity::Active),
                TitleError::Empty,
            ),
            (
                "blank",
                key(" \n ", 400, 120, Activity::Active),
                TitleError::Empty,
            ),
            (
                "no room",
                key("Notes", 80, 120, Activity::Active),
                TitleError::NoRoom,
            ),
            (
                "no bar",
                key("Notes", 0, 120, Activity::Active),
                TitleError::NoRoom,
            ),
            (
                "too wide",
                key("Notes", 70_000, 120, Activity::Active),
                TitleError::TooLarge {
                    width: 70_000,
                    height: 28,
                },
            ),
        ];
        for (name, key, want) in cases {
            assert_eq!(titles.render(&key).err(), Some(want), "{name}");
        }
    }

    #[test]
    fn a_kept_raster_is_the_same_one_and_a_changed_key_is_a_new_one() {
        let mut titles = engine();
        let first = titles
            .render(&key("Notes", 400, 120, Activity::Active))
            .expect("fits");
        let again = titles
            .render(&key("Notes", 400, 120, Activity::Active))
            .expect("fits");
        assert!(Arc::ptr_eq(&first.rgba, &again.rgba));
        for other in [
            key("Notes!", 400, 120, Activity::Active),
            key("Notes", 401, 120, Activity::Active),
            key("Notes", 400, 180, Activity::Active),
            key("Notes", 400, 120, Activity::Inactive),
        ] {
            let made = titles.render(&other).expect("fits");
            assert_ne!(made.id, first.id);
        }
    }

    #[test]
    fn a_long_title_is_cut_and_stays_off_the_lights_and_the_edge() {
        let mut titles = engine();
        let long = "A very long document title that cannot fit in a narrow window".repeat(3);
        let pixels = titles
            .render(&key(&long, 300, 120, Activity::Active))
            .expect("fits");
        let (left, right) = Margins {
            left: 73,
            right: 13,
        }
        .in_device(Scale(120));
        for row in pixels.rgba.chunks_exact(pixels.width as usize * 4) {
            for (x, px) in row.as_chunks::<4>().0.iter().enumerate() {
                // Two pixels of slack for a glyph's side bearing and its anti-aliased edge.
                let inside = (left as usize - 2..(pixels.width - right) as usize + 2).contains(&x);
                assert!(inside || px[3] == 0, "ink at column {x}");
            }
        }
    }

    #[test]
    fn the_cache_is_bounded() {
        let mut titles = engine();
        for n in 0..(KEPT * 2) {
            titles
                .render(&key(&format!("Window {n}"), 400, 120, Activity::Active))
                .expect("fits");
        }
        assert_eq!(titles.kept.len(), KEPT);
    }
}
