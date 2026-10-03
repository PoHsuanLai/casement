//! The plugged virtual outputs: each with its offscreen target, damage tracker and last
//! presented frame.

use super::{PresentedFrame, VirtualOutputSpec};
use casement_core::OutputName;
use smithay::backend::renderer::damage::OutputDamageTracker;
use std::collections::BTreeMap;

/// One plugged virtual output.
#[derive(Debug)]
pub struct VirtualOutput {
    /// What it was plugged as (with any later changes).
    pub spec: VirtualOutputSpec,
    /// Its damage tracker.
    pub tracker: OutputDamageTracker,
    /// The last frame it presented.
    pub last: Option<PresentedFrame>,
}

/// Every plugged virtual output, by connector name.
#[derive(Debug, Default)]
pub struct OutputSet {
    #[expect(dead_code, reason = "P0-B-headless: the plugged outputs")]
    outputs: BTreeMap<OutputName, VirtualOutput>,
}

impl OutputSet {
    /// No outputs.
    pub fn empty() -> OutputSet {
        OutputSet::default()
    }
}
