//! The icons of the windows on show, found when Mission Control opens and dropped when it closes.

use super::IconSearch;
use super::worker::{Job, Worker};
use casement_core::WindowId;
use casement_core::geometry::{LogicalRect, buffer_size};
use casement_core::raster::{RasterId, RasterImage};
use casement_icons::{
    Finder, IconCache, IconKey, Pixels, RgbaBuffer, Slot, SystemTheme, Want, plate,
};
use casement_render::OverviewIcon;
use casement_render::pass::ArgbImage;
use casement_wayland::xdg_toplevel_icon::IconParts;
use ds_core::geometry::scale::Scale;
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

/// A question put to the icon thread whose answer has not come: the window shows its plate
/// meanwhile.
#[derive(Debug)]
struct Waiting {
    /// The question, to find its answer in the cache.
    key: IconKey,
    /// The plate drawn meanwhile, with an id of its own so it is uploaded once.
    stand_in: RasterImage,
}

/// The icons found for the windows of one Mission Control, by window and pixel edge. The files
/// are read on a thread of their own: an icon not found yet is drawn as its plate, and replaces
/// it the frame after it is.
#[derive(Debug)]
pub struct IconBook {
    /// Looks icons up; none when icons are off.
    finder: Option<Finder<SystemTheme>>,
    /// The config folder the desktop's icon theme is named under.
    config: Option<PathBuf>,
    /// Whether Mission Control has been drawn since the book was last cleared: while it is
    /// not, the next icon asked for opens it and the desktop's icon theme is read again; once
    /// it is, the theme read when it opened stands.
    open: bool,
    /// What the lookups found, by question and edge.
    cache: IconCache,
    /// The thread that reads the files.
    worker: Worker,
    /// The questions put to the thread and not answered yet.
    asked: BTreeSet<(IconKey, u32)>,
    /// The windows drawn as their plate until their answer comes.
    waiting: BTreeMap<(WindowId, u32), Waiting>,
    /// The raster each window draws, made from the cache.
    drawn: BTreeMap<(WindowId, u32), RasterImage>,
    /// The icons last drawn, for the frames of the way out: the layout has no slots then, but
    /// the windows still fly and their icons leave with them.
    shown: BTreeMap<WindowId, OverviewIcon>,
}

