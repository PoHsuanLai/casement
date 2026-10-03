//! Key and button codes.

/// An XKB keysym: what a key means after the layout (`0x20` is space).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Keysym(pub u32);

/// An evdev key code: which key was pressed, before any layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct KeyCode(pub u32);

/// An evdev button code (`BTN_LEFT` is `0x110`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ButtonCode(pub u32);
