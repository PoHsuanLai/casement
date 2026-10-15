//! The tty backend's dma-buf capture offer (wave-2026-10-15/capgpu): the display GPU's formats and
//! modifiers come first, the software renderer's linear ones follow, and a seat whose devices
//! lend no renderer offers the linear ones alone.

use casement_backend::gpu::{GpuError, GpuHandle, RenderNodes};
use casement_backend::tty::{DevicePath, TtyBackend, TtySeams};
use casement_core::gpu::{DrmNodeId, GpuChoice, GpuFacts};
use casement_testkit::fakes::{
    FakeCaptureGpu, FakeKms, FakeKmsOpener, FakeSession, FakeUdev, SharedSession, SyntheticTopology,
};
use smithay::backend::allocator::{Format, Fourcc, Modifier};
use std::path::PathBuf;

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