impl IconBook {
    /// A book that looks for icons as `search` says.
    pub fn new(search: Option<IconSearch>) -> IconBook {
        let (finder, config) = match search {
            None => (None, None),
            Some(IconSearch {
                applications,
                config,
            }) => (
                Some(Finder {
                    applications,
                    theme: SystemTheme::default(),
                }),
                config,
            ),
        };
        IconBook {
            finder,
            config,
            open: false,
            cache: IconCache::new(),
            worker: Worker::default(),
            asked: BTreeSet::new(),
            waiting: BTreeMap::new(),
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

    /// The icon theme lookups use now.
    pub fn theme(&self) -> Option<&SystemTheme> {
        self.finder.as_ref().map(|finder| &finder.theme)
    }

    /// Reads the desktop's icon theme again when Mission Control has just opened, so a theme
    /// changed in the desktop's settings shows at the next opening.
    fn open(&mut self) {
        if self.open {
            return;
        }
        self.open = true;
        if let (Some(finder), Some(config)) = (self.finder.as_mut(), self.config.as_deref()) {
            finder.theme = SystemTheme::of_desktop(config);
        }
    }

    /// Lets go of every icon (Mission Control is closed).
    pub fn clear(&mut self) {
        self.open = false;
        self.cache.clear();
        self.asked.clear();
        self.waiting.clear();
        self.drawn.clear();
        self.shown.clear();
    }

    /// Files the answers the icon thread has given since the last call. An answer to a question
    /// asked in a Mission Control that has closed since is dropped.
    fn collect(&mut self) {
        for found in self.worker.answers() {
            if self.asked.remove(&(found.key.clone(), found.edge)) {
                self.cache.fill(found.key, found.edge, found.pixels);
            }
        }
    }

    /// Puts `want` to the icon thread, once. A thread that cannot be reached answers "no icon"
    /// at once, so the plate stays and the question is not asked again.
    fn ask(&mut self, want: Want, key: &IconKey, edge: u32) {
        let Some(finder) = self.finder.clone() else {
            return;
        };
        if self.cache.get(key, edge).is_some() || !self.asked.insert((key.clone(), edge)) {
            return;
        }
        let job = Job {
            key: key.clone(),
            want,
            edge,
            finder,
        };
        if self.worker.submit(job).is_err() {
            self.asked.remove(&(key.clone(), edge));
            self.cache.fill(key.clone(), edge, None);
        }
    }

    /// The raster `window` draws for an icon of `icon` logical pixels at `scale`. The first time it
    /// is asked for, the icon is looked for on the icon thread (the loop asks when Mission Control
    /// opens, and a window that appears later is asked for when it is first drawn) and the plate
    /// is drawn meanwhile; once it is found the icon replaces the plate and is kept. `want` says
    /// what to find, and is called once per window and edge; `plate_ink` is the straight RGBA of
    /// the plate drawn for an app with no icon. Nothing when icons are off.
    pub fn draw_of(
        &mut self,
        window: WindowId,
        icon: LogicalRect,
        scale: Scale,
        plate_ink: [u8; 4],
        want: impl FnOnce() -> Want,
    ) -> Option<RasterImage> {
        self.finder.as_ref()?;
        self.open();
        self.collect();
        let edge = buffer_size(icon.size, scale).w;
        let at = (window, edge);
        if let Some(kept) = self.drawn.get(&at) {
            return Some(kept.clone());
        }
        let key = match self.waiting.get(&at) {
            Some(waiting) => waiting.key.clone(),
            None => {
                let want = want();
                let key = want.key();
                self.ask(want, &key, edge);
                key
            }
        };
        match self.cache.get(&key, edge) {
            Some(slot) => {
                let draw = draw_of_slot(slot, edge, plate_ink)?;
                self.waiting.remove(&at);
                self.drawn.insert(at, draw.clone());
                Some(draw)
            }
            None => self.stand_in(at, key, edge, plate_ink),
        }
    }

    /// The plate drawn for `at` while its icon is looked for.
    fn stand_in(
        &mut self,
        at: (WindowId, u32),
        key: IconKey,
        edge: u32,
        plate_ink: [u8; 4],
    ) -> Option<RasterImage> {
        if let Some(waiting) = self.waiting.get(&at) {
            return Some(waiting.stand_in.clone());
        }
        let pixels = plate(edge, plate_ink)?;
        let stand_in = raster_of(self.cache.reserve(), pixels);
        self.waiting.insert(
            at,
            Waiting {
                key,
                stand_in: stand_in.clone(),
            },
        );
        Some(stand_in)
    }
}

/// The raster for `slot`: its icon, else the plate in `plate_ink`.
fn draw_of_slot(slot: &Slot, edge: u32, plate_ink: [u8; 4]) -> Option<RasterImage> {
    let pixels = slot.pixels.clone().or_else(|| plate(edge, plate_ink))?;
    Some(raster_of(slot.id, pixels))
}

/// `pixels` as the image the renderer takes, which raster `id` is.
fn raster_of(id: u64, pixels: Pixels) -> RasterImage {
    RasterImage {
        id: RasterId(id),
        width: pixels.edge,
        height: pixels.edge,
        rgba: pixels.rgba,
    }
}

/// The largest edge, in buffer pixels, of an icon buffer that is read: Mission Control draws an
/// icon a hundred pixels across at most, even at a high scale. A larger buffer is not read at
/// all (a client could hand over a window-sized one for every icon of every window).
const ICON_EDGE_LIMIT: u32 = 512;

/// The most buffers read for one window's icon.
const ICON_BUFFER_LIMIT: usize = 8;

/// Which of the icon buffers with these `edges` are read: the first [`ICON_BUFFER_LIMIT`] that are
/// no larger than [`ICON_EDGE_LIMIT`], as positions.
fn admissible(edges: &[u32]) -> Vec<usize> {
    edges
        .iter()
        .enumerate()
        .filter(|(_, edge)| **edge <= ICON_EDGE_LIMIT)
        .map(|(at, _)| at)
        .take(ICON_BUFFER_LIMIT)
        .collect()
}

/// What to find for a window with this `app_id` and the icon its toplevel set (if it did): the
/// toplevel's buffers are read out of shared memory here, once, when the overview opens.
pub fn want_of(app_id: &str, set: Option<IconParts>) -> Want {
    let (name, buffers) = set.map_or((None, Vec::new()), |parts| {
        let edges: Vec<u32> = parts.buffers.iter().map(|held| held.edge).collect();
        let buffers = admissible(&edges)
            .into_iter()
            .filter_map(|at| parts.buffers.get(at))
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
    fn only_small_icon_buffers_are_read_and_only_a_few() {
        assert_eq!(admissible(&[16, 512, 513, 64]), [0, 1, 3]);
        assert_eq!(admissible(&[4096]), Vec::<usize>::new());
        assert_eq!(admissible(&[32; 20]), [0, 1, 2, 3, 4, 5, 6, 7]);
    }

    #[test]
    fn a_book_with_icons_off_draws_nothing() {
        let mut book = IconBook::new(None);
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
        let mut book = IconBook::new(Some(IconSearch {
            applications: Vec::new(),
            config: None,
        }));
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

    #[test]
    fn an_icon_replaces_its_plate_once_the_thread_has_found_it() {
        let mut book = IconBook::new(Some(IconSearch {
            applications: Vec::new(),
            config: None,
        }));
        let red = RgbaBuffer {
            edge: 16,
            rgba: [255, 0, 0, 255].repeat(16 * 16),
        };
        let want = || Want {
            buffers: vec![red.clone()],
            ..Want::default()
        };
        let draw = |book: &mut IconBook| {
            book.draw_of(WindowId(1), icon(), Scale::ONE, [9, 9, 9, 255], want)
                .expect("a draw")
        };
        let plate_first = draw(&mut book);
        // The plate is the same raster until the answer comes.
        assert_eq!(draw(&mut book).id, plate_first.id);
        // Up to ten seconds for the thread; it takes a few milliseconds.
        let found = (0..2000)
            .find_map(|_| {
                let now = draw(&mut book);
                std::thread::sleep(std::time::Duration::from_millis(5));
                (now.id != plate_first.id).then_some(now)
            })
            .expect("the icon never came");
        let middle = (32 * found.width as usize + 32) * 4;
        assert_eq!(&found.rgba[middle..middle + 4], &[255, 0, 0, 255]);
    }

    #[test]
    fn the_theme_is_read_again_each_time_mission_control_opens() {
        let dir = tempfile::tempdir().expect("a scratch folder");
        let file = casement_icons::theme_file(dir.path());
        std::fs::create_dir_all(file.parent().expect("a parent")).expect("the folder");
        let mut book = IconBook::new(Some(IconSearch {
            applications: Vec::new(),
            config: Some(dir.path().to_owned()),
        }));
        let mut theme_after = |text: &str| {
            std::fs::write(&file, text).expect("the file");
            book.draw_of(
                WindowId(1),
                icon(),
                Scale::ONE,
                [1, 2, 3, 255],
                Want::default,
            );
            book.theme().cloned()
        };
        let first = theme_after("gtk-icon-theme-name=One\n");
        // Still open: the change waits for the next opening.
        let same = theme_after("gtk-icon-theme-name=Two\n");
        assert_eq!(first, Some(SystemTheme::new(Some("One".to_owned()))));
        assert_eq!(same, first);
        book.clear();
        std::fs::write(&file, "gtk-icon-theme-name=Two\n").expect("the file");
        book.draw_of(
            WindowId(1),
            icon(),
            Scale::ONE,
            [1, 2, 3, 255],
            Want::default,
        );
        assert_eq!(
            book.theme(),
            Some(&SystemTheme::new(Some("Two".to_owned())))
        );
    }
}
