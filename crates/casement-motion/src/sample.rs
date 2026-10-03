//! Sampling: the frame at an instant.

use crate::{FrameScene, MotionBook, Settled};
use casement_core::WindowId;
use casement_core::geometry::LogicalSize;
use casement_core::time::Mono;
use casement_core::window::FrameHint;
use casement_protocols::vocab::Emphasis;
use std::collections::BTreeMap;

/// Each window's committed buffer size: until a client commits its new size, the frame scales
/// the old buffer over the moving rectangle.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CommittedSizes(pub BTreeMap<WindowId, LogicalSize>);

/// How each window is drawn apart from where: its emphasis and its frame (the model's, read
/// at sampling).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WindowLooks(pub BTreeMap<WindowId, WindowLook>);

/// One window's look.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowLook {
    /// Key, main or inactive.
    pub emphasis: Emphasis,
    /// Who draws its frame and its corner radius.
    pub frame: FrameHint,
}

/// The frame at `now`: every window's quantised position, size and opacity grouped by output
/// with each output's strip offset, and whether anything still moves.
pub fn sample(
    book: &MotionBook,
    committed: &CommittedSizes,
    looks: &WindowLooks,
    now: Mono,
) -> (FrameScene, Settled) {
    let _ = (book, committed, looks, now);
    todo!("P0-F1-motion: sample every leg at now and quantise")
}
