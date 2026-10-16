//! What the renderer remembers between frames, held by the [`TextureBook`](crate::TextureBook)
//! it is drawn from rather than in globals: each cursor's raster per scale, and the lock
//! cover's element id.

use crate::element::{Raster, ShadowKey, ShadowSet, Tiles, WholeShadow};
use crate::lru::Lru;
use crate::scene::CursorShape;
use casement_core::WindowId;
use casement_core::window::FrameLook;
use smithay::backend::renderer::element::Id;
use smithay::backend::renderer::element::memory::MemoryRenderBuffer;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

/// What a frame raster is cached under: window, part, scale numerator, width, height, look, and
/// a stamp (a title's raster id; 0 for the parts that depend on nothing else).
pub(crate) type FrameKey = (WindowId, u8, u32, u32, u32, FrameLook, u64);

/// How many frame rasters the cache holds before it drops the one asked for longest ago: a
/// window being resized makes a new size every frame.
const FRAME_LIMIT: usize = 64;

/// How many shadow tile sets the cache holds (a set per scale, radius and look: a handful).
const SHADOW_LIMIT: usize = 8;

/// How many small-window shadow rasters the cache holds (one per size and look; such windows
/// are few).
const WHOLE_LIMIT: usize = 16;

/// How many icon buffers the cache holds: Mission Control shows a few dozen windows.
const ICON_LIMIT: usize = 64;

/// What an icon buffer is cached under: the window, the raster's id and its size in pixels. A
/// window keeps its own buffer, so two windows of one app never share an element id in a frame.
pub(crate) type IconKey = (WindowId, u64, u32, u32);

/// A small window's shadow buffers: the window, the shadow, and its width and height in pixels.
type WholeAt = (WindowId, ShadowKey, u32, u32);

/// The renderer's per-instance memory. A buffer that lives across frames keeps its element id
/// and commit, so a cursor or a cover that does not change damages nothing.
#[derive(Debug, Clone)]
pub(crate) struct DrawCache {
    /// Each named cursor's raster per scale, made once.
    rasters: RefCell<BTreeMap<(CursorShape, u32), MemoryRenderBuffer>>,
    /// Each server-side frame's rasters, by window, part (0 titlebar, 2 title; the shadow has its own map), scale, size and
    /// look. A window keeps its own buffers so no two elements in a frame share an id.
    frames: RefCell<Lru<FrameKey, MemoryRenderBuffer>>,
    /// Each shadow's tiles, made once per scale, radius and layers (no window size in the key).
    tiles: RefCell<Lru<ShadowKey, Rc<Tiles>>>,
    /// Each window's shadow as buffers, uploaded once from the tiles.
    shadows: RefCell<Lru<(WindowId, ShadowKey), Rc<ShadowSet>>>,
    /// Small windows' shadows, made once per size, scale, radius and layers.
    wholes: RefCell<Lru<(ShadowKey, u32, u32), Rc<Raster>>>,
    /// Each small window's shadow as a buffer, uploaded once from its raster.
    whole_sets: RefCell<Lru<WholeAt, Rc<WholeShadow>>>,
    /// Mission Control's app icons as buffers, uploaded once each and dropped when it closes.
    icons: RefCell<Lru<IconKey, MemoryRenderBuffer>>,
    /// The cover's id: one for every output (each output has its own damage tracker).
    cover: Id,
    /// The id of Mission Control's dimming layer (one for every output).
    dim: Id,
}

impl Default for DrawCache {
    fn default() -> DrawCache {
        DrawCache {
            rasters: RefCell::new(BTreeMap::new()),
            frames: RefCell::new(Lru::new(FRAME_LIMIT)),
            tiles: RefCell::new(Lru::new(SHADOW_LIMIT)),
            shadows: RefCell::new(Lru::new(FRAME_LIMIT)),
            wholes: RefCell::new(Lru::new(WHOLE_LIMIT)),
            whole_sets: RefCell::new(Lru::new(FRAME_LIMIT)),
            icons: RefCell::new(Lru::new(ICON_LIMIT)),
            cover: Id::new(),
            dim: Id::new(),
        }
    }
}

