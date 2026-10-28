//! The plugged virtual outputs: each with its offscreen target, damage tracker and last
//! presented frame.

use super::bare::BareLayer;
use super::renderer::Canvas;
use super::{PresentedFrame, VirtualOutputSpec};
use casement_core::OutputName;
use casement_core::output::{GammaRamp, OutputPower};
use casement_protocols::vocab::Transform as CoreTransform;
use smithay::backend::renderer::damage::OutputDamageTracker;
use smithay::utils::Transform;
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
    /// Whether it is lit (a dark output presents nothing).
    pub power: OutputPower,
    /// How many frames it has presented.
    pub presented: u64,
    /// The gamma ramp a client last set on it (`zwlr_gamma_control_v1`; the identity curve
    /// until one does). Recorded, never applied to pixels.
    pub gamma: GammaRamp,
    /// The offscreen image it draws into (none when the renderer could not make one: the
    /// output then presents blank frames).
    pub(super) canvas: Option<Canvas>,
    /// The same frames drawn again without the cursor (made with the first frame).
    pub(super) bare: Option<BareLayer>,
}

impl VirtualOutput {
    /// A lit output for `spec` drawing into `canvas`.
    pub(super) fn new(spec: VirtualOutputSpec, canvas: Option<Canvas>) -> VirtualOutput {
        VirtualOutput {
            tracker: tracker_of(&spec),
            spec,
            last: None,
            power: OutputPower::On,
            presented: 0,
            gamma: GammaRamp::Identity,
            canvas,
            bare: None,
        }
    }

    /// The output with `spec` and a fresh canvas and tracker (its mode, scale or transform
    /// changed): the next frame draws in full.
    pub(super) fn respecified(self, spec: VirtualOutputSpec, canvas: Option<Canvas>) -> Self {
        VirtualOutput {
            tracker: tracker_of(&spec),
            spec,
            canvas,
            bare: None,
            last: None,
            ..self
        }
    }
}

/// A damage tracker for `spec`'s mode, scale and transform.
pub(super) fn tracker_of(spec: &VirtualOutputSpec) -> OutputDamageTracker {
    let size = (
        i32::try_from(spec.mode.size.w).unwrap_or(i32::MAX),
        i32::try_from(spec.mode.size.h).unwrap_or(i32::MAX),
    );
    let scale = f64::from(spec.scale.0) / 120.0;
    OutputDamageTracker::new(size, scale, transform_of(spec.transform))
}

/// smithay's name for one of our transforms.
pub(crate) fn transform_of(transform: CoreTransform) -> Transform {
    match transform {
        CoreTransform::Normal => Transform::Normal,
        CoreTransform::R90 => Transform::_90,
        CoreTransform::R180 => Transform::_180,
        CoreTransform::R270 => Transform::_270,
        CoreTransform::Flipped => Transform::Flipped,
        CoreTransform::Flipped90 => Transform::Flipped90,
        CoreTransform::Flipped180 => Transform::Flipped180,
        CoreTransform::Flipped270 => Transform::Flipped270,
    }
}

/// Every plugged virtual output, by connector name.
#[derive(Debug, Default)]
pub struct OutputSet {
    outputs: BTreeMap<OutputName, VirtualOutput>,
}

impl OutputSet {
    /// No outputs.
    pub fn empty() -> OutputSet {
        OutputSet::default()
    }

    /// How many are plugged.
    pub fn len(&self) -> usize {
        self.outputs.len()
    }

    /// Whether none is plugged.
    pub fn is_empty(&self) -> bool {
        self.outputs.is_empty()
    }

    /// The output on `name`.
    pub fn get(&self, name: &OutputName) -> Option<&VirtualOutput> {
        self.outputs.get(name)
    }

    /// The output on `name`, mutably.
    pub fn get_mut(&mut self, name: &OutputName) -> Option<&mut VirtualOutput> {
        self.outputs.get_mut(name)
    }

    /// Every output, by connector name.
    pub fn iter(&self) -> impl Iterator<Item = (&OutputName, &VirtualOutput)> {
        self.outputs.iter()
    }

    /// Plugs `output` on `name`; the one it replaces, if any.
    pub fn insert(&mut self, name: OutputName, output: VirtualOutput) -> Option<VirtualOutput> {
        self.outputs.insert(name, output)
    }

    /// Unplugs `name`.
    pub fn remove(&mut self, name: &OutputName) -> Option<VirtualOutput> {
        self.outputs.remove(name)
    }
}
