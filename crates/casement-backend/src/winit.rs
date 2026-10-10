//! The winit backend: casement in a window of the user's own session, for the user's manual
//! feel checks (P0 exit, "nested winit feel"). Agents build it and never run it (§3.2); a
//! binary built with `testing` or `debug-inject` refuses it.
//!
//! The window is one output (`VirtualOutputSpec`, its size and scale); frames go through the
//! same `FrameSource` the headless backend draws, with the window's GLES renderer. Input from
//! the window is not wired at P0: it joins the generic conversion libinput's events take (P1).

use crate::backend::BackendError;
use crate::headless::{ConnectorEvent, FrameSource, VirtualOutputSpec, facts_of};
use casement_core::geometry::PhysicalRect;
use casement_core::time::VblankSeq;
use casement_render::render_output;
use smithay::backend::renderer::damage::OutputDamageTracker;
use smithay::backend::renderer::gles::GlesRenderer;
use smithay::backend::winit::{self, WinitEvent, WinitEventLoop, WinitGraphicsBackend};
use smithay::reexports::winit::dpi::LogicalSize;
use smithay::reexports::winit::event_loop::pump_events::PumpStatus;
use smithay::reexports::winit::window::WindowAttributes;
use smithay::utils::{Physical, Rectangle, Transform};
use std::fmt;
use std::ops::ControlFlow;

/// The nested window, its renderer and its input.
pub struct WinitBackend {
    output: VirtualOutputSpec,
    graphics: Box<WinitGraphicsBackend<GlesRenderer>>,
    events: Box<WinitEventLoop>,
    tracker: Box<OutputDamageTracker>,
    /// Frames presented so far.
    presented: u64,
    /// The connector event of the window's output, until it is reported.
    announce: Option<ConnectorEvent>,
}

impl fmt::Debug for WinitBackend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "WinitBackend({})", self.output.name.as_str())
    }
}

impl WinitBackend {
    /// Opens the window at `output`'s size and scale.
    pub fn start(output: VirtualOutputSpec) -> Result<WinitBackend, BackendError> {
        let scale = f64::from(output.scale.0) / 120.0;
        let logical = LogicalSize::new(
            f64::from(output.mode.size.w) / scale,
            f64::from(output.mode.size.h) / scale,
        );
        let attributes = WindowAttributes::default()
            .with_surface_size(logical)
            .with_title("casement")
            .with_visible(true);
        let (graphics, events) = winit::init_from_attributes::<GlesRenderer>(attributes)
            .map_err(BackendError::from)?;
        let size = graphics.window_size();
        let tracker = OutputDamageTracker::new(size, scale, Transform::Normal);
        let announce = Some(ConnectorEvent::Plugged(facts_of(&output)));
        Ok(WinitBackend {
            output,
            graphics: Box::new(graphics),
            events: Box::new(events),
            tracker: Box::new(tracker),
            presented: 0,
            announce,
        })
    }

    /// The window's output as a plugged connector, once.
    pub fn announce(&mut self) -> Vec<ConnectorEvent> {
        self.announce.take().into_iter().collect()
    }

    /// Reads the window's events: `Break` once the user closed it.
    pub fn dispatch(&mut self) -> ControlFlow<()> {
        let mut closed = false;
        let status = self.events.dispatch_new_events(|event| {
            if let WinitEvent::CloseRequested = event {
                closed = true;
            }
        });
        match (closed, status) {
            (true, _) | (_, PumpStatus::Exit(_)) => ControlFlow::Break(()),
            _ => ControlFlow::Continue(()),
        }
    }

    /// Draws `source` into the window and swaps; the number of the frame shown.
    pub fn present(&mut self, source: &dyn FrameSource) -> Result<Option<VblankSeq>, BackendError> {
        let age = self.graphics.buffer_age().unwrap_or(0);
        let damage = {
            let (renderer, mut framebuffer) = self
                .graphics
                .bind()
                .map_err(BackendError::from)?;
            let elements = source.gles(renderer);
            let rendered = render_output(
                renderer,
                &mut framebuffer,
                &mut self.tracker,
                &elements,
                age,
            )
            .map_err(BackendError::from)?;
            rendered.damage.iter().map(smithay_rect).collect::<Vec<_>>()
        };
        self.graphics
            .submit(Some(&damage))
            .map_err(BackendError::from)?;
        self.presented = self.presented.saturating_add(1);
        Ok(Some(VblankSeq(self.presented)))
    }
}

/// `rect` as smithay's.
fn smithay_rect(rect: &PhysicalRect) -> Rectangle<i32, Physical> {
    let side = |n: u32| i32::try_from(n).unwrap_or(i32::MAX);
    Rectangle::new((rect.x, rect.y).into(), (side(rect.w), side(rect.h)).into())
}
