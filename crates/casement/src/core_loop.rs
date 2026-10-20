//! The core loop: `Model::step` → `Relayout` → arrange → retarget → frames.
//!
//! The edge calls [`feed`] with what happened; the model answers with effects, which
//! [`crate::exec::exec`] carries out. Frames are drawn at each vblank ([`frame`]); outputs
//! come and go through [`plug_output`] and [`unplug_output`], the path KMS hotplug takes
//! too; input the backend queued is drained into the model by [`drain_input`].

use self::scene::{SceneSource, empty_frame, layers_on, row_of, sample_scene_at};
use crate::cli::StartMode;
use crate::exec;
use crate::lock_marker::Marked;
use crate::state::State;
use casement_backend::headless::Shown;
use casement_backend::tty::PresentOutcome;
use casement_backend::{BackendKind, ConnectorEvent, HeadlessBackend};
use casement_core::frame::Repeat;
use casement_core::lock::LockEvent;
use casement_core::output::OutputConfig;
use casement_core::settings::CasementSettings;
use casement_core::space::SpaceSeed;
use casement_core::time::{Mono, VblankSeq};
use casement_core::{Cause, Effect, Event, Model, OutputKey, OutputName, WindowId};
use casement_layout::{ConfigureNeed, SceneLayout, spec_of_framed};
use casement_motion::{FrameScene, retarget_driven};
use casement_render::{LayerDraw, OutputView};
use casement_wayland::{commit_timing, fifo, presentation, session_lock, xdg_shell};
use smithay::desktop::PopupManager;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::wayland::compositor::{
    SurfaceAttributes, TraversalAction, with_surface_tree_downward,
};
use std::collections::BTreeSet;

mod agent_gate;
mod bodies;
mod closing;
mod deliver;
mod divider;
mod fingers;
mod frames;
mod grab;
mod idle;
mod ime;
mod input;
mod menu;
pub(crate) mod nested;
mod overview;
mod overview_drag;
mod presence;
mod scene;
mod surfaces;
mod tile;
mod titles;
pub(crate) mod tty;
mod windows;

pub(crate) use bodies::{
    Bodies, before_commit as keep_body_before_commit, before_destroy as keep_body_before_destroy,
};
pub(crate) use closing::Kept;
pub(crate) use divider::DividerPointer;
pub(crate) use frames::Over as FrameOver;
pub(crate) use grab::start as start_grab;
pub(crate) use idle::sync_inhibitors;
pub use presence::Presence;
pub(crate) use surfaces::resend_scale;

/// Steps every event at the clock's now and carries out the effects ([`crate::exec::exec`]);
/// a `Relayout` arranges once per feed (not once per event), retargets motion and asks the
/// outputs for frames. An `Event::Input` (of any origin: the backend's queue, a virtual
/// keyboard) is also told to the idle machine and then delivered to the clients by the reach
/// table, the hit test and the seat, so every input takes the one path.
pub fn feed(state: &mut State, events: Vec<Event>) {
    if events.is_empty() {
        return;
    }
    let now = state.clock.now();
    let outer = crate::timing::enter(state, &events);
    note_settings(state, &events);
    let model = std::mem::replace(&mut state.core.model, standing_in(now));
    let events_seen = events.clone();
    let (model, effects) = events.into_iter().fold(
        (model, Vec::<Effect>::new()),
        |(model, mut effects), event| {
            let (model, more) = model.step(event, now);
            effects.extend(more);
            (model, effects)
        },
    );
    state.core.model = model;
    exec::exec(state, effects);
    crate::timing::hear(state, &events_seen, now);
    surfaces::sync(state, &events_seen);
    crate::handlers::sync_outputs(state);
    fingers::follow(state);
    idle::hear(state, &events_seen, now);
    ime::hear(state, &events_seen);
    menu::hear(state, &events_seen);
    events_seen
        .iter()
        .filter_map(|event| match event {
            Event::Input(input) => Some(input),
            _ => None,
        })
        .for_each(|input| deliver::deliver(state, input));
    deliver::refocus(state);
    presence::update(state);
    crate::timing::leave(state, outer);
}

/// The pointer has not moved but a commit may have changed what is under it (a layer's first
/// buffer, a surface that unmapped); `feed` returns early on no events, so a commit asks for
/// this itself.
pub fn refocus_pointer(state: &mut State) {
    deliver::refocus(state);
}

