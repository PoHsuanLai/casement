//! `FakeCaptureGpu`: the display GPU a fake device lends to draw captures, recording what it was
//! asked to draw.

use casement_backend::BackendError;
use casement_backend::tty::CaptureGpu;
use casement_render::RenderError;
use casement_render::pass::CapturedFrame;
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

/// What the fake GPU lists and has been asked.
#[derive(Debug, Default)]
pub struct CaptureGpuWorld {
    /// The formats and modifiers it lists.
    pub formats: Vec<Format>,
    /// What it does when asked to draw.
    pub behaviour: GpuBehaviour,
    /// The format of each buffer it was asked to draw into, in order.
    pub asked: Vec<Format>,
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
}
