//! The model: every window, app, Space, output, layer surface, the lock, focus, timers and
//! settings, and the one transition that moves them.

use crate::app::{App, AppKey};
use crate::effect::Effect;
use crate::event::Event;
use crate::focus::Focus;
use crate::ids::{OutputKey, WindowId};
use crate::invariants::InvariantBreak;
use crate::layer::LayerBook;
use crate::lock::LockState;
use crate::output::{OutputConfig, OutputFacts};
use crate::settings::CasementSettings;
use crate::space::{SpaceBook, SpaceSeed};
use crate::time::{Mono, TimerBook};
use std::collections::BTreeMap;

/// Everything casement knows, as one value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Model {
    windows: BTreeMap<WindowId, crate::window::Window>,
    apps: BTreeMap<AppKey, App>,
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
    /// machines (focus, lock, output policy, Spaces) and keeps every invariant.
    pub fn step(self, event: Event, now: Mono) -> (Model, Vec<Effect>) {
        let _ = (event, now, &self.seed);
        todo!("P0-F1b-model: route the event to the machines")
    }

    /// Whether every invariant holds.
    pub fn check_invariants(&self) -> Result<(), InvariantBreak> {
        todo!("P0-F1b-model: the InvariantBreak checks")
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