/// A model to hold the place of the real one while a step has it by value.
pub(super) fn standing_in(now: Mono) -> Model {
    Model::new(
        CasementSettings::defaults(),
        now,
        SpaceSeed::from_entropy([0; 16]),
    )
}

/// New settings reach what is already running, at once: the keyboard's repeat to the clients
/// already bound (`repeat_info`), the pointer's to the devices already present.
fn note_settings(state: &mut State, events: &[Event]) {
    for event in events {
        if let Event::SettingsChanged(settings) = event {
            if let Err(fault) =
                casement_wayland::seat::apply_repeat_info(&state.wl.seat.seat, &settings.keyboard)
            {
                tracing::warn!(%fault, "repeat info did not follow the settings");
            }
            tty::reconfigure_pointers(state, &settings.pointer);
        }
    }
}

/// One frame on `output`: sample motion at the clock's now, build the elements, render,
/// present, send frame callbacks and presentation feedback at the frame's instant.
pub fn frame(state: &mut State, output: &OutputKey) {
    windows::refresh_layers(state);
    presence::update(state);
    let now = state.clock.now();
    let Some(plugged) = state.core.model.outputs().0.get(output) else {
        return;
    };
    let (name, config) = (plugged.facts.name.clone(), plugged.config);
    let sampled = sample_scene_at(state, now);
    let in_scene: Vec<WindowId> = sampled
        .outputs
        .iter()
        .flat_map(|frame| frame.windows.iter().map(|placed| placed.window))
        .collect();
    let frame = sampled
        .outputs
        .into_iter()
        .find(|frame| frame.output == *output)
        .unwrap_or_else(|| empty_frame(output));
    let view = OutputView {
        area: config.logical_rect(),
        scale: config.scale,
        mode: config.mode.size,
    };
    let styles = frames::drawn(state);
    closing::remember(state, &styles);
    closing::forget(state, &in_scene);
    let leaving = closing::on(state, &frame);
    let overview = overview::draw(state, &view);
    let mut titles = titles::of(state, &frame, &view, &styles, &overview);
    titles.extend(titles::of_closing(state, &frame, &view, &leaving));
    let layers = LayerDraw {
        frames: styles,
        titles,
        closing: leaving,
        minimise_effect: state.core.model.settings().windows.minimise_effect,
        finish: crate::appearance::blur_finish(state.render.transparency, state.render.scheme),
        overview,
        backdrop: divider::backdrop(state, output),
        tile_preview: tile::preview(state, output),
        row: row_of(state, &frame, now),
        ..layers_on(&state.core.model, output)
    };
    let capture = tty::capture_watch(&state.wl.image_capture.book, &state.wl.screencopy, output);
    let source = SceneSource {
        frame: &frame,
        view: &view,
        layers: &layers,
        book: &state.render.textures,
        chrome: &state.render.chrome,
    };
    let shown = match &mut state.backend {
        BackendKind::Headless(backend) => {
            present_headless(backend, &name, now, &source).map(PresentOutcome::from)
        }
        BackendKind::Winit(winit) => winit.present(&source).map(PresentOutcome::from),
        BackendKind::Tty(seat) => tty::present(seat, &name, &config, capture, now, &source),
        // A backend this build does not know presents nothing.
        _ => Ok(PresentOutcome::Waiting),
    };
    match shown {
        Ok(PresentOutcome::Flipped(seq)) => {
            state.render.frame_faults.clear(&name);
            presented(state, output, &frame, config, now, seq);
            crate::timing::flipped(state, output, &frame, now);
        }
        Ok(PresentOutcome::Unchanged) => {
            state.render.frame_faults.clear(&name);
            send_callbacks(state, output, &frame, now);
            crate::timing::unchanged(state);
        }
        Ok(_) => {}
        Err(error) => {
            let repeat = state.render.frame_faults.admit(name.clone(), now);
            if let Repeat::Report { suppressed } = repeat {
                tracing::warn!(output = name.as_str(), %error, suppressed, "a frame failed");
            }
        }
    }
    // A capture that waited for a frame (the first of a session) may have one now.
    crate::handlers::serve_captures(state, output);
}

