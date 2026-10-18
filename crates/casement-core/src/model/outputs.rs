//! Outputs: plugging a display gives it a configuration and a desktop; unplugging one moves
//! its Spaces to another output, or leaves its windows homeless until one is plugged.

use super::moved::Moved;
use super::{Model, OutputState};
use crate::cause::Cause;
use crate::effect::Effect;
use crate::focus::FocusIn;
use crate::ids::OutputKey;
use crate::layer::LayerBook;
use crate::output::{OutputConfig, OutputFacts, OutputPower, SavedOutputs, place_outputs};
use crate::space::SpaceContent;
use crate::time::TimerKey;
use crate::window::{SpacePlace, WindowState};
use casement_protocols::vocab::WindowRole;
use porter_core::SpaceId;

impl Model {
    /// A display was plugged. One already known only has its facts replaced. A new one is
    /// placed right of the others at its first-sight scale, gets a desktop it shows, and
    /// takes in the windows that were waiting for an output.
    pub(super) fn plug(self, facts: OutputFacts) -> Moved {
        let key = facts.key.clone();
        if let Some(known) = self.outputs.0.get(&key).cloned() {
            let mut model = self;
            model.outputs.0.insert(key, OutputState { facts, ..known });
            return Moved::of(model);
        }
        let config = self.first_config(&facts);
        let cover = self.cover_for_new(&key);
        let mut model = self;
        model
            .outputs
            .0
            .insert(key.clone(), OutputState { facts, config });
        let (model, desktop) = model.with_space(&key, SpaceContent::Desktop);
        let shown = desktop.clone();
        let mut model = model;
        model.spaces = model.spaces.with_shown(key.clone(), desktop);
        // The Spaces the file stored for this display take their ids (P1-M2).
        let mut model = model.adopt_ids(&key);
        if let Some(choice) = model.stored_choice(&key) {
            model.spaces = model.spaces.with_shown(key.clone(), choice);
        }
        let shown = model
            .spaces
            .shown_on(&key)
            .map_or(shown, |space| space.id.clone());
        let model = model.homed_on(&key, &shown);
        Moved::of(model)
            .with(Effect::ApplyOutput {
                output: key.clone(),
                config,
            })
            .with_all(cover)
            .then(|model| model.redrawn(Cause::OutputChange, vec![key]))
            .then(Model::night_drive)
    }

    /// The configuration a new output gets: place it among the plugged ones, which keep
    /// theirs. A display that offers no mode (a 0x0 mode) is left dark.
    fn first_config(&self, facts: &OutputFacts) -> OutputConfig {
        let present: Vec<OutputFacts> = self
            .outputs
            .0
            .values()
            .map(|state| state.facts.clone())
            .chain(std::iter::once(facts.clone()))
            .collect();
        let saved = SavedOutputs(
            self.outputs
                .0
                .iter()
                .map(|(key, state)| (key.clone(), state.config))
                .collect(),
        );
        let placed = place_outputs(&present, &saved, &self.settings.outputs.first_sight);
        let config = placed
            .into_iter()
            .find(|(key, _)| *key == facts.key)
            .map(|(_, config)| config);
        config.map_or_else(|| dark_config(facts), lit_if_it_has_a_mode)
    }

    /// The windows waiting for an output take `output` as their home: shell windows on every
    /// Space of it, the rest on its `desktop`.
    fn homed_on(self, output: &OutputKey, desktop: &SpaceId) -> Model {
        let mut model = self;
        for window in model.windows.values_mut() {
            let waiting = window.place == SpacePlace::AllSpaces && window.home.is_none();
            match (waiting, window.role) {
                (true, WindowRole::Shell) => window.home = Some(output.clone()),
                (true, _) => window.place = SpacePlace::On(desktop.clone()),
                (false, _) => {}
            }
        }
        model
    }

