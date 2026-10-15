//! The winit backend: casement in a window of the user's own session, for the user's manual
//! feel checks (P0 exit, "nested winit feel"). Agents build it and never run it (§3.2); a
//! binary built with `testing` or `debug-inject` refuses it.
//!
//! The window is one output: its physical size and the host's scale factor are the mode and
//! scale (`winit_spec`), a resize or a scale change is a connector change like a display's.
//! Frames go through the same `FrameSource` the headless backend draws, with the window's GLES
//! renderer, flipped ([`HOST_TRANSFORM`]). The window is one seat with a keyboard and a pointer:
//! its input goes through the generic conversion libinput's events take, into the same
//! [`FedInput`] the tty backend feeds.

use crate::backend::BackendError;
use crate::headless::{ConnectorEvent, FrameSource, VirtualOutputSpec, facts_of};
use crate::input::{DeviceName, FedDevice, FedInput, PollContext, input_kind_of};
use crate::winit_held::HeldInput;
use crate::winit_spec::{HOST_TRANSFORM, physical_of, spec_of_window};
use casement_core::DeviceId;
use casement_core::geometry::PhysicalRect;
use casement_core::input::{DeviceClass, InputEvent, InputKind, Origin};
use casement_core::output::{OutputConfig, OutputPower};
use casement_core::time::VblankSeq;
use casement_render::render_output;
use smithay::backend::input::InputEvent as RawInput;
use smithay::backend::renderer::damage::OutputDamageTracker;
use smithay::backend::renderer::gles::GlesRenderer;
use smithay::backend::winit::{self, WinitEvent, WinitEventLoop, WinitGraphicsBackend, WinitInput};
use smithay::reexports::winit::dpi::LogicalSize;
use smithay::reexports::winit::event_loop::pump_events::PumpStatus;
use smithay::reexports::winit::window::WindowAttributes;
use smithay::utils::{Physical, Rectangle, Size};
use std::fmt;
use std::ops::ControlFlow;

/// The window's keyboard.
const KEYBOARD: DeviceId = DeviceId(1);

/// The window's pointer.
const POINTER: DeviceId = DeviceId(2);

/// What a read of the window's events found.
#[derive(Debug)]
#[non_exhaustive]
pub struct WindowTurn {
    /// `Break` once the user closed the window.
    pub flow: ControlFlow<()>,
    /// The seat's devices that appeared and the input events, in order.
    pub fed: Vec<FedInput>,
    /// The output's changes (a resize, a new scale factor).
    pub connectors: Vec<ConnectorEvent>,
}

/// The nested window, its renderer and its input.
pub struct WinitBackend {
    /// The window's output: the requested spec with the window's real size and scale.
    output: VirtualOutputSpec,
    graphics: Box<WinitGraphicsBackend<GlesRenderer>>,
    events: Box<WinitEventLoop>,
    tracker: Box<OutputDamageTracker>,
    /// Frames presented so far.
    presented: u64,
    /// The connector event of the window's output, until it is reported.
    announce: Option<ConnectorEvent>,
    /// The seat's devices, until they are reported.
    devices: Vec<FedInput>,
    /// Keys and buttons held, released when the window loses focus.
    held: HeldInput,
    /// Whether the host asked for the next frame: a swap before it blocks until it does, and a
    /// hidden window is never asked.
    paced: Paced,
}

/// The host's pacing of the window's frames.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Paced {
    /// A frame was shown; the next waits for the host's redraw.
    Waiting,
    /// The host asked for a frame.
    Granted,
}

impl fmt::Debug for WinitBackend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "WinitBackend({})", self.output.name.as_str())
    }
}

impl WinitBackend {
    /// Opens the window at `output`'s size and scale; the output it announces is the window's
    /// own size and scale, which the host may have changed.
    pub fn start(requested: VirtualOutputSpec) -> Result<WinitBackend, BackendError> {
        let scale = f64::from(requested.scale.0) / 120.0;
        let logical = LogicalSize::new(
            f64::from(requested.mode.size.w) / scale,
            f64::from(requested.mode.size.h) / scale,
        );
        let attributes = WindowAttributes::default()
            .with_surface_size(logical)
            .with_title("casement")
            .with_visible(true);
        let (graphics, events) =
            winit::init_from_attributes::<GlesRenderer>(attributes).map_err(BackendError::from)?;
        let output = spec_of_window(&requested, &graphics);
        let announce = Some(ConnectorEvent::Plugged(facts_of(&output)));
        Ok(WinitBackend {
            tracker: Box::new(tracker_of(&output)),
            output,
            graphics: Box::new(graphics),
            events: Box::new(events),
            presented: 0,
            announce,
            devices: seat_devices(),
            held: HeldInput::new(),
            paced: Paced::Granted,
        })
    }

    /// The window's output as a plugged connector, once.
    pub fn announce(&mut self) -> Vec<ConnectorEvent> {
        self.announce.take().into_iter().collect()
    }

    /// The configuration the window's output has: its size and scale, which the model must not
    /// choose for itself.
    pub fn config(&self) -> OutputConfig {
        OutputConfig {
            mode: self.output.mode,
            scale: self.output.scale,
            transform: self.output.transform,
            position: self.output.position,
            power: OutputPower::On,
        }
    }