/// Presents on the headless backend; the sequence number of the frame shown.
fn present_headless(
    backend: &mut HeadlessBackend,
    name: &OutputName,
    now: Mono,
    source: &SceneSource<'_>,
) -> Result<Option<VblankSeq>, casement_backend::BackendError> {
    let seq = VblankSeq(
        backend
            .outputs()
            .get(name)
            .map_or(0, |output| output.presented)
            .saturating_add(1),
    );
    Ok(match backend.present(name, seq, now, source)? {
        Shown::Frame => Some(seq),
        Shown::Dark => None,
    })
}

/// What follows a presented frame: frame callbacks to the surfaces drawn, then the
/// presentation, fifo and commit-timing feedback.
fn presented(
    state: &mut State,
    output: &OutputKey,
    frame: &casement_motion::OutputFrame,
    config: OutputConfig,
    now: Mono,
    seq: VblankSeq,
) {
    send_callbacks(state, output, frame, now);
    // Presentation and commit timing speak `CLOCK_MONOTONIC`, not the session's `Mono`.
    let at = presentation::Presented {
        seq,
        at: state.clock.on_monotonic(now),
        refresh: config.mode.refresh,
    };
    let faults = [
        presentation::apply_presented(&mut state.wl.presentation, output, at),
        fifo::apply_presented(&mut state.wl.fifo, output, at),
        commit_timing::apply_presented(&mut state.wl.commit_timing, output, at),
    ];
    for fault in faults.into_iter().filter_map(Result::err) {
        tracing::warn!(%fault, "presentation feedback failed");
    }
    state.release_blockers();
}

/// Frame callbacks to the surfaces a frame of `output` drew, at the frame's instant.
fn send_callbacks(
    state: &mut State,
    output: &OutputKey,
    frame: &casement_motion::OutputFrame,
    now: Mono,
) {
    let roots = drawn_roots(state, output, frame);
    let sent: u64 = roots
        .iter()
        .map(|root| send_frame_callbacks(root, now))
        .sum();
    if let BackendKind::Headless(backend) = &mut state.backend {
        backend.note_callbacks(sent);
    }
}

/// The root surfaces a frame of `output` drew: its windows, its layers, their popups, its lock
/// surface and the cursor.
fn drawn_roots(
    state: &State,
    output: &OutputKey,
    frame: &casement_motion::OutputFrame,
) -> Vec<WlSurface> {
    let book = &state.render.textures;
    let windows = frame
        .windows
        .iter()
        .filter_map(|placed| book.window(&placed.window));
    let layers = state
        .core
        .model
        .layers()
        .iter()
        .filter(|(_, spec)| spec.output == *output)
        .filter_map(|(key, _)| book.layer(key));
    let lock = book.lock(output);
    let roots: Vec<WlSurface> = windows
        .chain(layers)
        .chain(lock)
        .map(|entry| entry.root.clone())
        .collect();
    // A popup is drawn with its root (and may hang from another popup): it has frames too.
    let popups: Vec<WlSurface> = roots
        .iter()
        .flat_map(PopupManager::popups_for_surface)
        .map(|(popup, _)| popup.wl_surface().clone())
        .collect();
    roots
        .into_iter()
        .chain(popups)
        .chain(book.cursor().cloned())
        .collect()
}

/// Sends `root` and its subsurfaces' frame callbacks `done` at `now`; how many it sent.
fn send_frame_callbacks(root: &WlSurface, now: Mono) -> u64 {
    let millis = u32::try_from(now.0.as_millis()).unwrap_or(u32::MAX);
    let mut sent = 0_u64;
    with_surface_tree_downward(
        root,
        (),
        |_, _, ()| TraversalAction::DoChildren(()),
        |_, states, ()| {
            let mut attributes = states.cached_state.get::<SurfaceAttributes>();
            for callback in attributes.current().frame_callbacks.drain(..) {
                callback.done(millis);
                sent = sent.saturating_add(1);
            }
        },
        |_, _, ()| true,
    );
    sent
}

