//! `FakeCaptureGpu`: the display GPU a fake device lends to draw captures, recording what it was
//! asked to draw.

use casement_backend::BackendError;
use casement_backend::headless::FrameSource;
use casement_backend::tty::{CaptureGpu, Picture, SceneSupport};
use casement_core::OutputName;
use casement_core::geometry::PhysicalRect;
use casement_render::RenderError;
use casement_render::pass::{CapturedFrame, CursorInclusion};
use smithay::backend::allocator::dmabuf::Dmabuf;
use smithay::backend::allocator::{Buffer as _, Format};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

/// What the fake GPU does when asked to draw.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum GpuBehaviour {
    /// It draws.
    #[default]
    Draws,
    /// It refuses every buffer (binding failed).
    Refuses,
}

/// Whether the fake GPU keeps the watched outputs' frames on the GPU.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum SceneBehaviour {
    /// It keeps none: the software shadow reads frames back.
    #[default]
    Absent,
    /// It draws scenes.
    Draws,
    /// It keeps scenes but refuses every draw.
    Refuses,
}

/// What the fake GPU lists and has been asked.
#[derive(Debug, Default)]
pub struct CaptureGpuWorld {
    /// The formats and modifiers it lists.
    pub formats: Vec<Format>,
    /// What it does when asked to draw.
    pub behaviour: GpuBehaviour,
    /// The format of each buffer it was asked to draw into, in order.
    pub asked: Vec<Format>,
    /// Whether it keeps scenes.
    pub scenes: SceneBehaviour,
    /// The outputs it was asked to draw a scene for, in order.
    pub drawn: Vec<OutputName>,
    /// The outputs whose scene it was told to free, in order.
    pub forgotten: Vec<OutputName>,
}

/// A fake capture GPU: a shared handle, so a test keeps one clone to read back.
#[derive(Debug, Clone, Default)]
pub struct FakeCaptureGpu {
    /// The world, shared with every clone.
    pub world: Arc<Mutex<CaptureGpuWorld>>,
}

impl FakeCaptureGpu {
    /// A GPU that lists `formats` and draws.
    pub fn listing(formats: Vec<Format>) -> FakeCaptureGpu {
        FakeCaptureGpu {
            world: Arc::new(Mutex::new(CaptureGpuWorld {
                formats,
                ..CaptureGpuWorld::default()
            })),
        }
    }

    /// The world, locked.
    pub fn world(&self) -> MutexGuard<'_, CaptureGpuWorld> {
        self.world.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl CaptureGpu for FakeCaptureGpu {
    fn formats(&self) -> Vec<Format> {
        self.world().formats.clone()
    }

    fn write_dmabuf(
        &mut self,
        target: &mut Dmabuf,
        frame: &CapturedFrame,
    ) -> Result<(), BackendError> {
        let _ = frame;
        let mut world = self.world();
        world.asked.push(target.format());
        match world.behaviour {
            GpuBehaviour::Draws => Ok(()),
            GpuBehaviour::Refuses => Err(RenderError::NoTarget.into()),
        }
    }

    fn scenes(&self) -> SceneSupport {
        match self.world().scenes {
            SceneBehaviour::Absent => SceneSupport::Absent,
            _ => SceneSupport::Drawn,
        }
    }

    fn draw_scene(
        &mut self,
        output: &OutputName,
        picture: &Picture,
        source: &dyn FrameSource,
    ) -> Result<Vec<PhysicalRect>, BackendError> {
        let _ = source;
        let mut world = self.world();
        world.drawn.push(output.clone());
        match world.scenes {
            SceneBehaviour::Draws => Ok(vec![PhysicalRect {
                x: 0,
                y: 0,
                w: picture.size.w,
                h: picture.size.h,
            }]),
            _ => Err(RenderError::NoTarget.into()),
        }
    }

    fn blit_scene(
        &mut self,
        output: &OutputName,
        cursor: CursorInclusion,
        from: PhysicalRect,
        target: &mut Dmabuf,
    ) -> Result<(), BackendError> {
        let _ = (output, cursor, from);
        self.world().asked.push(target.format());
        Ok(())
    }

    fn forget_scene(&mut self, output: &OutputName) {
        self.world().forgotten.push(output.clone());
    }
}
