//! Layer surfaces: the bar, the dock and the wallpaper, and the room they take from windows.

use super::Model;
use super::moved::Moved;
use crate::cause::Cause;
use crate::effect::Effect;
use crate::geometry::LogicalSize;
use crate::ids::{LayerKey, OutputKey};
use crate::layer::{Anchor, Anchors, ExclusiveZone, LayerSpec};
use std::collections::BTreeMap;

impl Model {
    /// A layer surface mapped on a plugged output. It is told its size, and the windows are
    /// laid out again when it reserves room. A surface on an output that is not plugged is
    /// not mapped.
    pub(super) fn map_layer(self, layer: LayerKey, spec: LayerSpec) -> Moved {
        if !self.outputs.0.contains_key(&spec.output) {
            return Moved::of(self);
        }
        self.with_layer(layer, spec)
    }

    /// A layer surface changed what it asks for. One never mapped is not known.
    pub(super) fn change_layer(self, layer: LayerKey, spec: LayerSpec) -> Moved {
        match self.layers.get(&layer) {
            Some(_) if self.outputs.0.contains_key(&spec.output) => self.with_layer(layer, spec),
            _ => Moved::of(self),
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
                        .filter_map(|(key, spec)| {
                            let room = arranged.room(key)?;
                            Some((*key, layer_size(spec, room.size)))
                        })
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

/// The size a layer surface is configured to: along an axis it is held to both ends of, the
/// output less its margins; otherwise 0, the surface chooses (wlr-layer-shell).
fn layer_size(spec: &LayerSpec, output: LogicalSize) -> LogicalSize {
    let Anchors {
        top,
        bottom,
        left,
        right,
    } = spec.anchor;
    LogicalSize {
        w: along(
            (left, right),
            output.w,
            (spec.margin.left, spec.margin.right),
        ),
        h: along(
            (top, bottom),
            output.h,
            (spec.margin.top, spec.margin.bottom),
        ),
    }
}

/// The length along one axis: all the room less the margins when both ends are held, else 0.
fn along(ends: (Anchor, Anchor), room: u32, margins: (i32, i32)) -> u32 {
    match ends {
        (Anchor::Held, Anchor::Held) => {
            let taken = i64::from(margins.0) + i64::from(margins.1);
            u32::try_from((i64::from(room) - taken).max(0)).unwrap_or(0)
        }
        _ => 0,
    }
}
