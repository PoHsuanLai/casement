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
mod focus_feed;
mod keyboard;
mod layers;
mod lock;
mod moved;
mod outputs;
mod query;
mod requests;
mod spaces;
mod states;
mod timers;
mod windows;

#[cfg(test)]
mod tests;

use crate::app::{App, AppKey};
use crate::effect::Effect;
use crate::event::Event;
use crate::focus::Focus;
use crate::ids::{ClientKey, OutputKey, WindowId};
use crate::invariants::{self, InvariantBreak};
use crate::layer::LayerBook;
use crate::lock::LockState;
use crate::output::{OutputConfig, OutputFacts};
use crate::settings::CasementSettings;
use crate::space::{SpaceBook, SpaceSeed};
use crate::time::{Mono, TimerBook};
use moved::Moved;
use std::collections::BTreeMap;

/// Everything casement knows, as one value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Model {
    windows: BTreeMap<WindowId, crate::window::Window>,
    apps: BTreeMap<AppKey, App>,
    /// Each connected client and the app its windows belong to, once it has mapped one.
    clients: BTreeMap<ClientKey, Option<AppKey>>,
    spaces: SpaceBook,
    outputs: OutputBook,
    layers: LayerBook,
    lock: LockState,
    focus: Focus,
    timers: TimerBook,
    settings: CasementSettings,
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
            outputs: OutputBook::default(),
            layers: LayerBook::empty(),
            lock: LockState::Unlocked,
            focus: Focus::none(),
            timers: TimerBook::empty(),
            settings,
            seed,
            now: epoch,
        }
    }

    /// The model after `event` at `now`, and the effects it asks for. Routes the event to the
    /// machines (focus, lock, output policy, Spaces, layers) and keeps every invariant.
    ///
    /// Input events change nothing: what a key or a press means is decided where the target
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
        let moved = f(Model { now, ..self });
        let after = moved.model.keyboard_target();
        let moved = if before == after {
            moved
        } else {
            moved.with(Effect::Keyboard(after))
        };
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
            Event::ToplevelMeta { window, meta } => self.meta(window, meta),
            Event::WindowHint { window, hint } => self.hint(window, hint),
            Event::LayerMapped { layer, spec } => self.map_layer(layer, spec),
            Event::LayerChanged { layer, spec } => self.change_layer(layer, spec),
            Event::LayerUnmapped { layer } => self.unmap_layer(layer),
            Event::Lock(event) => self.lock_event(event),
            Event::Input(_) => Moved::of(self),
            Event::OutputPlugged(facts) => self.plug(facts),
            Event::OutputUnplugged(output) => self.unplug(&output),
            Event::OutputConfigured { output, config } => self.configure_output(&output, config),
            Event::TimerFired(key) => self.fire(key),
            Event::Pong { client } => self.pong(client),
            Event::SettingsChanged(settings) => self.change_settings(*settings),
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
