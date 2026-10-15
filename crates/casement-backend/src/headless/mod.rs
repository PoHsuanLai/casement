//! The headless backend: virtual outputs rendered offscreen, a vblank source on the session
//! clock, a render-cost model, and [`crate::VirtualInput`] on the same input path libinput
//! takes. What every test and every jailed run uses (11 §2).

mod bare;
mod cost;
mod gamma;
mod input;
mod output_set;
mod renderer;
mod vblank;
mod virtual_output;

pub use cost::RenderCostModel;
pub use gamma::GAMMA_SIZE;
pub use input::HeadlessInput;
#[cfg(feature = "tty")]
pub(crate) use output_set::transform_of;
pub use output_set::{OutputSet, VirtualOutput};
pub use renderer::{HeadlessRenderer, SoftRenderer};
pub use vblank::VblankSource;
pub use virtual_output::{EdidSource, FixtureName, GpuId, SpecError, VirtualOutputSpec};

use crate::backend::BackendError;
use crate::input::VirtualInput;
use bare::BareLayer;
use casement_core::geometry::{LogicalPoint, PhysicalRect, PhysicalSize};
use casement_core::output::{
    EdidFacts, EdidSerial, Mode, OutputConfig, OutputFacts, OutputPower, PhysicalMm,
};
use casement_core::time::{Mono, VblankSeq};
use casement_core::{OutputKey, OutputName};
use casement_render::CasementElement;
use ds_core::geometry::scale::Scale;
use renderer::{Canvas, Drawn};
use smithay::backend::renderer::gles::GlesRenderer;
use smithay::backend::renderer::pixman::PixmanRenderer;
#[cfg(feature = "testing")]
use smithay::backend::renderer::test::DummyRenderer;

/// Whether clients are offered dma-buf (`--dmabuf none|soft`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DmabufMode {
    /// No `zwp_linux_dmabuf_v1` global: clients fall back to shm (shell-host F80).
    None,
    /// A fake main device over `/dev/udmabuf`.
    Soft,
}

/// How the headless backend starts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeadlessConfig {
    /// The outputs present at start.
    pub outputs: Vec<VirtualOutputSpec>,
    /// The renderer.
    pub renderer: SoftRenderer,
    /// Whether dma-buf is offered.
    pub dmabuf: DmabufMode,
    /// What a frame costs.
    pub cost: RenderCostModel,
}

/// What a frame is drawn from: the elements for each renderer the backend can run (smithay's
/// element types are generic over the renderer, so each gets its own method and the trait stays
/// object-safe). The composition root implements it once, generically, for all three.
pub trait FrameSource {
    /// The elements for pixman, front first.
    fn pixman(&self, renderer: &mut PixmanRenderer) -> Vec<CasementElement<PixmanRenderer>>;
    /// The elements for GLES on llvmpipe, front first.
    fn gles(&self, renderer: &mut GlesRenderer) -> Vec<CasementElement<GlesRenderer>>;
    /// The elements for smithay's dummy renderer, front first.
    #[cfg(feature = "testing")]
    fn dummy(&self, renderer: &mut DummyRenderer) -> Vec<CasementElement<DummyRenderer>>;
}

/// What presenting a frame did.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Shown {
    /// The output presented a frame (its [`HeadlessBackend::last_frame`] moved).
    Frame,
    /// The output is dark, or not plugged: nothing was presented.
    Dark,
}

/// The headless backend.
#[derive(Debug)]
pub struct HeadlessBackend {
    outputs: OutputSet,
    renderer: HeadlessRenderer,
    input: VirtualInput,
    cost: RenderCostModel,
    /// Connector events of the outputs plugged at start, not yet reported.
    announce: Vec<ConnectorEvent>,
    /// Frame callbacks the composition root reported sending.
    callbacks: u64,
}