/// Arranges the model again, retargets motion from the last arrangement and asks clients
/// whose size must change for it (a configure each).
pub(crate) fn relayout(state: &mut State, cause: &Cause) {
    let now = state.clock.now();
    let spec = spec_of_framed(&state.core.model, &frames::reserves(state));
    let next = state.core.layout.arrange(&spec, &state.core.knobs);
    let motion = std::mem::replace(&mut state.core.motion, casement_motion::MotionBook::empty());
    // A swipe that moves a row draws it itself, finish included: that row must not slide again.
    let driven: BTreeSet<OutputKey> = state
        .core
        .model
        .space_strip(now)
        .map(|strip| strip.output)
        .into_iter()
        .collect();
    state.core.motion = retarget_driven(
        motion,
        &state.core.arrangement,
        &next,
        cause,
        Default::default(),
        now,
        &driven,
    );
    for target in &next.windows {
        if let Some(size) = configure_size(state, target, cause) {
            let model = &state.core.model;
            let Some(states) = model.states_of(target.window) else {
                continue;
            };
            let bounds = model
                .outputs()
                .0
                .get(&target.output)
                .map_or(size, |output| {
                    model
                        .layers()
                        .usable_area(&target.output, output.config.logical_rect())
                        .size
                });
            let configure = casement_core::ConfigureSpec {
                size: casement_core::ConfigureSize::Exact(size),
                states,
                bounds,
            };
            if let Err(fault) =
                xdg_shell::apply_configure(&mut state.wl.xdg_shell, target.window, &configure)
            {
                tracing::warn!(window = target.window.0, %fault, "a configure failed");
            }
        }
    }
    state.core.arrangement = next;
    crate::handlers::refit_reactive_popups(state);
    crate::handlers::space_shot::poll(state);
}

/// The size to configure `target` to: what layout asks, and for a drag also the size the person
/// dragged the frame to when the client's committed buffer is not that yet (a floating window's
/// frame is its own size, so layout sees nothing to ask).
fn configure_size(
    state: &State,
    target: &casement_layout::WindowTarget,
    cause: &Cause,
) -> Option<casement_core::geometry::LogicalSize> {
    match (target.size, cause) {
        (ConfigureNeed::Resize(size), _) => Some(size),
        (ConfigureNeed::Keep, Cause::Drag) => state
            .render
            .textures
            .window(&target.window)
            .filter(|entry| entry.committed != target.rect.size)
            .map(|_| target.rect.size),
        (ConfigureNeed::Keep, _) => None,
    }
}

/// The frame the scene is at now (what a golden of motion reads).
pub fn sample_scene(state: &State) -> FrameScene {
    sample_scene_at(state, state.clock.now())
}

/// Reports the outputs the backend plugged at start (the caller does it once, after the
/// clock and display are attached, so start-up and hotplug are one path).
pub fn announce_outputs(state: &mut State) {
    load_stored_spaces(state);
    restore_lock(state);
    let events = match &mut state.backend {
        BackendKind::Headless(backend) => backend.announce(),
        BackendKind::Winit(winit) => winit.announce(),
        BackendKind::Tty(tty) => tty.announce(),
        _ => Vec::new(),
    };
    for event in events {
        connector(state, event);
    }
}

/// Reads the Spaces file through the injected store and tells the model, so a Space keeps its
/// id across a restart (P1-M2). A file that cannot be read is logged and the run goes on with
/// fresh ids.
fn load_stored_spaces(state: &mut State) {
    match crate::space_store::load_spaces(state.seams.space_store.as_ref()) {
        Ok(file) => feed(state, vec![Event::SpacesLoaded(Box::new(file))]),
        Err(error) => tracing::warn!(%error, "the Spaces file could not be read"),
    }
}

/// A lock the last run left behind (its marker is there) is held again before any output is
/// plugged, so every output comes up under the cover until the shell locks again and unlocks.
fn restore_lock(state: &mut State) {
    if state.seams.lock_marker.marked() == Marked::Yes {
        feed(state, vec![Event::Lock(LockEvent::Restored)]);
    }
}

/// Plugs a virtual output on the headless backend and reports it to the model; the connector
/// name. (A backend that is not headless plugs nothing.)
pub fn plug_output(state: &mut State, spec: casement_backend::VirtualOutputSpec) -> OutputName {
    let name = spec.name.clone();
    if let BackendKind::Headless(backend) = &mut state.backend {
        let event = backend.plug(spec);
        connector(state, event);
    }
    name
}

/// Unplugs `name` on the headless backend and reports it to the model.
pub fn unplug_output(state: &mut State, name: &OutputName) {
    if let BackendKind::Headless(backend) = &mut state.backend {
        let event = backend.unplug(name);
        connector(state, event);
    }
}

