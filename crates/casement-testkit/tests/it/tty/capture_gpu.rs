//! The tty backend's dma-buf capture offer (wave-2026-10-15/capgpu): the display GPU's formats and
//! modifiers come first, the software renderer's linear ones follow, and a seat whose devices
//! lend no renderer offers the linear ones alone.

use casement_backend::FramePixels;
use casement_backend::capture_path::{Import, RendererClass};
use casement_backend::gpu::{GpuError, GpuHandle, RenderNodes};
use casement_backend::headless::FrameSource;
use casement_backend::tty::{DevicePath, TtyBackend, TtySeams, Watch};
use casement_core::OutputName;
use casement_core::geometry::{LogicalPoint, Transform};
use casement_core::gpu::{DrmNodeId, GpuChoice, GpuFacts};
use casement_core::output::{OutputConfig, OutputPower};
use casement_core::time::Mono;
use casement_render::CasementElement;
use casement_testkit::fakes::{
    FakeCaptureGpu, FakeKms, FakeKmsOpener, FakeSession, FakeUdev, SceneBehaviour, SharedSession,
    SyntheticTopology,
};
use ds_core::geometry::scale::Scale;
use smithay::backend::allocator::{Format, Fourcc, Modifier};
use smithay::backend::renderer::gles::GlesRenderer;
use smithay::backend::renderer::pixman::PixmanRenderer;
use smithay::backend::renderer::test::DummyRenderer;
use std::path::PathBuf;
use std::time::Duration;

fn card() -> DevicePath {
    DevicePath(PathBuf::from("/dev/dri/card0"))
}

#[derive(Debug)]
struct NoGpus;

impl RenderNodes for NoGpus {
    fn nodes(&self) -> Vec<GpuFacts> {
        Vec::new()
    }

    fn open(&mut self, node: &DrmNodeId) -> Result<GpuHandle, GpuError> {
        Err(GpuError::UnknownNode(*node))
    }
}

fn format(code: Fourcc, modifier: Modifier) -> Format {
    Format { code, modifier }
}

fn tiled() -> Modifier {
    Modifier::from(0x0100_0000_0000_0001_u64)
}

fn backend(kms: FakeKms) -> TtyBackend {
    let mut opener = FakeKmsOpener::default();
    opener.devices.insert(card(), kms);
    let seams = TtySeams::new(
        Box::new(SharedSession::new(FakeSession::default())),
        Box::new(FakeUdev::scripted(Vec::new()).with_present(vec![card()])),
        Box::new(opener),
        Box::new(NoGpus),
        GpuChoice::Auto,
    );
    TtyBackend::start(seams).expect("start")
}

#[test]
fn the_gpus_formats_come_first_then_the_linear_ones() {
    let argb = Fourcc::Argb8888;
    let kms = FakeKms::with(SyntheticTopology::LaptopPanel.resources());
    kms.world().capture_gpu = Some(FakeCaptureGpu::listing(vec![
        format(argb, tiled()),
        format(argb, Modifier::Linear),
    ]));
    let offered = backend(kms).capture_formats();
    assert_eq!(
        offered,
        [
            format(argb, tiled()),
            format(argb, Modifier::Linear),
            format(Fourcc::Xrgb8888, Modifier::Linear)
        ],
        "the GPU's, then the software renderer's linear ones it has not listed"
    );
}

#[test]
fn a_device_that_lends_no_renderer_offers_the_linear_formats_alone() {
    let kms = FakeKms::with(SyntheticTopology::LaptopPanel.resources());
    let offered = backend(kms).capture_formats();
    assert_eq!(
        offered,
        [
            format(Fourcc::Argb8888, Modifier::Linear),
            format(Fourcc::Xrgb8888, Modifier::Linear)
        ]
    );
}

/// A scene with nothing in it: the fakes never look at it.
#[derive(Debug)]
struct Empty;

impl FrameSource for Empty {
    fn pixman(&self, _: &mut PixmanRenderer) -> Vec<CasementElement<PixmanRenderer>> {
        Vec::new()
    }
    fn gles(&self, _: &mut GlesRenderer) -> Vec<CasementElement<GlesRenderer>> {
        Vec::new()
    }
    fn dummy(&self, _: &mut DummyRenderer) -> Vec<CasementElement<DummyRenderer>> {
        Vec::new()
    }
}