impl DrawCache {
    /// The cover's element id.
    pub(crate) fn cover_id(&self) -> Id {
        self.cover.clone()
    }

    /// The dimming layer's element id.
    pub(crate) fn dim_id(&self) -> Id {
        self.dim.clone()
    }

    /// The raster of `shape` at `scale` (the scale's numerator), made by `make` the first time.
    pub(crate) fn raster(
        &self,
        shape: CursorShape,
        scale: u32,
        make: impl FnOnce() -> MemoryRenderBuffer,
    ) -> MemoryRenderBuffer {
        self.rasters
            .borrow_mut()
            .entry((shape, scale))
            .or_insert_with(make)
            .clone()
    }

    /// The shadow of `window` under `key`: the tiles are made by `make` the first time any
    /// window asks, and a window's buffers are built from them once. A window being resized
    /// asks with the same key every frame, so it never rasters again.
    pub(crate) fn shadow(
        &self,
        window: WindowId,
        key: ShadowKey,
        make: impl FnOnce() -> Tiles,
    ) -> Rc<ShadowSet> {
        self.shadows.borrow_mut().get_or_make((window, key), || {
            let tiles = self.tiles.borrow_mut().get_or_make(key, || Rc::new(make()));
            Rc::new(ShadowSet::new(tiles))
        })
    }

    /// The shadow of `window` when it is too small for the tiles: a raster of its `size`, made
    /// by `make` the first time any window of that size and look asks.
    pub(crate) fn whole_shadow(
        &self,
        window: WindowId,
        key: ShadowKey,
        size: (u32, u32),
        make: impl FnOnce() -> Raster,
    ) -> Rc<WholeShadow> {
        let at = (window, key, size.0, size.1);
        self.whole_sets.borrow_mut().get_or_make(at, || {
            let raster = self
                .wholes
                .borrow_mut()
                .get_or_make((key, size.0, size.1), || Rc::new(make()));
            Rc::new(WholeShadow::new(raster))
        })
    }

    /// The icon buffer under `key`, made by `make` the first time.
    pub(crate) fn icon(
        &self,
        key: IconKey,
        make: impl FnOnce() -> MemoryRenderBuffer,
    ) -> MemoryRenderBuffer {
        self.icons.borrow_mut().get_or_make(key, make)
    }

    /// Lets go of every icon buffer (Mission Control is not drawn).
    pub(crate) fn drop_icons(&self) {
        self.icons.borrow_mut().clear();
    }