impl HeadlessBackend {
    /// Starts the renderer and plugs `config`'s outputs (reported through [`HeadlessBackend::
    /// announce`] by the caller, so start-up and hotplug are one code path).
    pub fn start(config: HeadlessConfig) -> Result<HeadlessBackend, BackendError> {
        let renderer = HeadlessRenderer::start(config.renderer)?;
        let mut backend = HeadlessBackend {
            outputs: OutputSet::empty(),
            renderer,
            input: VirtualInput::empty(),
            cost: config.cost,
            announce: Vec::new(),
            callbacks: 0,
        };
        let plugged: Vec<ConnectorEvent> = config
            .outputs
            .into_iter()
            .map(|spec| backend.plug(spec))
            .collect();
        backend.announce = plugged;
        Ok(backend)
    }

    /// The connector events of the outputs plugged at start, once (then none).
    pub fn announce(&mut self) -> Vec<ConnectorEvent> {
        std::mem::take(&mut self.announce)
    }

    /// Plugs a virtual output: the same event a KMS connector gives. Plugging a connector
    /// that is already plugged replaces it ([`ConnectorEvent::Changed`]).
    pub fn plug(&mut self, spec: VirtualOutputSpec) -> ConnectorEvent {
        let facts = facts_of(&spec);
        let canvas = self.canvas_for(&spec);
        let name = spec.name.clone();
        let known = self
            .outputs
            .insert(name, VirtualOutput::new(spec, canvas))
            .is_some();
        match known {
            true => ConnectorEvent::Changed(facts),
            false => ConnectorEvent::Plugged(facts),
        }
    }

    /// Unplugs `name` (an output that is not plugged reports the key it would have had).
    pub fn unplug(&mut self, name: &OutputName) -> ConnectorEvent {
        let key = match self.outputs.remove(name) {
            Some(gone) => facts_of(&gone.spec).key,
            None => OutputKey::of(&synthetic_edid(), name),
        };
        ConnectorEvent::Unplugged(key)
    }

    /// Changes `name`'s mode, scale, position or power ([`ConnectorEvent::Changed`]; an output
    /// that is not plugged reports [`ConnectorEvent::Unplugged`]).
    pub fn set(&mut self, name: &OutputName, change: OutputChange) -> ConnectorEvent {
        let Some(output) = self.outputs.remove(name) else {
            return ConnectorEvent::Unplugged(OutputKey::of(&synthetic_edid(), name));
        };
        let power = match change {
            OutputChange::Power(power) => power,
            _ => output.power,
        };
        let spec = respec(&output.spec, change);
        let facts = facts_of(&spec);
        let output = self.refitted(output, spec);
        self.outputs
            .insert(name.clone(), VirtualOutput { power, ..output });
        ConnectorEvent::Changed(facts)
    }

    /// Applies a whole configuration to `name` at once (mode, scale, transform, position,
    /// power): what `Effect::ApplyOutput` asks. An output that is not plugged is left alone.
    pub fn apply(&mut self, name: &OutputName, config: &OutputConfig) -> ConnectorEvent {
        let Some(output) = self.outputs.remove(name) else {
            return ConnectorEvent::Unplugged(OutputKey::of(&synthetic_edid(), name));
        };
        let spec = VirtualOutputSpec {
            mode: config.mode,
            scale: config.scale,
            transform: config.transform,
            position: config.position,
            ..output.spec.clone()
        };
        let facts = facts_of(&spec);
        let output = self.refitted(output, spec);
        self.outputs.insert(
            name.clone(),
            VirtualOutput {
                power: config.power,
                ..output
            },
        );
        ConnectorEvent::Changed(facts)
    }

    /// The last frame `name` presented (screenshots and capture).
    pub fn last_frame(&self, name: &OutputName) -> Option<&PresentedFrame> {
        self.outputs
            .get(name)
            .and_then(|output| output.last.as_ref())
    }

    /// Draws `source` alone into a fresh image of `size` at `scale` and reads it back: a window's
    /// pixels without whatever overlaps it, on or off any screen (window capture).
    pub fn render_offscreen(
        &mut self,
        size: PhysicalSize,
        scale: Scale,
        source: &dyn FrameSource,
    ) -> Result<RgbaFrame, BackendError> {
        let bytes = self.renderer.offscreen(size, scale, source)?;
        Ok(RgbaFrame { size, bytes })
    }

