//! Layer surfaces: the bar, the dock and the wallpaper, and the room they take from windows.

use super::Model;
use super::moved::Moved;
use crate::cause::Cause;
use crate::effect::Effect;
use crate::geometry::{LogicalPoint, LogicalSize};
use crate::ids::{LayerKey, OutputKey};
use crate::layer::{ExclusiveZone, KeyboardInteractivity, LayerSpec};
use crate::lock::LockState;
use crate::time::Mono;
use std::collections::BTreeMap;

impl Model {
    /// A layer surface mapped on a plugged output. It is told its size, and the windows are
    /// laid out again when it reserves room. A surface on an output that is not plugged is
    /// not mapped.
    pub(super) fn map_layer(self, layer: LayerKey, spec: LayerSpec) -> Moved {
        if !self.outputs.0.contains_key(&spec.output) {
            return Moved::of(self);
        }
        let takes_keyboard = spec.keyboard == KeyboardInteractivity::OnDemand
            && self.focus.key.is_none()
            && self.demand.is_none();
        let model = Model {
            demand: takes_keyboard.then_some(layer).or(self.demand),
            ..self
        };
        model.with_layer(layer, spec)
    }

    /// A press landed on a layer surface: one that takes the keyboard on demand holds it from
    /// now (a window is pressed or keyed to take it back). Pressing any other layer changes
    /// nothing.
    pub fn press_layer(self, layer: LayerKey, now: Mono) -> (Model, Vec<Effect>) {
        self.run(now, |model| {
            let on_demand = model
                .layers
                .get(&layer)
                .is_some_and(|spec| spec.keyboard == KeyboardInteractivity::OnDemand);
            match (&model.lock, on_demand) {
                (LockState::Unlocked, true) => Moved::of(Model {
                    demand: Some(layer),
                    ..model
                }),
                _ => Moved::of(model),
            }
        })
    }

    /// A layer surface changed what it asks for. One never mapped is not known.
    pub(super) fn change_layer(self, layer: LayerKey, spec: LayerSpec) -> Moved {
        match self.layers.get(&layer) {
            Some(_) if self.outputs.0.contains_key(&spec.output) => self.with_layer(layer, spec),
            _ => Moved::of(self),
        }
    }

    /// A layer surface was pinned with its top-left at `at`. One never mapped is not known.
    pub(super) fn place_layer(self, layer: LayerKey, at: LogicalPoint) -> Moved {
        match self.layers.get(&layer).cloned() {
            Some(spec) => Moved::of(Model {
                layers: self.layers.with_pin(layer, at),
                ..self
            })
            .then(|model| model.laid_out_for(&spec, &spec)),
            None => Moved::of(self),
        }
    }

    /// A layer surface unmapped. Surfaces that avoid other zones get the room back.
    pub(super) fn unmap_layer(self, layer: LayerKey) -> Moved {
        let Some(spec) = self.layers.get(&layer).cloned() else {
            return Moved::of(self);
        };
        let outputs = [spec.output.clone()];
        let sizes = self.sizes_on(&outputs);
        let model = Model {
            layers: self.layers.without_layer(&layer),
            ..self
        };
        Moved::of(model)
            .then(|model| model.reconfigured(&outputs, &sizes, Some(layer)))
            .then(|model| model.laid_out_for(&spec, &spec))
    }

    /// The model with `layer` as `spec`: told its size, the surfaces that avoid other zones
    /// told theirs when this one's zone moved them, and the scene laid out again when its
    /// exclusive zone, before or after, takes room.
    fn with_layer(self, layer: LayerKey, spec: LayerSpec) -> Moved {
        let before = self.layers.get(&layer).cloned();
        let outputs: Vec<OutputKey> = before
            .iter()
            .map(|before| before.output.clone())
            .chain([spec.output.clone()])
            .collect();
        let sizes = self.sizes_on(&outputs);
        let model = Model {
            layers: self.layers.with_layer(layer, spec.clone()),
            ..self
        };
        let size = model.sizes_on(&outputs).get(&layer).copied();
        Moved::of(model)
            .with_all(size.map(|size| Effect::ConfigureLayer { layer, size }))
            .then(|model| model.reconfigured(&outputs, &sizes, Some(layer)))
            .then(|model| model.laid_out_for(before.as_ref().unwrap_or(&spec), &spec))
    }

    /// The configured size of every layer surface on `outputs`: each takes its size from the
    /// room [`crate::layer::LayerBook::arrange`] gives it.
    pub(super) fn sizes_on(&self, outputs: &[OutputKey]) -> BTreeMap<LayerKey, LogicalSize> {
        outputs
            .iter()
            .filter_map(|output| {
                let full = self.output_rect(output)?;
                let arranged = self.layers.arrange(output, full);
                Some(
                    self.layers
                        .iter()
                        .filter(|(_, spec)| spec.output == *output)
                        .filter_map(|(key, _)| Some((*key, arranged.configure_size(key)?)))
                        .collect::<Vec<_>>(),
                )
            })
            .flatten()
            .collect()
    }

    /// Every layer surface on `outputs` other than `except` whose size is not what `sizes` says
    /// it was, told its new size (a surface follows the zones of those arranged before it).
    pub(super) fn reconfigured(
        self,
        outputs: &[OutputKey],
        sizes: &BTreeMap<LayerKey, LogicalSize>,
        except: Option<LayerKey>,
    ) -> Moved {
        let effects: Vec<Effect> = self
            .sizes_on(outputs)
            .into_iter()
            .filter(|(key, size)| {
                Some(*key) != except && sizes.get(key).is_some_and(|was| was != size)
            })
            .map(|(layer, size)| Effect::ConfigureLayer { layer, size })
            .collect();
        Moved::of(self).with_all(effects)
    }

    /// Windows are laid out again when `before` or `after` reserves room on an output.
    fn laid_out_for(self, before: &LayerSpec, after: &LayerSpec) -> Moved {
        let mut outputs: Vec<OutputKey> = [before, after]
            .into_iter()
            .filter(|spec| matches!(spec.exclusive, ExclusiveZone::Reserve(_)))
            .map(|spec| spec.output.clone())
            .collect();
        outputs.sort();
        outputs.dedup();
        match outputs.is_empty() {
            true => Moved::of(self),
            false => self.redrawn(Cause::Configure, outputs),
        }
    }
}
