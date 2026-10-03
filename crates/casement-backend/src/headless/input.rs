//! How the headless backend's input reaches the event loop: the queued
//! [`crate::VirtualInput`] events, drained once per loop turn in the order pushed.

use crate::input::{VirtualInput, VirtualInputEvent};

/// The headless backend's input source (drained by the composition root's loop).
#[derive(Debug, Default)]
pub struct HeadlessInput;

impl HeadlessInput {
    /// Every event queued on `input` since the last drain, oldest first.
    pub fn drain(input: &mut VirtualInput) -> Vec<VirtualInputEvent> {
        let _ = input;
        todo!("P0-B-headless: HeadlessInput::drain")
    }
}
