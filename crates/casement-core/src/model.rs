//! The model: every window, app, Space, output, layer surface, the lock, focus, timers and
//! settings, and the one transition that moves them.
//!
//! `step` routes each event to the machine that owns it: focus (`focus::step`), Spaces
//! (`model/spaces.rs`), outputs and their policy, layers, the lock, and the window records.
//! Each handler takes the model and returns a [`Moved`]: the next model and the effects it
//! asked for, so a transition reads as a chain. Keyboard focus is never moved by a handler:
//! it is derived once per step from who holds it (the lock, an exclusive layer, the key
//! window), and an effect goes out when that changed.

mod apps;
mod clients;
mod commands;
mod desktop_notes;
mod desktop_places;
mod desktop_requests;
mod focus_feed;
mod keyboard;
mod layers;
mod lock;
mod moved;
mod night_light;
mod output_requests;
mod outputs;
mod overview;
mod picture;
mod published;
mod query;
mod requests;
mod shell_requests;
mod space_persist;
mod spaces;
mod split;
mod states;
mod swipe;
mod timers;
mod window_menu;
mod windows;
mod wlr_requests;
mod workspace_requests;

#[cfg(test)]
mod tests;

use crate::app::{App, AppKey};
use crate::cause::MinimiseTarget;
use crate::effect::Effect;
use crate::event::Event;
use crate::focus::Focus;
use crate::ids::{ClientKey, LayerKey, OutputKey, WindowId};
use crate::invariants::{self, InvariantBreak};
use crate::layer::LayerBook;
use crate::lock::LockState;
use crate::output::{OutputConfig, OutputFacts};
use crate::overview::Overview;
use crate::settings::CasementSettings;
use crate::space::{SpaceBook, SpaceFile, SpaceSeed};
use crate::time::{Mono, TimerBook};
use moved::Moved;
pub use overview::OverviewPick;
pub use query::OpeningRoom;
use std::collections::{BTreeMap, BTreeSet};
pub use swipe::{SpaceStrip, SwipeOwner};

/// Everything casement knows, as one value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Model {
    windows: BTreeMap<WindowId, crate::window::Window>,
    apps: BTreeMap<AppKey, App>,
    /// Each connected client and the app its windows belong to, once it has mapped one.
    clients: BTreeMap<ClientKey, Option<AppKey>>,
    spaces: SpaceBook,
    /// The Spaces file as it was read at start (P1-M2): its Spaces wait here for their output
    /// to be plugged, and what is stored still names them until then.
    remembered: SpaceFile,
    /// Where each window's minimise animation should go: the rectangle its window-list client
    /// set (`zwlr_foreign_toplevel_handle_v1.set_rectangle`; P1-M2).
    minimise_rects: BTreeMap<WindowId, MinimiseTarget>,
    /// The windows put by hand (`Event::WindowPlaced`): layout keeps their rectangle as given.
    hand_placed: BTreeSet<WindowId>,
    outputs: OutputBook,
    layers: LayerBook,
    /// The on-demand layer surface that holds the keyboard: the one that last mapped above the
    /// windows (nothing exclusive or locked holding the keyboard), or the one a press last landed on. A window pressed or keyed takes it back.
    demand: Option<LayerKey>,
    /// The Split View whose other half is being chosen, if one is (`model/split.rs`).
    split_choice: Option<split::SplitChoice>,
    lock: LockState,
    /// Mission Control's life (`model/overview.rs`).
    overview: Overview,
    focus: Focus,
    /// The multi-finger swipes in flight (`model/swipe.rs`).
    swipe: swipe::SwipeState,
    timers: TimerBook,
    settings: CasementSettings,
    /// The night light's ramps and holds (`model/night_light.rs`).
    night: night_light::NightDrive,
    seed: SpaceSeed,
    now: Mono,
}

/// Every plugged output: what it reports and how it is configured.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct OutputBook(pub BTreeMap<OutputKey, OutputState>);

/// One plugged output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputState {
    /// What it reports.
    pub facts: OutputFacts,
    /// How it is configured.
    pub config: OutputConfig,
}

impl Model {
    /// An empty desktop at `epoch` under `settings`, minting Space ids from `seed`.
    pub fn new(settings: CasementSettings, epoch: Mono, seed: SpaceSeed) -> Model {
        Model {
            windows: BTreeMap::new(),
            apps: BTreeMap::new(),
            clients: BTreeMap::new(),
            spaces: SpaceBook::empty(),
            remembered: SpaceFile::empty(),
            minimise_rects: BTreeMap::new(),
            hand_placed: BTreeSet::new(),
            outputs: OutputBook::default(),
            layers: LayerBook::empty(),
            demand: None,
            split_choice: None,
            lock: LockState::Unlocked,
            overview: Overview::Closed,
            focus: Focus::none(),
            swipe: swipe::SwipeState::default(),
            timers: TimerBook::empty(),
            settings,
            night: night_light::NightDrive::default(),
            seed,
            now: epoch,
        }
    }

