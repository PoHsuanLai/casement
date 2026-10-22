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
mod display_trial;
mod focus_feed;
mod keyboard;
mod layers;
mod lease_focus;
mod leases;
mod lock;
mod moved;
mod night_light;
mod output_requests;
mod outputs;
mod overview;
mod picture;
mod pip;
pub mod privilege;
pub use lease_focus::LeaseFocusRefusal;
pub use leases::Leasable;
mod published;
mod query;
mod requests;
mod row;
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
use crate::input::{InputKind, Leases};
use crate::invariants::{self, InvariantBreak};
use crate::layer::LayerBook;
use crate::lock::LockState;
use crate::output::{OutputConfig, OutputFacts};
use crate::overview::Overview;
use crate::reveal::{FullScreenBar, Reduced};
use crate::settings::CasementSettings;
use crate::space::{SpaceBook, SpaceFile, SpaceSeed};
use crate::time::{Mono, TimerBook, TimerKey};
use moved::Moved;
pub use overview::OverviewPick;
use privilege::{Input, Privilege};
pub use query::OpeningRoom;
pub use spaces::MAX_DESKTOPS_PER_OUTPUT;
use std::collections::{BTreeMap, BTreeSet};
pub use swipe::{SpaceStrip, SwipeOwner};

/// Whether a step compares what clients are shown before and after it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Watch {
    /// It may change the window lists, the Spaces or the desktop's notes.
    Published,
    /// It cannot (an input event that is not a swipe).
    Nothing,
}

