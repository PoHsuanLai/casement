//! Layer surfaces: the bar, the dock and the wallpaper, and the room they take from windows.

use super::Model;
use super::moved::Moved;
use crate::cause::Cause;
use crate::effect::Effect;
use crate::geometry::LogicalSize;
use crate::ids::{LayerKey, OutputKey};
use crate::layer::{Anchor, Anchors, ExclusiveZone, LayerSpec};

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

    /// A layer surface unmapped.
    pub(super) fn unmap_layer(self, layer: LayerKey) -> Moved {
        let Some(spec) = self.layers.get(&layer).cloned() else {
            return Moved::of(self);
        };
        let model = Model {
            layers: self.layers.without_layer(&layer),
            ..self
        };
        model.laid_out_for(&spec, &spec)
    }

    /// The model with `layer` as `spec`: told its size, and the scene laid out again when its
    /// exclusive zone, before or after, takes room.
    fn with_layer(self, layer: LayerKey, spec: LayerSpec) -> Moved {
        let before = self.layers.get(&layer).cloned();
        let size = self
            .output_rect(&spec.output)
            .map_or(LogicalSize { w: 0, h: 0 }, |rect| {
                layer_size(&spec, rect.size)
            });
        let model = Model {
            layers: self.layers.with_layer(layer, spec.clone()),
            ..self
        };
        Moved::of(model)
            .with(Effect::ConfigureLayer { layer, size })
            .then(|model| model.laid_out_for(before.as_ref().unwrap_or(&spec), &spec))
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