/// A one-panel seat whose device lends a GPU that lists linear ARGB8888 and behaves as
/// `scenes` says, with the panel lit and watched, and `frames` frames presented.
fn watched(scenes: SceneBehaviour, frames: u64) -> (TtyBackend, FakeCaptureGpu, OutputName) {
    let kms = FakeKms::with(SyntheticTopology::LaptopPanel.resources());
    let gpu = FakeCaptureGpu::listing(vec![format(Fourcc::Argb8888, Modifier::Linear)]);
    gpu.world().scenes = scenes;
    kms.world().capture_gpu = Some(gpu.clone());
    let panel = SyntheticTopology::LaptopPanel.resources().connectors[0].clone();
    let config = OutputConfig {
        mode: panel.modes[0],
        scale: Scale::ONE,
        transform: Transform::Normal,
        position: LogicalPoint { x: 0, y: 0 },
        power: OutputPower::On,
    };
    let mut backend = backend(kms);
    assert_eq!(backend.announce().len(), 1, "the panel is announced");
    let at = |n: u64| Mono(Duration::from_millis(n));
    backend.apply(&panel.name, &config, at(0)).expect("lit");
    backend
        .watch(&panel.name, &config, Watch::On)
        .expect("watched");
    for tick in 1..=frames {
        backend
            .present(&panel.name, &Empty, at(tick))
            .expect("a frame");
    }
    (backend, gpu, panel.name)
}

#[test]
fn a_gpu_that_keeps_scenes_holds_the_watched_frame_on_the_gpu() {
    let (backend, gpu, name) = watched(SceneBehaviour::Draws, 4);
    let frame = backend.last_frame(&name).expect("a frame was kept");
    assert!(
        matches!(frame.pixels, FramePixels::Resident(_)),
        "not read back: {:?}",
        frame.pixels
    );
    assert!(!gpu.world().drawn.is_empty(), "the GPU drew the scene");
    let argb = format(Fourcc::Argb8888, Modifier::Linear);
    assert_eq!(
        backend.output_class(&name, argb),
        RendererClass::Gpu(Import::Accepted),
        "the frame is on the GPU, which lists the buffer's format"
    );
    let argb_tiled = format(Fourcc::Argb8888, tiled());
    assert_eq!(
        backend.output_class(&name, argb_tiled),
        RendererClass::Gpu(Import::Refused),
        "a modifier the GPU does not list"
    );
}

#[test]
fn a_gpu_that_refuses_the_scene_leaves_the_software_shadow_to_read_the_frame() {
    let (backend, gpu, name) = watched(SceneBehaviour::Refuses, 4);
    let frame = backend.last_frame(&name).expect("the shadow drew a frame");
    assert!(
        matches!(frame.pixels, FramePixels::Rgba(_)),
        "read back by the shadow: {:?}",
        frame.pixels
    );
    assert!(!gpu.world().drawn.is_empty(), "the GPU was asked first");
    assert_eq!(
        backend.output_class(&name, format(Fourcc::Argb8888, Modifier::Linear)),
        RendererClass::Software,
        "the frame is on the CPU: the read-back path"
    );
}

#[test]
fn a_gpu_without_scenes_is_never_asked_to_draw_one() {
    let (backend, gpu, name) = watched(SceneBehaviour::Absent, 4);
    assert!(gpu.world().drawn.is_empty());
    assert!(matches!(
        backend.last_frame(&name).map(|frame| &frame.pixels),
        Some(FramePixels::Rgba(_))
    ));
}

#[test]
fn an_unwatched_output_frees_its_scene() {
    let (mut backend, gpu, name) = watched(SceneBehaviour::Draws, 2);
    let panel = SyntheticTopology::LaptopPanel.resources().connectors[0].clone();
    let config = OutputConfig {
        mode: panel.modes[0],
        scale: Scale::ONE,
        transform: Transform::Normal,
        position: LogicalPoint { x: 0, y: 0 },
        power: OutputPower::On,
    };
    backend
        .watch(&name, &config, Watch::Off)
        .expect("unwatched");
    assert_eq!(gpu.world().forgotten, [name.clone()]);
    assert!(backend.last_frame(&name).is_none(), "nothing is kept");
}