    /// A display was unplugged. Its Spaces (and the windows on them) go to the primary one
    /// of the rest, at the end of its row; with none left they go, and every window waits
    /// for an output. Its layer surfaces and timers go with it.
    pub(super) fn unplug(self, output: &OutputKey) -> Moved {
        if !self.outputs.0.contains_key(output) {
            return Moved::of(self);
        }
        let mut model = self.night_forget(output);
        model.outputs.0.remove(output);
        let model = model.lock_without_output(output);
        let heir = model.primary_output();
        let model = match &heir {
            Some(heir) => model.moved_to(output, heir),
            None => model.homeless(output),
        };
        let model = model.without_layers_on(output);
        let timers = [
            TimerKey::FrameWatch(output.clone()),
            TimerKey::SpaceFinish(output.clone()),
        ];
        let model = timers.iter().fold(model, |mut model, key| {
            model.timers = model.timers.cancel(key);
            model
        });
        let refocus = heir
            .and_then(|heir| model.spaces.shown_on(&heir).cloned())
            .map(|shown| FocusIn::SpaceShown {
                output: shown.output,
                space: shown.id,
            });
        let outputs = model.outputs.0.keys().cloned().collect();
        Moved::of(model)
            .with_all(timers.into_iter().map(Effect::CancelTimer))
            .then(|model| match refocus {
                Some(input) => model.feed(input),
                None => Moved::of(model),
            })
            .then(|model| model.redrawn(Cause::OutputChange, outputs))
    }

    /// `from`'s Spaces join the end of `to`'s row, and its every-Space windows follow.
    fn moved_to(self, from: &OutputKey, to: &OutputKey) -> Model {
        let moving: Vec<_> = self.spaces.row(from).into_iter().cloned().collect();
        let mut model = moving.into_iter().fold(self, |model, space| {
            let position = crate::space::SpaceIndex(
                u32::try_from(model.spaces.row(to).len()).unwrap_or(u32::MAX),
            );
            let spaces = model.spaces.with_space(crate::space::Space {
                output: to.clone(),
                position,
                ..space
            });
            Model { spaces, ..model }
        });
        model.spaces = model.spaces.without_shown(from);
        for window in model.windows.values_mut() {
            if window.home.as_ref() == Some(from) {
                window.home = Some(to.clone());
            }
        }
        model.renumbered(to)
    }

    /// The last output left: its Spaces go, full-screen windows come back to the state they
    /// had, and every window waits on every Space with no home.
    fn homeless(self, from: &OutputKey) -> Model {
        let mut model = self;
        for space in model
            .spaces
            .row(from)
            .into_iter()
            .cloned()
            .collect::<Vec<_>>()
        {
            model.spaces = model.spaces.without_space(&space.id);
        }
        for window in model.windows.values_mut() {
            if let WindowState::FullScreen { restore, .. } = &window.state {
                window.state = restore.clone().into_state();
            }
            window.place = SpacePlace::AllSpaces;
            window.home = None;
        }
        model
    }

    /// The model without the layer surfaces on `output`.
    fn without_layers_on(self, output: &OutputKey) -> Model {
        let gone: Vec<_> = self
            .layers
            .iter()
            .filter(|(_, spec)| spec.output == *output)
            .map(|(key, _)| *key)
            .collect();
        let layers = gone.iter().fold(self.layers, LayerBook::without_layer);
        Model { layers, ..self }
    }

    /// An output's configuration was applied (or changed): the windows are laid out for it.
    pub(super) fn configure_output(self, output: &OutputKey, config: OutputConfig) -> Moved {
        let Some(state) = self.outputs.0.get(output).cloned() else {
            return Moved::of(self);
        };
        let outputs = [output.clone()];
        let sizes = self.sizes_on(&outputs);
        let mut model = self;
        model
            .outputs
            .0
            .insert(output.clone(), OutputState { config, ..state });
        Moved::of(model)
            .then(|model| model.reconfigured(&outputs, &sizes, None))
            .then(|model| model.redrawn(Cause::OutputChange, outputs.to_vec()))
    }
}

/// A configuration for a display `place_outputs` did not return (it always does): dark.
fn dark_config(facts: &OutputFacts) -> OutputConfig {
    lit_if_it_has_a_mode(OutputConfig {
        mode: crate::output::pick_mode(facts, &crate::output::ModeWish::Preferred),
        scale: ds_core::geometry::scale::Scale::ONE,
        transform: crate::geometry::Transform::Normal,
        position: crate::geometry::LogicalPoint { x: 0, y: 0 },
        power: OutputPower::Off,
    })
}

/// `config`, left dark when its mode has no pixels (a display that offers no mode).
fn lit_if_it_has_a_mode(config: OutputConfig) -> OutputConfig {
    match (config.mode.size.w, config.mode.size.h) {
        (0, _) | (_, 0) => OutputConfig {
            power: OutputPower::Off,
            ..config
        },
        _ => config,
    }
}
