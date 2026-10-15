//! The icons of the windows on show, found when Mission Control opens and dropped when it closes.

use super::IconSearch;
use casement_core::WindowId;
use casement_core::geometry::{LogicalRect, buffer_size};
use casement_icons::{Finder, IconCache, RgbaBuffer, SystemTheme, Want, plate};
use casement_render::pass::ArgbImage;
use casement_render::{OverviewIcon, TitleDraw};
use casement_wayland::xdg_toplevel_icon::IconParts;
use ds_core::geometry::scale::Scale;
use std::collections::BTreeMap;

/// The icons found for the windows of one Mission Control, by window and pixel edge.
#[derive(Debug)]
pub struct IconBook {
    /// Looks icons up; none when icons are off.
    finder: Option<Finder<SystemTheme>>,
    /// What the lookups found, by question and edge.
    cache: IconCache,
    /// The raster each window draws, made from the cache.
    drawn: BTreeMap<(WindowId, u32), TitleDraw>,
    /// The icons last drawn, for the frames of the way out: the layout has no slots then, but
    /// the windows still fly and their icons leave with them.
    shown: BTreeMap<WindowId, OverviewIcon>,
}

impl IconBook {
    /// A book that looks for icons as `search` says.
    pub fn new(search: IconSearch) -> IconBook {
        let finder = match search {
            IconSearch::Off => None,
            IconSearch::Dirs(applications) => Some(Finder {
                applications,
                theme: SystemTheme::default(),
            }),
        };
        IconBook {
            finder,
            cache: IconCache::new(),
            drawn: BTreeMap::new(),
            shown: BTreeMap::new(),
        }
    }

    /// Keeps `icons` as the ones last drawn.
    pub fn remember(&mut self, icons: &BTreeMap<WindowId, OverviewIcon>) {
        self.shown
            .extend(icons.iter().map(|(window, icon)| (*window, icon.clone())));
    }

    /// The icons last drawn.
    pub fn remembered(&self) -> &BTreeMap<WindowId, OverviewIcon> {
        &self.shown
    }

    /// Lets go of every icon (Mission Control is closed).
    pub fn clear(&mut self) {
        self.cache.clear();
        self.drawn.clear();
        self.shown.clear();
    }

    /// The raster `window` draws for an icon of `icon` logical pixels at `scale`: found the first
    /// time it is asked for (this reads files, so the loop asks when Mission Control opens, and a
    /// window that appears later is found when it is first drawn) and kept after. `want` says what
    /// to find; `plate_ink` is the straight RGBA of the plate drawn for an app with no icon.
    /// Nothing when icons are off.
    pub fn draw_of(
        &mut self,
        window: WindowId,
        icon: LogicalRect,
        scale: Scale,
        plate_ink: [u8; 4],
        want: impl FnOnce() -> Want,
    ) -> Option<TitleDraw> {
        let finder = self.finder.as_ref()?;
        let edge = buffer_size(icon.size, scale).w;
        if let Some(kept) = self.drawn.get(&(window, edge)) {
            return Some(kept.clone());
        }
        let slot = self.cache.ensure(&want(), edge, finder);
        let pixels = slot.pixels.clone().or_else(|| plate(edge, plate_ink))?;
        let draw = TitleDraw {
            id: slot.id,
            width: pixels.edge,
            height: pixels.edge,
            rgba: pixels.rgba,
        };
        self.drawn.insert((window, edge), draw.clone());
        Some(draw)
    }
}

/// What to find for a window with this `app_id` and the icon its toplevel set (if it did): the
/// toplevel's buffers are read out of shared memory here, once, when the overview opens.
pub fn want_of(app_id: &str, set: Option<IconParts>) -> Want {
    let (name, buffers) = set.map_or((None, Vec::new()), |parts| {
        let buffers = parts
            .buffers
            .iter()
            .filter_map(|held| ArgbImage::from_shm(&held.buffer).ok())
            .filter(|image| image.size.w == image.size.h)
            .filter_map(|image| RgbaBuffer::from_argb(image.size.w, &image.argb))
            .collect();
        (parts.name, buffers)
    });
    Want {
        app_id: Some(app_id.to_owned()).filter(|id| !id.is_empty()),
        name,
        buffers,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use casement_core::geometry::{LogicalPoint, LogicalSize};

    fn icon() -> LogicalRect {
        LogicalRect::new(LogicalPoint { x: 0, y: 0 }, LogicalSize { w: 64, h: 64 })
    }

    #[test]
    fn a_book_with_icons_off_draws_nothing() {
        let mut book = IconBook::new(IconSearch::Off);
        let got = book.draw_of(
            WindowId(1),
            icon(),
            Scale::ONE,
            [1, 2, 3, 255],
            Want::default,
        );
        assert_eq!(got, None);
    }

    #[test]
    fn an_app_without_an_icon_gets_the_plate_at_the_output_scale_and_keeps_it() {
        let mut book = IconBook::new(IconSearch::Dirs(Vec::new()));
        let mut asked = 0;
        for _ in 0..3 {
            let got = book
                .draw_of(WindowId(1), icon(), Scale(240), [9, 9, 9, 255], || {
                    asked += 1;
                    Want::default()
                })
                .expect("a plate");
            assert_eq!((got.width, got.height), (128, 128));
        }
        assert_eq!(asked, 1);
        book.clear();
        let again = book.draw_of(WindowId(1), icon(), Scale(240), [9, 9, 9, 255], || {
            asked += 1;
            Want::default()
        });
        assert!(again.is_some());
        assert_eq!(asked, 2);
    }
}
