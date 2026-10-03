//! Every layer surface, and the usable area their exclusive zones leave on an output.

use super::LayerSpec;
use crate::geometry::LogicalRect;
use crate::ids::{LayerKey, OutputKey};
use std::collections::BTreeMap;

/// Every mapped layer surface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayerBook {
    layers: BTreeMap<LayerKey, LayerSpec>,
}

impl LayerBook {
    /// No layer surfaces.
    pub fn empty() -> LayerBook {
        LayerBook {
            layers: BTreeMap::new(),
        }
    }

    /// The book with `layer` mapped as `spec` (or changed to it).
    pub fn with_layer(self, layer: LayerKey, spec: LayerSpec) -> LayerBook {
        let mut layers = self.layers;
        layers.insert(layer, spec);
        LayerBook { layers }
    }

    /// The book without `layer`.
    pub fn without_layer(self, layer: &LayerKey) -> LayerBook {
        let mut layers = self.layers;
        layers.remove(layer);
        LayerBook { layers }
    }

    /// What `full` (an output's whole logical rectangle) leaves for windows once every layer
    /// surface on `output` that reserves an exclusive zone has taken it, beside the edge it is
    /// anchored to (its margin on that edge included).
    pub fn usable_area(&self, output: &OutputKey, full: LogicalRect) -> LogicalRect {
        let _ = (output, full, &self.layers);
        todo!("P0-F1-output: subtract each Reserve zone from its anchored edge")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::{LogicalPoint, LogicalSize};
    use crate::ids::OutputName;
    use crate::layer::{
        Anchor, Anchors, ExclusiveZone, KeyboardInteractivity, LayerLevel, Margins, ShellRoleTag,
    };
    use crate::output::{EdidFacts, EdidSerial};

    fn output(name: &str) -> OutputKey {
        let edid = EdidFacts {
            make: String::new(),
            model: String::new(),
            serial: EdidSerial::Unknown,
        };
        OutputKey::of(&edid, &OutputName::parse(name).expect("name"))
    }

    fn rect(x: i32, y: i32, w: u32, h: u32) -> LogicalRect {
        LogicalRect::new(LogicalPoint { x, y }, LogicalSize { w, h })
    }

    const NO_MARGIN: Margins = Margins {
        top: 0,
        right: 0,
        bottom: 0,
        left: 0,
    };

    /// A layer on `on` held to the edges named in `edges` ("t", "b", "l", "r").
    fn layer(on: &str, edges: &str, exclusive: ExclusiveZone, margin: Margins) -> LayerSpec {
        let held = |c: char| {
            if edges.contains(c) {
                Anchor::Held
            } else {
                Anchor::Free
            }
        };
        LayerSpec {
            output: output(on),
            level: LayerLevel::Top,
            anchor: Anchors {
                top: held('t'),
                bottom: held('b'),
                left: held('l'),
                right: held('r'),
            },
            exclusive,
            margin,
            keyboard: KeyboardInteractivity::None,
            namespace: "test".to_owned(),
            role: ShellRoleTag::Unassigned,
        }
    }

    #[test]
    #[ignore = "P0-F1-output"]
    fn exclusive_zones_shrink_the_usable_area() {
        let full = rect(0, 0, 1920, 1080);
        let top_margin = Margins {
            top: 4,
            ..NO_MARGIN
        };
        let cases: Vec<(&str, Vec<LayerSpec>, LogicalRect)> = vec![
            ("no layers", vec![], full),
            (
                "bar across the top",
                vec![layer(
                    "HEADLESS-1",
                    "tlr",
                    ExclusiveZone::Reserve(24),
                    NO_MARGIN,
                )],
                rect(0, 24, 1920, 1056),
            ),
            (
                "bar held to the top only",
                vec![layer(
                    "HEADLESS-1",
                    "t",
                    ExclusiveZone::Reserve(24),
                    NO_MARGIN,
                )],
                rect(0, 24, 1920, 1056),
            ),
            (
                "bar with a top margin",
                vec![layer(
                    "HEADLESS-1",
                    "tlr",
                    ExclusiveZone::Reserve(24),
                    top_margin,
                )],
                rect(0, 28, 1920, 1052),
            ),
            (
                "bar and dock",
                vec![
                    layer("HEADLESS-1", "tlr", ExclusiveZone::Reserve(24), NO_MARGIN),
                    layer("HEADLESS-1", "blr", ExclusiveZone::Reserve(80), NO_MARGIN),
                ],
                rect(0, 24, 1920, 976),
            ),
            (
                "side dock",
                vec![layer(
                    "HEADLESS-1",
                    "ltb",
                    ExclusiveZone::Reserve(64),
                    NO_MARGIN,
                )],
                rect(64, 0, 1856, 1080),
            ),
            (
                "respect reserves nothing",
                vec![layer(
                    "HEADLESS-1",
                    "tlr",
                    ExclusiveZone::Respect,
                    NO_MARGIN,
                )],
                full,
            ),
            (
                "ignore reserves nothing",
                vec![layer("HEADLESS-1", "tlr", ExclusiveZone::Ignore, NO_MARGIN)],
                full,
            ),
            (
                "another output's bar",
                vec![layer(
                    "HEADLESS-2",
                    "tlr",
                    ExclusiveZone::Reserve(24),
                    NO_MARGIN,
                )],
                full,
            ),
            (
                "held to opposite edges reserves nothing",
                vec![layer(
                    "HEADLESS-1",
                    "tb",
                    ExclusiveZone::Reserve(24),
                    NO_MARGIN,
                )],
                full,
            ),
        ];
        for (name, layers, want) in cases {
            let book = layers
                .into_iter()
                .enumerate()
                .fold(LayerBook::empty(), |book, (n, spec)| {
                    book.with_layer(LayerKey(n as u64), spec)
                });
            assert_eq!(
                book.usable_area(&output("HEADLESS-1"), full),
                want,
                "{name}"
            );
        }
    }
}