    /// The model after `event` at `now`, and the effects it asks for. Routes the event to the
    /// machines (focus, lock, output policy, Spaces, layers) and keeps every invariant.
    ///
    /// Input events change nothing, except the multi-finger swipes: those move the Spaces and
    /// Mission Control (`model/swipe.rs`). What a key or a press means is decided where the target
    /// is known ([`Model::reach`], [`Model::press`], [`Model::command`]).
    pub fn step(self, event: Event, now: Mono) -> (Model, Vec<Effect>) {
        self.run(now, |model| model.route(event))
    }

    /// Whether every invariant holds.
    pub fn check_invariants(&self) -> Result<(), InvariantBreak> {
        invariants::check(self)
    }

    /// `f` on this model at `now`, then the keyboard effect if who holds the keyboard moved.
    fn run(self, now: Mono, f: impl FnOnce(Model) -> Moved) -> (Model, Vec<Effect>) {
        let before = self.keyboard_target();
        let keyed = self.focus.key;
        let shown = published::Published::of(&self);
        let mut moved = f(Model { now, ..self });
        moved.model.forget_gone_minimise_rects();
        moved.model.forget_gone_placements();
        // A window that becomes key takes the keyboard back from an on-demand layer.
        if moved.model.focus.key != keyed && moved.model.focus.key.is_some() {
            moved.model.demand = None;
        }
        let after = moved.model.keyboard_target();
        let moved = if before == after {
            moved
        } else {
            moved.with(Effect::Keyboard(after))
        };
        let publish = shown.changes_to(&moved.model);
        let moved = moved.with_all(publish);
        (moved.model, moved.effects)
    }

    /// Where `event` goes.
    fn route(self, event: Event) -> Moved {
        match event {
            Event::ClientConnected { client, .. } => self.connect(client),
            Event::ClientGone { client } => self.disconnect(client),
            Event::ToplevelMapped(mapped) => self.map(mapped),
            Event::ToplevelUnmapped { window } => self.unmap(window),
            Event::ToplevelCommitted { window, size } => self.commit(window, size),
            Event::ToplevelRequest { window, request } => self.request(window, request),
            Event::ToplevelModality { window, modality } => self.modality(window, modality),
            Event::ToplevelMeta { window, meta } => self.meta(window, meta),
            Event::WindowDragged { window, rect } => self.dragged(window, rect),
            Event::WindowPlaced { window, rect } => self.placed_by_hand(window, rect),
            Event::WindowHint { window, hint } => self.hint(window, hint),
            Event::LayerMapped { layer, spec } => self.map_layer(layer, spec),
            Event::LayerChanged { layer, spec } => self.change_layer(layer, spec),
            Event::LayerPlaced { layer, at } => self.place_layer(layer, at),
            Event::LayerUnmapped { layer } => self.unmap_layer(layer),
            Event::Lock(event) => self.lock_event(event),
            Event::Input(event) => self.input_seen().then(|model| model.swipe_input(event)),
            Event::OutputPlugged(facts) => self.plug(facts),
            Event::OutputUnplugged(output) => self.unplug(&output),
            Event::OutputConfigured { output, config } => self.configure_output(&output, config),
            Event::TimerFired(key) => self.fire(key),
            Event::Pong { client } => self.pong(client),
            Event::SettingsChanged(settings) => self.change_settings(*settings),
            Event::OutputRequested { output, change } => self.request_output(output, change),
            Event::GammaSet { output, ramp } => self.set_gamma(output, ramp),
            Event::NightLightTick(minute) => self.night_tick(minute),
            Event::WlrToplevel { window, request } => self.wlr_request(window, request),
            Event::Workspace(request) => self.workspace_request(request),
            Event::SpacesLoaded(file) => self.restore_spaces(*file),
            Event::Desktop(request) => self.desktop_request(request),
            Event::Shell(request) => self.shell_request(request),
        }
    }

    /// Every window.
    pub fn windows(&self) -> &BTreeMap<WindowId, crate::window::Window> {
        &self.windows
    }

    /// Every app.
    pub fn apps(&self) -> &BTreeMap<AppKey, App> {
        &self.apps
    }

    /// Every Space.
    pub fn spaces(&self) -> &SpaceBook {
        &self.spaces
    }

    /// Where `window` should minimise to, when its window-list client set a rectangle.
    pub fn minimise_target(&self, window: WindowId) -> Option<&MinimiseTarget> {
        self.minimise_rects.get(&window)
    }

    /// Every plugged output.
    pub fn outputs(&self) -> &OutputBook {
        &self.outputs
    }

    /// Every layer surface.
    pub fn layers(&self) -> &LayerBook {
        &self.layers
    }

    /// The session lock.
    pub fn lock(&self) -> &LockState {
        &self.lock
    }

    /// Focus.
    pub fn focus(&self) -> &Focus {
        &self.focus
    }

    /// The armed timers.
    pub fn timers(&self) -> &TimerBook {
        &self.timers
    }

    /// The settings.
    pub fn settings(&self) -> &CasementSettings {
        &self.settings
    }

    /// The instant of the latest step (the epoch before any).
    pub fn now(&self) -> Mono {
        self.now
    }
}