/// Puts `config` on `output` as if its display had been reconfigured: the backend, the vblank
/// clock and `wl_output` take it (`Effect::ApplyOutput`), then the model hears it applied.
pub fn configure_output(state: &mut State, output: &OutputKey, config: OutputConfig) {
    let apply = Effect::ApplyOutput {
        output: output.clone(),
        config,
    };
    exec::exec(state, vec![apply]);
    let output = output.clone();
    feed(state, vec![Event::OutputConfigured { output, config }]);
}

/// What a connector did, reported to the model (the same path whatever the backend). A
/// headless output keeps the mode, scale, transform and position its spec names: the model's
/// first-sight choice is applied first (it is the policy under test elsewhere) and the spec
/// then overrides it.
fn connector(state: &mut State, event: ConnectorEvent) {
    match event {
        ConnectorEvent::Plugged(facts)
            if state.start_mode == StartMode::Safe && !state.core.model.outputs().0.is_empty() =>
        {
            // `--safe` drives the first display only: the others stay dark.
            tracing::warn!(
                output = facts.name.as_str(),
                "safe mode: only the first display is used"
            );
        }
        ConnectorEvent::Plugged(facts) => {
            let (key, name) = (facts.key.clone(), facts.name.clone());
            // Read before the model's choice is applied: applying it rewrites the backend's spec.
            let wanted = spec_config(state, &name);
            feed(state, vec![Event::OutputPlugged(facts)]);
            if let Some(config) = wanted {
                configure_output(state, &key, config);
            }
            deliver::start_pointer(state);
        }
        ConnectorEvent::Changed(facts) => {
            let name = facts.name.clone();
            feed(state, vec![Event::OutputPlugged(facts)]);
            // A window's new size or scale is its output's new configuration.
            if let (Some(key), Some(config)) = (key_of(state, &name), window_config(state, &name)) {
                configure_output(state, &key, config);
            }
        }
        ConnectorEvent::Unplugged(key) => {
            feed(state, vec![Event::OutputUnplugged(key.clone())]);
            state.clock.unplug_output(&key);
            state.render.trackers.remove(&key);
            exec::withdraw_output(state, &key);
        }
    }
}

/// The configuration the winit window has, for its output `name`.
fn window_config(state: &State, name: &OutputName) -> Option<OutputConfig> {
    match &state.backend {
        BackendKind::Winit(window) if window.output_name() == name => Some(window.config()),
        _ => None,
    }
}

/// The key of the plugged output called `name`.
fn key_of(state: &State, name: &OutputName) -> Option<OutputKey> {
    let outputs = state.core.model.outputs();
    outputs
        .0
        .iter()
        .find(|(_, plugged)| plugged.facts.name == *name)
        .map(|(key, _)| key.clone())
}

/// The configuration `name`'s spec describes (headless) or the window has (winit).
fn spec_config(state: &State, name: &OutputName) -> Option<OutputConfig> {
    if let Some(config) = window_config(state, name) {
        return Some(config);
    }
    let BackendKind::Headless(backend) = &state.backend else {
        return None;
    };
    let output = backend.outputs().get(name)?;
    Some(OutputConfig {
        mode: output.spec.mode,
        scale: output.spec.scale,
        transform: output.spec.transform,
        position: output.spec.position,
        power: output.power,
    })
}

/// What happens once per loop turn: the input the backend queued is delivered, a locker that
/// died is noticed (smithay has no callback for it), and clients that left are told to the model. How many things moved.
pub fn turn(state: &mut State) -> usize {
    let events = session_lock::poll_locker(&mut state.wl.session_lock);
    let polled = events.len();
    feed(state, events);
    let gone = crate::clients::take_gone(state);
    let departed = gone.len();
    feed(state, gone);
    if departed > 0 {
        crate::exec::resync_keyboard(state);
    }
    let drained = drain_input(state);
    let reported = tty::drain(state);
    ime::settle(state);
    drained + polled + departed + reported
}

/// Delivers the input the backend queued, in the order it came: each event is told to the
/// model (`Event::Input`, for what listens) and then delivered to the clients by the reach
/// table, the hit test and the seat. How many events there were.
pub fn drain_input(state: &mut State) -> usize {
    let BackendKind::Headless(backend) = &mut state.backend else {
        return 0;
    };
    let queued = casement_backend::headless::HeadlessInput::drain(backend.input());
    let count = queued.len();
    for event in queued.iter().filter_map(input::event_of) {
        feed(state, vec![Event::Input(event)]);
    }
    count
}