/// Everything casement knows, as one value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Model {
    windows: BTreeMap<WindowId, crate::window::Window>,
    apps: BTreeMap<AppKey, App>,
    /// Each connected client and the apps its windows belong to: none until it maps one, several
    /// for a connection that carries more than one app (the X11 satellite; a peer that claims a
    /// different app id for each toplevel).
    clients: BTreeMap<ClientKey, BTreeSet<AppKey>>,
    /// The clients that came through the shell's own connection: the only ones whose lock
    /// takeover, exclusive keyboard layers and full-size zones are honoured.
    shell_clients: BTreeSet<ClientKey>,
    /// The clients that hold a `quire_window_manager_v1` bound at version 2 or later: an app
    /// asked to quit is sent `quit` through them instead of having its windows closed.
    quit_listeners: BTreeSet<ClientKey>,
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
    /// The session lock, with who asked for it: a lock whose locker is gone is taken over only
    /// by the shell or by that client again.
    lock: LockState,
    /// The agent's leases, each holding one window: what an agent event may reach.
    leases: Leases,
    /// Mission Control's life (`model/overview.rs`).
    overview: Overview,
    focus: Focus,
    /// The multi-finger swipes in flight (`model/swipe.rs`).
    swipe: swipe::SwipeState,
    timers: TimerBook,
    /// The display change a program made that the person has not yet kept or reverted
    /// (`model/display_trial.rs`).
    display_trial: Option<display_trial::Trial>,
    /// The last display trial id minted.
    trial_ids: u32,
    settings: CasementSettings,
    /// The night light's ramps and holds (`model/night_light.rs`).
    night: night_light::NightDrive,
    /// Which programs may use the powers that need the person's consent (`model/privilege`).
    privilege: Privilege,
    /// What the bar does in full screen (`quire_shell_v1.set_full_screen_bar`); the last request
    /// wins. Layout reads it for the rectangle a full-screen window covers.
    full_screen_bar: FullScreenBar,
    /// How much motion the shell says the person allows (`set_motion`); the last request wins.
    motion: Reduced,
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
            shell_clients: BTreeSet::new(),
            quit_listeners: BTreeSet::new(),
            spaces: SpaceBook::empty(),
            remembered: SpaceFile::empty(),
            minimise_rects: BTreeMap::new(),
            hand_placed: BTreeSet::new(),
            outputs: OutputBook::default(),
            layers: LayerBook::empty(),
            demand: None,
            split_choice: None,
            lock: LockState::Unlocked,
            leases: Leases::none(),
            overview: Overview::Closed,
            focus: Focus::none(),
            swipe: swipe::SwipeState::default(),
            timers: TimerBook::empty(),
            display_trial: None,
            trial_ids: 0,
            settings,
            night: night_light::NightDrive::default(),
            privilege: Privilege::new(epoch),
            full_screen_bar: FullScreenBar::Reveal,
            motion: Reduced::No,
            seed,
            now: epoch,
        }
    }

    /// What the bar does in full screen now (it slides in over the window until the shell says
    /// to keep it).
    pub fn full_screen_bar(&self) -> FullScreenBar {
        self.full_screen_bar
    }

    /// How much motion the shell says the person allows (full until it says otherwise).
    pub fn motion(&self) -> Reduced {
        self.motion
    }

    /// The model after `event` at `now`, and the effects it asks for. Routes the event to the
    /// machines (focus, lock, output policy, Spaces, layers) and keeps every invariant.
    ///
    /// Input events change nothing, except the multi-finger swipes: those move the Spaces and
    /// Mission Control (`model/swipe.rs`). What a key or a press means is decided where the target
    /// is known ([`Model::reach`], [`Model::press`], [`Model::command`]).
    pub fn step(self, event: Event, now: Mono) -> (Model, Vec<Effect>) {
        // An input event that is not a swipe changes nothing a window-list or desktop client
        // is shown, so it skips the two snapshots (each clones every window): at input rate
        // they were the cost a client with thousands of windows could stall the loop with.
        let watch = match &event {
            Event::Input(input) if !matches!(input.kind, InputKind::Swipe(_)) => Watch::Nothing,
            // Timers and ticks that change nothing a client is shown, at key-repeat and idle
            // rates (CASC-03).
            Event::TimerFired(
                TimerKey::KeyRepeat(_)
                | TimerKey::IdleNotify(..)
                | TimerKey::ImeHold
                | TimerKey::AgentHold,
            )
            | Event::NightLightTick(_) => Watch::Nothing,
            // Consent changes nothing a window-list or desktop client is shown.
            Event::ClientStanding { .. }
            | Event::PrivilegeUse { .. }
            | Event::PermissionAnswered { .. }
            | Event::PrivilegeGranted { .. }
            | Event::PrivilegeRevoked { .. }
            | Event::PrivilegeForgotten { .. }
            | Event::ShellReady
            | Event::ShellGone
            | Event::BookLoaded(_)
            | Event::FloorLoaded(_) => Watch::Nothing,
            _ => Watch::Published,
        };
        self.run_watching(now, watch, |model| model.route(event))
    }

    /// Whether every invariant holds.
    pub fn check_invariants(&self) -> Result<(), InvariantBreak> {
        invariants::check(self)
    }

    /// `f` on this model at `now`, then the keyboard effect if who holds the keyboard moved,
    /// and the publishing effects for what `f` changed.
    fn run(self, now: Mono, f: impl FnOnce(Model) -> Moved) -> (Model, Vec<Effect>) {
        self.run_watching(now, Watch::Published, f)
    }

    /// [`Model::run`], publishing only when `watch` says the step can change what clients are
    /// shown.
    fn run_watching(
        self,
        now: Mono,
        watch: Watch,
        f: impl FnOnce(Model) -> Moved,
    ) -> (Model, Vec<Effect>) {
        let before = self.keyboard_target();
        let keyed = self.focus.key;
        let shown = match watch {
            Watch::Published => Some(published::Published::of(&self)),
            Watch::Nothing => None,
        };
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
        let publish = shown.map_or_else(Vec::new, |shown| shown.changes_to(&moved.model));
        let moved = moved.with_all(publish);
        (moved.model, moved.effects)
    }

    /// Where `event` goes.
    fn route(self, event: Event) -> Moved {
        match event {
            Event::ClientConnected { client, role, .. } => self.connect(client, &role),
            Event::ClientGone { client } => self.disconnect(client),
            Event::ToplevelMapped(mapped) => self.map(mapped),
            Event::ToplevelUnmapped { window } => self.unmap(window),
            Event::LeaseGranted { lease, window } => self.grant_lease(lease, window),
            Event::LeaseFocus { lease } => self.focus_leased(lease),
            Event::LeaseEnded { lease } => self.end_lease(lease),
            Event::ToplevelCommitted { window, size } => self.commit(window, size),
            Event::ToplevelRequest { window, request } => {
                self.unless_locked(|model| model.request(window, request))
            }
            Event::ToplevelModality { window, modality } => self.modality(window, modality),
            Event::ToplevelMeta { window, meta } => self.meta(window, meta),
            Event::WindowDragged { window, rect } => self.dragged(window, rect),
            Event::WindowDragEnded { window } => self.drag_ended(window),
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
            Event::QuitListener { client, toggle } => self.quit_listener(client, toggle),
            Event::SettingsChanged(settings) => self.change_settings(*settings),
            Event::OutputRequested { output, change, by } => {
                self.request_output(output, change, by)
            }
            Event::DisplaysAnswered { trial, answer } => self.display_trial_answered(trial, answer),
            Event::GammaSet { output, ramp } => self.set_gamma(output, ramp),
            Event::NightLightTick(minute) => self.night_tick(minute),
            Event::WlrToplevel { window, request } => {
                self.unless_locked(|model| model.wlr_request(window, request))
            }
            Event::Workspace(request) => {
                self.unless_locked(|model| model.workspace_request(request))
            }
            Event::SpacesLoaded(file) => self.restore_spaces(*file),
            Event::Desktop(request) => self.unless_locked(|model| model.desktop_request(request)),
            Event::Shell(request) => self.shell_request(request),
            Event::ClientStanding { client, standing } => {
                self.privileged(Input::Standing { client, standing })
            }
            Event::PrivilegeUse { client, power } => self.privileged(Input::Use { client, power }),
            Event::PermissionAnswered { ask, answer, at } => {
                self.privileged(Input::Answered { ask, answer, at })
            }
            Event::PrivilegeGranted { subject, power, at } => {
                self.privileged(Input::Granted { subject, power, at })
            }
            Event::PrivilegeRevoked { subject, power, at } => {
                self.privileged(Input::Revoked { subject, power, at })
            }
            Event::PrivilegeForgotten { subject, power } => {
                self.privileged(Input::Forgotten { subject, power })
            }
            Event::ShellReady => self.privileged(Input::ShellReady),
            Event::ShellGone => self.privileged(Input::ShellGone),
            Event::BookLoaded(book) => self.privileged(Input::BookLoaded(*book)),
            Event::FloorLoaded(floor) => self.privileged(Input::FloorLoaded(*floor)),
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

    /// The agent's leases.
    pub fn leases(&self) -> &Leases {
        &self.leases
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
