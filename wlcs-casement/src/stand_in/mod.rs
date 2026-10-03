//! The stand-in compositor behind the [`Compositor`](crate::Compositor) seam: compositor,
//! subcompositor, shm, xdg-shell, a seat with pointer, keyboard and touch, output with
//! xdg-output, relative pointer and data device. Floating windows at (0, 0), `DummyRenderer`,
//! a 16 ms frame tick. State shape after smithay's `smallvil` (MIT; THIRD-PARTY.md).
//!
//! Not casement: it exists so the wlcs build, the shim and the subsets are proven while the
//! fixture's `start` is still being written. It never claims a casement behaviour.

mod handlers;
mod input;
mod relax;

use crate::adapter::{Flow, WlcsEvent};
use smithay::backend::renderer::damage::OutputDamageTracker;
use smithay::backend::renderer::element::surface::WaylandSurfaceRenderElement;
use smithay::backend::renderer::test::{DummyFramebuffer, DummyRenderer};
use smithay::desktop::{PopupManager, Space, Window, space::render_output};
use smithay::input::{Seat, SeatState};
use smithay::output::{Mode, Output, PhysicalProperties, Subpixel};
use smithay::reexports::calloop::channel::{Channel, Event as ChannelEvent};
use smithay::reexports::calloop::generic::Generic;
use smithay::reexports::calloop::{EventLoop, Interest, Mode as PollMode, PostAction};
use smithay::reexports::wayland_server::{Client, Display, DisplayHandle};
use smithay::wayland::compositor::CompositorState;
use smithay::wayland::output::OutputManagerState;
use smithay::wayland::relative_pointer::RelativePointerManagerState;
use smithay::wayland::selection::data_device::DataDeviceState;
use smithay::wayland::shell::xdg::XdgShellState;
use smithay::wayland::shm::ShmState;
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// The frame tick.
const TICK: Duration = Duration::from_millis(16);

/// The compositor state the smithay handlers are implemented on.
pub struct StandIn {
    dh: DisplayHandle,
    flow: Flow,
    start: Instant,
    space: Space<Window>,
    popups: PopupManager,
    clients: HashMap<i32, Client>,
    compositor: CompositorState,
    xdg: XdgShellState,
    shm: ShmState,
    _outputs: OutputManagerState,
    _relative: RelativePointerManagerState,
    seat_state: SeatState<StandIn>,
    data_device: DataDeviceState,
    seat: Seat<StandIn>,
}

impl std::fmt::Debug for StandIn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("StandIn")
    }
}

impl StandIn {
    fn new(dh: &DisplayHandle) -> StandIn {
        let mut seat_state = SeatState::new();
        let mut seat = seat_state.new_wl_seat(dh, "stand-in");
        let _ = seat.add_keyboard(Default::default(), 200, 25);
        seat.add_pointer();
        seat.add_touch();
        StandIn {
            dh: dh.clone(),
            flow: Flow::Continue,
            start: Instant::now(),
            space: Space::default(),
            popups: PopupManager::default(),
            clients: HashMap::new(),
            compositor: CompositorState::new::<StandIn>(dh),
            xdg: XdgShellState::new::<StandIn>(dh),
            shm: ShmState::new::<StandIn>(dh, vec![]),
            _outputs: OutputManagerState::new_with_xdg_output::<StandIn>(dh),
            _relative: RelativePointerManagerState::new::<StandIn>(dh),
            data_device: DataDeviceState::new::<StandIn>(dh),
            seat_state,
            seat,
        }
    }
}

/// The one output: 800x600 at 60 Hz, at the origin.
fn make_output(dh: &DisplayHandle, space: &mut Space<Window>) -> Output {
    let output = Output::new(
        "stand-in".into(),
        PhysicalProperties {
            size: (0, 0).into(),
            subpixel: Subpixel::Unknown,
            make: "casement".into(),
            model: "wlcs".into(),
            serial_number: "0".into(),
        },
    );
    let _global = output.create_global::<StandIn>(dh);
    let mode = Mode {
        size: (800, 600).into(),
        refresh: 60_000,
    };
    output.change_current_state(Some(mode), None, None, Some((0, 0).into()));
    output.set_preferred(mode);
    space.map_output(&output, (0, 0));
    output
}

/// Runs the stand-in until [`WlcsEvent::Exit`] or the channel closes.
pub fn run(events: Channel<WlcsEvent>) {
    let Ok(mut event_loop) = EventLoop::<StandIn>::try_new() else {
        return;
    };
    let Ok(mut display) = Display::<StandIn>::new() else {
        return;
    };
    let dh = display.handle();
    let mut state = StandIn::new(&dh);

    let handle = event_loop.handle();
    let _ = handle.insert_source(events, |event, _, state: &mut StandIn| match event {
        ChannelEvent::Msg(request) => state.flow = state.serve_one(request),
        ChannelEvent::Closed => state.flow = Flow::Stop,
    });
    // Wake on client traffic; the dispatch itself happens below with the owned Display.
    if let Ok(fd) = display.backend().poll_fd().try_clone_to_owned() {
        let _ = handle.insert_source(
            Generic::new(fd, Interest::READ, PollMode::Level),
            |_, _, _| Ok(PostAction::Continue),
        );
    }

    let output = make_output(&dh, &mut state.space);
    let mut tracker = OutputDamageTracker::from_output(&output);
    let mut renderer = DummyRenderer::default();

    while state.flow == Flow::Continue {
        let _ = render_output::<_, WaylandSurfaceRenderElement<DummyRenderer>, _, _>(
            &output,
            &mut renderer,
            &mut DummyFramebuffer,
            1.0,
            0,
            [&state.space],
            &[],
            &mut tracker,
            [0.0, 0.0, 0.0, 1.0],
        );
        let now = state.start.elapsed();
        for window in state.space.elements() {
            window.send_frame(&output, now, Some(Duration::ZERO), |_, _| {
                Some(output.clone())
            });
        }
        if event_loop.dispatch(Some(TICK), &mut state).is_err() {
            break;
        }
        let _ = display.dispatch_clients(&mut state);
        state.space.refresh();
        state.popups.cleanup();
        let _ = display.flush_clients();
    }
}