    /// The virtual input devices.
    pub fn input(&mut self) -> &mut VirtualInput {
        &mut self.input
    }

    /// The plugged outputs.
    pub fn outputs(&self) -> &OutputSet {
        &self.outputs
    }

    /// What a frame costs, when the model says (the frame's number counts every output's
    /// presented frames).
    pub fn render_cost(&self, frame: u64) -> Option<std::time::Duration> {
        self.cost.cost_of(frame)
    }

    /// Frame callbacks sent so far, as the composition root reported them.
    pub fn callbacks_sent(&self) -> u64 {
        self.callbacks
    }

    /// The composition root sent `n` frame callbacks.
    pub fn note_callbacks(&mut self, n: u64) {
        self.callbacks = self.callbacks.saturating_add(n);
    }

    /// How many frames every output has presented.
    pub fn frames_presented(&self) -> u64 {
        self.outputs
            .iter()
            .map(|(_, output)| output.presented)
            .sum()
    }

    /// Draws `source` into `name`'s image and records it as the frame presented at vblank `seq`
    /// of the session clock's instant `at`. A dark or unplugged output presents nothing.
    pub fn present(
        &mut self,
        name: &OutputName,
        seq: VblankSeq,
        at: Mono,
        source: &dyn FrameSource,
    ) -> Result<Shown, BackendError> {
        let renderer = &mut self.renderer;
        let Some(output) = self.outputs.get_mut(name) else {
            return Ok(Shown::Dark);
        };
        if output.power == OutputPower::Off {
            return Ok(Shown::Dark);
        }
        let size = output.spec.mode.size;
        let drawn = match output.canvas.as_mut() {
            Some(canvas) => {
                let age = canvas.age();
                let drawn = renderer.draw(canvas, &mut output.tracker, source, age)?;
                canvas.mark_drawn();
                drawn
            }
            None => Drawn::Nothing(Vec::new()),
        };
        // The same frame again without the cursor, for the captures that do not ask for it.
        if output.bare.is_none() {
            output.bare = BareLayer::start(renderer, &output.spec);
        }
        let bare = match output.bare.as_mut() {
            Some(layer) => layer.draw(renderer, source).unwrap_or_else(|error| {
                tracing::warn!(output = name.as_str(), %error, "the bare frame was not drawn");
                Drawn::Unchanged
            }),
            None => Drawn::Unchanged,
        };
        let bare = match bare {
            Drawn::Pixels(bytes, _) => Some(RgbaFrame { size, bytes }),
            _ => None,
        };
        output.presented = output.presented.saturating_add(1);
        let (pixels, damage) = match drawn {
            Drawn::Pixels(bytes, damage) => {
                (Some(FramePixels::Rgba(RgbaFrame { size, bytes })), damage)
            }
            Drawn::Nothing(damage) => (Some(FramePixels::Dummy), damage),
            Drawn::Unchanged => (None, Vec::new()),
        };
        output.last = Some(match (pixels, output.last.take()) {
            (Some(pixels), last) => PresentedFrame {
                seq,
                at,
                pixels,
                damage,
                bare: bare.or_else(|| last.and_then(|last| last.bare)),
            },
            (None, Some(last)) => PresentedFrame {
                seq,
                at,
                pixels: last.pixels,
                damage,
                bare: bare.or(last.bare),
            },
            (None, None) => PresentedFrame {
                seq,
                at,
                pixels: FramePixels::Dummy,
                damage,
                bare,
            },
        });
        Ok(Shown::Frame)
    }

    /// A canvas for `spec`'s mode; a renderer fault is logged and the output draws blank.
    fn canvas_for(&mut self, spec: &VirtualOutputSpec) -> Option<Canvas> {
        match self.renderer.canvas(spec.mode.size) {
            Ok(canvas) => Some(canvas),
            Err(error) => {
                tracing::warn!(output = spec.name.as_str(), %error, "no canvas for the output");
                None
            }
        }
    }

