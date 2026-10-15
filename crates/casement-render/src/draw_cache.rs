//! What the renderer remembers between frames, held by the [`TextureBook`](crate::TextureBook)
//! it is drawn from rather than in globals: each cursor's raster per scale, and the lock
//! cover's element id.

use crate::scene::CursorShape;
use casement_core::WindowId;
use casement_core::window::FrameLook;
use smithay::backend::renderer::element::Id;
use smithay::backend::renderer::element::memory::MemoryRenderBuffer;
use std::cell::RefCell;
use std::collections::BTreeMap;

/// What a frame raster is cached under: window, part, scale numerator, width, height, look.
pub(crate) type FrameKey = (WindowId, u8, u32, u32, u32, FrameLook);

/// How many frame rasters the cache holds before it starts again: a window being resized makes
/// a new size every frame.
const FRAME_LIMIT: usize = 64;

/// The renderer's per-instance memory. A buffer that lives across frames keeps its element id
/// and commit, so a cursor or a cover that does not change damages nothing.
#[derive(Debug, Clone)]
pub(crate) struct DrawCache {
    /// Each named cursor's raster per scale, made once.
    rasters: RefCell<BTreeMap<(CursorShape, u32), MemoryRenderBuffer>>,
    /// Each server-side frame's rasters, by window, part (0 titlebar, 1 shadow), scale, size and
    /// look. A window keeps its own buffers so no two elements in a frame share an id.
    frames: RefCell<BTreeMap<FrameKey, MemoryRenderBuffer>>,
    /// The cover's id: one for every output (each output has its own damage tracker).
    cover: Id,
}

impl Default for DrawCache {
    fn default() -> DrawCache {
        DrawCache {
            rasters: RefCell::new(BTreeMap::new()),
            frames: RefCell::new(BTreeMap::new()),
            cover: Id::new(),
        }
    }
}

impl DrawCache {
    /// The cover's element id.
    pub(crate) fn cover_id(&self) -> Id {
        self.cover.clone()
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
