//! Capture into a client's dma-buf (wave-2026-10-15/dmabuf): the GPU path blits the output's own
//! frame, and a renderer that cannot (pixman, or a GL renderer that binds no dma-buf) refuses so
//! the caller reads the frame back. Both fill the buffer with the pixels a shared-memory capture
//! holds. They need `/dev/udmabuf` (bound into the jail); a host without it skips.

use casement_backend::capture_path::{Import, RendererClass};
use casement_backend::gpu::FixedUdmabuf;
use casement_backend::headless::FrameSource;
use casement_backend::{
    DmabufMode, FramePixels, HeadlessBackend, HeadlessConfig, RenderCostModel, SoftRenderer,
    VirtualOutputSpec,
};
use casement_core::OutputName;
use casement_core::geometry::{PhysicalRect, PhysicalSize};
use casement_core::time::{Mono, VblankSeq};
use casement_render::pass::{ArgbImage, CapturedFrame, CursorInclusion};
use casement_render::{CasementElement, Compositor, Rgba8, solid_rect};
use ds_core::geometry::scale::Scale;
use smithay::backend::allocator::dmabuf::Dmabuf;
use smithay::backend::allocator::{Allocator, Format, Fourcc, Modifier};
use smithay::backend::renderer::element::Id;
use smithay::backend::renderer::gles::GlesRenderer;
use smithay::backend::renderer::pixman::PixmanRenderer;
#[cfg(feature = "testing")]
use smithay::backend::renderer::test::DummyRenderer;
use smithay::backend::renderer::utils::CommitCounter;
use std::time::Duration;

const SIDE: u32 = 64;

/// A blue field with a red square in it: opaque, so every renderer agrees on every byte.
#[derive(Debug)]
struct Scene;

fn elements<R: Compositor>() -> Vec<CasementElement<R>> {
    let rect = |x, y, w, h| PhysicalRect { x, y, w, h };
    let paint = |rect, r, g, b| {
        CasementElement::Solid(solid_rect(
            Id::new(),
            rect,
            Rgba8 { r, g, b, a: 255 },
            CommitCounter::default(),
        ))
    };
    vec![
        paint(rect(8, 4, 20, 12), 200, 40, 90),
        paint(rect(0, 0, SIDE, SIDE), 10, 20, 130),
    ]
}

impl FrameSource for Scene {
    fn pixman(&self, _: &mut PixmanRenderer) -> Vec<CasementElement<PixmanRenderer>> {
        elements()
    }
    fn gles(&self, _: &mut GlesRenderer) -> Vec<CasementElement<GlesRenderer>> {
        elements()
    }
    #[cfg(feature = "testing")]
    fn dummy(&self, _: &mut DummyRenderer) -> Vec<CasementElement<DummyRenderer>> {
        elements()
    }
}

fn whole() -> PhysicalRect {
    PhysicalRect {
        x: 0,
        y: 0,
        w: SIDE,
        h: SIDE,
    }
}

fn linear_argb() -> Format {
    Format {
        code: Fourcc::Argb8888,
        modifier: Modifier::Linear,
    }
}

/// A backend with one output that has presented a frame of [`Scene`], and a client buffer for it;
/// none when the host cannot start the renderer or has no `/dev/udmabuf`.
fn rig(renderer: SoftRenderer) -> Option<(HeadlessBackend, OutputName, Dmabuf)> {
    let spec = VirtualOutputSpec::parse("HEADLESS-1:64x64@60000:120:0,0").ok()?;
    let name = spec.name.clone();
    let mut backend = HeadlessBackend::start(HeadlessConfig {
        outputs: vec![spec],
        renderer,
        dmabuf: DmabufMode::None,
        cost: RenderCostModel::Fixed(Duration::ZERO),
    })
    .ok()?;
    backend.announce();
    backend
        .present(&name, VblankSeq(1), Mono(Duration::ZERO), &Scene)
        .expect("a frame");
    let buffer = FixedUdmabuf::new()
        .ok()?
        .create_buffer(SIDE, SIDE, Fourcc::Argb8888, &[Modifier::Linear])
        .ok()?;
    Some((backend, name, buffer))
}

/// What a shared-memory capture of the last frame holds.
fn shm_image(backend: &HeadlessBackend, name: &OutputName) -> ArgbImage {
    let frame = backend.last_frame(name).expect("a frame");
    let FramePixels::Rgba(rgba) = &frame.pixels else {
        panic!("the renderer read the frame back");
    };
    ArgbImage::from_rgba(PhysicalSize { w: SIDE, h: SIDE }, &rgba.bytes)
}

#[test]
fn a_gl_renderer_that_binds_the_buffer_blits_the_frame_with_the_pixels_shm_holds() {
    let Some((mut backend, name, mut buffer)) = rig(SoftRenderer::Llvmpipe) else {
        eprintln!("skipped: no GL renderer or no /dev/udmabuf here");
        return;
    };
    if backend.capture_class(linear_argb()) != RendererClass::Gpu(Import::Accepted) {
        eprintln!("skipped: this GL renderer binds no linear dma-buf");
        return;
    }
    backend
        .blit_output_dmabuf(&name, CursorInclusion::Include, whole(), &mut buffer)
        .expect("the frame is blitted");
    let drawn = backend.read_dmabuf(&buffer).expect("read the buffer back");
    assert_eq!(drawn.argb, shm_image(&backend, &name).argb);
}

#[test]
fn a_software_renderer_refuses_the_direct_draw_and_the_read_back_fills_the_same_pixels() {
    let Some((mut backend, name, mut buffer)) = rig(SoftRenderer::Pixman) else {
        eprintln!("skipped: no /dev/udmabuf here");
        return;
    };
    assert_eq!(
        backend.capture_class(linear_argb()),
        RendererClass::Software,
        "pixman keeps its frames on the CPU"
    );
    assert!(
        backend
            .blit_output_dmabuf(&name, CursorInclusion::Include, whole(), &mut buffer)
            .is_err(),
        "a CPU frame is not blitted"
    );
    assert!(
        backend
            .render_window_dmabuf(
                &mut buffer,
                PhysicalSize { w: SIDE, h: SIDE },
                Scale::ONE,
                &Scene
            )
            .is_err(),
        "nor is a window rendered into the buffer"
    );
    // The fallback: read the frame back, then upload it.
    let frame = backend.last_frame(&name).expect("a frame");
    let FramePixels::Rgba(rgba) = &frame.pixels else {
        panic!("pixman reads the frame back");
    };
    let captured = CapturedFrame::from_rgba(whole(), &rgba.bytes);
    backend
        .write_dmabuf(&mut buffer, &captured)
        .expect("the frame is uploaded");
    let drawn = backend.read_dmabuf(&buffer).expect("read the buffer back");
    assert_eq!(drawn.argb, shm_image(&backend, &name).argb);
}

#[test]
fn an_output_that_has_presented_nothing_has_no_frame_to_blit() {
    let Some((mut backend, _, mut buffer)) = rig(SoftRenderer::Pixman) else {
        eprintln!("skipped: no /dev/udmabuf here");
        return;
    };
    let other = OutputName::parse("HEADLESS-9").expect("a name");
    let error = backend
        .blit_output_dmabuf(&other, CursorInclusion::Include, whole(), &mut buffer)
        .expect_err("nothing to blit");
    assert!(matches!(error, casement_backend::BackendError::NoFrame));
}