    /// The connector name of the window's output.
    pub fn output_name(&self) -> &casement_core::OutputName {
        &self.output.name
    }

    /// Reads the window's events: a close ends the run, a resize changes the output, input is
    /// converted (an absolute position over `context.extent`, stamped `context.now`).
    pub fn dispatch(&mut self, context: &PollContext) -> WindowTurn {
        let mut seen = Vec::new();
        let status = self.events.dispatch_new_events(|event| seen.push(event));
        let mut turn = WindowTurn {
            flow: ControlFlow::Continue(()),
            fed: std::mem::take(&mut self.devices),
            connectors: Vec::new(),
        };
        for event in seen {
            self.take(event, context, &mut turn);
        }
        if matches!(status, PumpStatus::Exit(_)) {
            turn.flow = ControlFlow::Break(());
        }
        turn
    }

    /// One window event into `turn`.
    fn take(&mut self, event: WinitEvent, context: &PollContext, turn: &mut WindowTurn) {
        match event {
            WinitEvent::CloseRequested => turn.flow = ControlFlow::Break(()),
            WinitEvent::Resized { size, scale_factor } => {
                turn.connectors.extend(self.resized(size, scale_factor));
            }
            WinitEvent::Focus(false) => {
                let released = self.held.release_all();
                turn.fed.extend(
                    released
                        .into_iter()
                        .filter_map(|kind| stamped(kind, context)),
                );
            }
            WinitEvent::Input(raw) => turn.fed.extend(self.converted(&raw, context)),
            WinitEvent::Redraw => self.paced = Paced::Granted,
            _ => {}
        }
    }

    /// The output's change when the window is now `size` at `scale_factor`; none when nothing
    /// differs or the window has no area (minimised).
    fn resized(&mut self, size: Size<i32, Physical>, scale_factor: f64) -> Option<ConnectorEvent> {
        let size = physical_of(size);
        let next = crate::winit_spec::window_spec(&self.output, size, scale_factor);
        let area = size.w > 0 && size.h > 0;
        let changed = next.mode != self.output.mode || next.scale != self.output.scale;
        (area && changed).then(|| {
            *self.tracker = tracker_of(&next);
            self.output = next;
            ConnectorEvent::Changed(facts_of(&self.output))
        })
    }

    /// `raw` as the seat's event; none for what has no conversion (a touch, the backend's own).
    fn converted(&mut self, raw: &RawInput<WinitInput>, context: &PollContext) -> Option<FedInput> {
        let kind = input_kind_of(raw, context.extent)?;
        let fed = stamped(kind.clone(), context)?;
        self.held.note(&kind);
        Some(fed)
    }

    /// Draws `source` into the window and swaps; the number of the frame shown. None while the
    /// host has not asked for a frame: swapping then would block the whole loop (a hidden window
    /// is never asked).
    pub fn present(&mut self, source: &dyn FrameSource) -> Result<Option<VblankSeq>, BackendError> {
        if self.paced == Paced::Waiting {
            return Ok(None);
        }
        let age = self.graphics.buffer_age().unwrap_or(0);
        let damage = {
            let (renderer, mut framebuffer) = self.graphics.bind().map_err(BackendError::from)?;
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
        self.paced = Paced::Waiting;
        self.graphics.window().request_redraw();
        self.presented = self.presented.saturating_add(1);
        Ok(Some(VblankSeq(self.presented)))
    }
}

/// The damage tracker of `output`'s window: its size and scale, drawn flipped.
fn tracker_of(output: &VirtualOutputSpec) -> OutputDamageTracker {
    let size = (
        i32::try_from(output.mode.size.w).unwrap_or(i32::MAX),
        i32::try_from(output.mode.size.h).unwrap_or(i32::MAX),
    );
    let scale = f64::from(output.scale.0) / 120.0;
    OutputDamageTracker::new(size, scale, HOST_TRANSFORM)
}

/// The window's keyboard and pointer as devices that appeared.
fn seat_devices() -> Vec<FedInput> {
    let device = |id, class, name: &str| {
        FedInput::Appeared(FedDevice::new(id, class, DeviceName(name.to_owned())))
    };
    vec![
        device(KEYBOARD, DeviceClass::Keyboard, "casement window keyboard"),
        device(POINTER, DeviceClass::Pointer, "casement window pointer"),
    ]
}

/// The device and class that produce `kind`; none for a touch (the window announces none).
fn device_of(kind: &InputKind) -> Option<(DeviceId, DeviceClass)> {
    match kind {
        InputKind::Key { .. } => Some((KEYBOARD, DeviceClass::Keyboard)),
        InputKind::Touch(_) => None,
        _ => Some((POINTER, DeviceClass::Pointer)),
    }
}

/// `kind` as the seat's event, stamped; none when no window device produces it.
fn stamped(kind: InputKind, context: &PollContext) -> Option<FedInput> {
    let (device, class) = device_of(&kind)?;
    Some(FedInput::Event(InputEvent {
        origin: Origin::Physical(class),
        device,
        at: context.now,
        kind,
    }))
}

/// `rect` as smithay's.
fn smithay_rect(rect: &PhysicalRect) -> Rectangle<i32, Physical> {
    let side = |n: u32| i32::try_from(n).unwrap_or(i32::MAX);
    Rectangle::new((rect.x, rect.y).into(), (side(rect.w), side(rect.h)).into())
}