    /// The frame raster under `key`, made by `make` the first time.
    pub(crate) fn frame(
        &self,
        key: FrameKey,
        make: impl FnOnce() -> MemoryRenderBuffer,
    ) -> MemoryRenderBuffer {
        self.frames.borrow_mut().get_or_make(key, make)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::element::shadow_key;
    use casement_core::window::{FrameColour, FrameLook, LightDraw, ShadowLayer};
    use ds_core::geometry::scale::Scale;

    fn look() -> FrameLook {
        let c = FrameColour {
            r: 0,
            g: 0,
            b: 0,
            a: 255,
        };
        let light = LightDraw {
            fill: c,
            ring: c,
            mark: None,
        };
        FrameLook {
            radius: 4,
            titlebar: c,
            rule: c,
            lights: [light; 3],
            shadow: [ShadowLayer {
                dx: 0,
                dy: 2,
                blur: 4,
                spread: 0,
                colour: c,
            }; 2],
        }
    }

    #[test]
    fn resizing_through_a_hundred_sizes_makes_the_tiles_once() {
        let cache = DrawCache::default();
        let look = look();
        let made = std::cell::Cell::new(0);
        let first = cache.shadow(WindowId(1), shadow_key(&look, Scale::ONE), || {
            made.set(made.get() + 1);
            crate::element::shadow_tiles(&look, Scale::ONE)
        });
        for width in 200..300 {
            // The window's size is not part of the key, so each size finds the same set.
            let _ = width;
            let again = cache.shadow(WindowId(1), shadow_key(&look, Scale::ONE), || {
                made.set(made.get() + 1);
                crate::element::shadow_tiles(&look, Scale::ONE)
            });
            assert!(Rc::ptr_eq(&first, &again));
        }
        assert_eq!(made.get(), 1);
    }

    #[test]
    fn two_windows_share_the_raster_but_not_the_buffers() {
        let cache = DrawCache::default();
        let look = look();
        let made = std::cell::Cell::new(0);
        let make = || {
            made.set(made.get() + 1);
            crate::element::shadow_tiles(&look, Scale::ONE)
        };
        let a = cache.shadow(WindowId(1), shadow_key(&look, Scale::ONE), make);
        let b = cache.shadow(WindowId(2), shadow_key(&look, Scale::ONE), make);
        assert_eq!(made.get(), 1);
        assert!(!Rc::ptr_eq(&a, &b));
    }

    #[test]
    fn an_icon_is_uploaded_once_until_the_overview_closes() {
        use smithay::backend::allocator::Fourcc;
        use smithay::utils::Transform;
        let cache = DrawCache::default();
        let made = std::cell::Cell::new(0);
        let ask = || {
            cache.icon((WindowId(1), 7, 1, 1), || {
                made.set(made.get() + 1);
                MemoryRenderBuffer::from_slice(
                    &[0u8; 4],
                    Fourcc::Argb8888,
                    (1, 1),
                    1,
                    Transform::Normal,
                    None,
                )
            })
        };
        ask();
        ask();
        assert_eq!(made.get(), 1);
        cache.drop_icons();
        ask();
        assert_eq!(made.get(), 2);
    }

    #[test]
    fn a_window_resized_through_a_hundred_sizes_does_not_cost_another_window_its_frame() {
        use smithay::backend::allocator::Fourcc;
        use smithay::utils::Transform;
        let cache = DrawCache::default();
        let look = look();
        let made = std::cell::Cell::new(0);
        let buffer = || {
            MemoryRenderBuffer::from_slice(
                &[0u8; 4],
                Fourcc::Argb8888,
                (1, 1),
                1,
                Transform::Normal,
                None,
            )
        };
        for width in 0..100 {
            // The steady window asks every frame; the resized one asks with a new width.
            cache.frame((WindowId(1), 0, 120, 300, 28, look, 0), || {
                made.set(made.get() + 1);
                buffer()
            });
            cache.frame((WindowId(2), 0, 120, 300 + width, 28, look, 0), buffer);
        }
        assert_eq!(made.get(), 1, "the steady window's frame was never dropped");
    }

    #[test]
    fn a_new_scale_is_a_new_raster() {
        let cache = DrawCache::default();
        let look = look();
        let made = std::cell::Cell::new(0);
        for scale in [Scale::ONE, Scale(240), Scale::ONE] {
            cache.shadow(WindowId(1), shadow_key(&look, scale), || {
                made.set(made.get() + 1);
                crate::element::shadow_tiles(&look, scale)
            });
        }
        assert_eq!(made.get(), 2);
    }

    #[test]
    fn a_small_windows_raster_is_made_once_per_size() {
        let cache = DrawCache::default();
        let look = look();
        let made = std::cell::Cell::new(0);
        let key = shadow_key(&look, Scale::ONE);
        let ask = |window: u64, size: (u32, u32)| {
            cache.whole_shadow(WindowId(window), key, size, || {
                made.set(made.get() + 1);
                Raster {
                    w: size.0,
                    h: size.1,
                    argb: vec![0; (size.0 * size.1 * 4) as usize],
                }
            })
        };
        let a = ask(1, (60, 40));
        let again = ask(1, (60, 40));
        assert!(Rc::ptr_eq(&a, &again));
        let other_window = ask(2, (60, 40));
        assert!(
            !Rc::ptr_eq(&a, &other_window),
            "no two elements share a buffer"
        );
        assert_eq!(made.get(), 1, "the raster is shared by size");
        ask(1, (61, 40));
        assert_eq!(made.get(), 2, "a new size is a new raster");
    }
}
