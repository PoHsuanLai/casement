//! What the renderer remembers between frames, held by the [`TextureBook`](crate::TextureBook)
//! it is drawn from rather than in globals: each cursor's raster per scale, and the lock
//! cover's element id.

use crate::element::{ShadowKey, ShadowSet, Tiles};
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

/// How many frame rasters the cache holds before it starts again: a window being resized makes
/// a new size every frame.
const FRAME_LIMIT: usize = 64;

/// How many shadow tile sets the cache holds (a set per scale, radius and look: a handful).
const SHADOW_LIMIT: usize = 8;

/// The renderer's per-instance memory. A buffer that lives across frames keeps its element id
/// and commit, so a cursor or a cover that does not change damages nothing.
#[derive(Debug, Clone)]
pub(crate) struct DrawCache {
    /// Each named cursor's raster per scale, made once.
    rasters: RefCell<BTreeMap<(CursorShape, u32), MemoryRenderBuffer>>,
    /// Each server-side frame's rasters, by window, part (0 titlebar, 2 title; the shadow has its own map), scale, size and
    /// look. A window keeps its own buffers so no two elements in a frame share an id.
    frames: RefCell<BTreeMap<FrameKey, MemoryRenderBuffer>>,
    /// Each shadow's tiles, made once per scale, radius and layers (no window size in the key).
    tiles: RefCell<BTreeMap<ShadowKey, Rc<Tiles>>>,
    /// Each window's shadow as buffers, uploaded once from the tiles.
    shadows: RefCell<BTreeMap<(WindowId, ShadowKey), Rc<ShadowSet>>>,
    /// The cover's id: one for every output (each output has its own damage tracker).
    cover: Id,
    /// The id of Mission Control's dimming layer (one for every output).
    dim: Id,
}

impl Default for DrawCache {
    fn default() -> DrawCache {
        DrawCache {
            rasters: RefCell::new(BTreeMap::new()),
            frames: RefCell::new(BTreeMap::new()),
            tiles: RefCell::new(BTreeMap::new()),
            shadows: RefCell::new(BTreeMap::new()),
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
        if let Some(set) = self.shadows.borrow().get(&(window, key)) {
            return Rc::clone(set);
        }
        let tiles = {
            let mut tiles = self.tiles.borrow_mut();
            if tiles.len() >= SHADOW_LIMIT && !tiles.contains_key(&key) {
                tiles.clear();
            }
            Rc::clone(tiles.entry(key).or_insert_with(|| Rc::new(make())))
        };
        let set = Rc::new(ShadowSet::new(tiles));
        let mut shadows = self.shadows.borrow_mut();
        if shadows.len() >= FRAME_LIMIT {
            shadows.clear();
        }
        shadows.insert((window, key), Rc::clone(&set));
        set
    }

    /// The frame raster under `key`, made by `make` the first time.
    pub(crate) fn frame(
        &self,
        key: FrameKey,
        make: impl FnOnce() -> MemoryRenderBuffer,
    ) -> MemoryRenderBuffer {
        let mut frames = self.frames.borrow_mut();
        if frames.len() >= FRAME_LIMIT && !frames.contains_key(&key) {
            frames.clear();
        }
        frames.entry(key).or_insert_with(make).clone()
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
}