    /// `output` with `spec`; a canvas and tracker are made anew only when the mode, scale or
    /// transform changed (a move or a power change keeps the last frame).
    fn refitted(&mut self, output: VirtualOutput, spec: VirtualOutputSpec) -> VirtualOutput {
        let same_picture = output.spec.mode.size == spec.mode.size
            && output.spec.scale == spec.scale
            && output.spec.transform == spec.transform;
        match same_picture {
            true => VirtualOutput { spec, ..output },
            false => {
                let canvas = self.canvas_for(&spec);
                output.respecified(spec, canvas)
            }
        }
    }
}

/// The spec after `change`.
fn respec(spec: &VirtualOutputSpec, change: OutputChange) -> VirtualOutputSpec {
    let spec = spec.clone();
    match change {
        OutputChange::Mode(mode) => VirtualOutputSpec { mode, ..spec },
        OutputChange::Scale(scale) => VirtualOutputSpec { scale, ..spec },
        OutputChange::Position(position) => VirtualOutputSpec { position, ..spec },
        OutputChange::Power(_) => spec,
    }
}

/// The EDID of an output nobody described: no serial, so its key is its connector.
fn synthetic_edid() -> EdidFacts {
    EdidFacts {
        make: "Casement".to_owned(),
        model: "Virtual".to_owned(),
        serial: EdidSerial::Unknown,
    }
}

/// What `spec`'s display reports about itself.
pub fn facts_of(spec: &VirtualOutputSpec) -> OutputFacts {
    let edid = match &spec.edid {
        EdidSource::Synthetic(facts) => facts.clone(),
        EdidSource::Recorded(fixture) => EdidFacts {
            make: "Recorded".to_owned(),
            model: fixture.0.clone(),
            serial: EdidSerial::Known(fixture.0.clone()),
        },
    };
    OutputFacts {
        name: spec.name.clone(),
        key: OutputKey::of(&edid, &spec.name),
        edid,
        modes: vec![spec.mode],
        physical: PhysicalMm { w: 0, h: 0 },
        vrr: spec.vrr,
    }
}

/// A change to a plugged output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputChange {
    /// A new mode.
    Mode(Mode),
    /// A new scale.
    Scale(Scale),
    /// A new position in the global logical space.
    Position(LogicalPoint),
    /// Powered on or off.
    Power(OutputPower),
}

/// What a connector did: the one event shape headless and KMS both produce.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectorEvent {
    /// A display appeared.
    Plugged(OutputFacts),
    /// A display went away.
    Unplugged(OutputKey),
    /// A display's facts changed.
    Changed(OutputFacts),
}

/// One frame an output presented.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PresentedFrame {
    /// The vblank it was shown at.
    pub seq: VblankSeq,
    /// When.
    pub at: Mono,
    /// Its pixels, with the cursor drawn.
    pub pixels: FramePixels,
    /// What changed since the frame before, in the frame's pixels (empty when nothing did: the
    /// scene was still).
    pub damage: Vec<PhysicalRect>,
    /// The same pixels without the cursor, once the output has drawn a frame and the renderer
    /// reads frames back.
    pub bare: Option<RgbaFrame>,
}

/// A presented frame's pixels. Every renderer reads back `Abgr8888` (RGBA byte order), so
/// pixman and llvmpipe frames compare byte for byte (SP2). (The plan's sketch held renderer
/// images; a read-back is what screenshots and goldens use: FINDINGS.)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FramePixels {
    /// Read back from pixman or GLES.
    Rgba(RgbaFrame),
    /// The dummy renderer draws nothing.
    Dummy,
}

/// Read-back pixels: rows top to bottom, four bytes per pixel in R, G, B, A order,
/// premultiplied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RgbaFrame {
    /// The size in physical pixels.
    pub size: PhysicalSize,
    /// `size.w * size.h * 4` bytes.
    pub bytes: Vec<u8>,
}
