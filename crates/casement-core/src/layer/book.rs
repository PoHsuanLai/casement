//! Every layer surface, and the usable area their exclusive zones leave on an output.

use super::{LayerSpec, ShellRoleTag};
use crate::geometry::{LogicalPoint, LogicalRect};
use crate::ids::{LayerKey, OutputKey};
use casement_protocols::vocab::{FullScreenBar, ShellRole};
use std::collections::BTreeMap;

/// Every mapped layer surface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayerBook {
    layers: BTreeMap<LayerKey, LayerSpec>,
    /// Where the surfaces put by hand sit (the top-left); the others sit where their anchors say.
    pins: BTreeMap<LayerKey, LogicalPoint>,
}

impl LayerBook {
    /// No layer surfaces.
    pub fn empty() -> LayerBook {
        LayerBook {
            layers: BTreeMap::new(),
            pins: BTreeMap::new(),
        }
    }

    /// The book with `layer` mapped as `spec` (or changed to it).
    pub fn with_layer(self, layer: LayerKey, spec: LayerSpec) -> LayerBook {
        let mut layers = self.layers;
        layers.insert(layer, spec);
        LayerBook { layers, ..self }
    }

    /// The book with `layer` put with its top-left at `at`, wherever its anchors would have
    /// put it.
    pub fn with_pin(self, layer: LayerKey, at: LogicalPoint) -> LayerBook {
        let mut pins = self.pins;
        pins.insert(layer, at);
        LayerBook { pins, ..self }
    }

    /// The book without `layer`.
    pub fn without_layer(self, layer: &LayerKey) -> LayerBook {
        let mut layers = self.layers;
        layers.remove(layer);
        let mut pins = self.pins;
        pins.remove(layer);
        LayerBook { layers, pins }
    }

    /// A surface that holds the shell role `role`, if one does: the lowest key when a bar or a
    /// dock is held on several outputs (use [`LayerBook::role_holder_on`] to name the output).
    pub fn role_holder(&self, role: ShellRole) -> Option<LayerKey> {
        self.layers
            .iter()
            .find(|(_, spec)| spec.role == ShellRoleTag::Assigned(role))
            .map(|(key, _)| *key)
    }

    /// The surface that holds `role` for `output`: on that output for a bar or a dock (each
    /// output has its own), the one holder of any other role.
    pub fn role_holder_on(&self, role: ShellRole, output: &OutputKey) -> Option<LayerKey> {
        self.layers
            .iter()
            .find(|(_, spec)| holds(spec, role, output))
            .map(|(key, _)| *key)
    }

    /// The surface that serves `output` as `role`: the one on `output`, else (a bar or a dock on
    /// no such output) any other holder.
    pub fn role_holder_near(&self, role: ShellRole, output: &OutputKey) -> Option<LayerKey> {
        self.role_holder_on(role, output)
            .or_else(|| self.role_holder(role))
    }

    /// Whether a surface other than `layer` already holds `role` where `layer` (on `output`)
    /// would take it: on the same output for a bar or a dock, anywhere for any other role.
    pub fn role_taken(&self, role: ShellRole, layer: LayerKey, output: &OutputKey) -> bool {
        self.layers
            .iter()
            .any(|(key, spec)| *key != layer && holds(spec, role, output))
    }

    /// The rectangle a full-screen window covers on `output` (whose whole rectangle is `full`)
    /// under `mode`: all of it with `Reveal`, less the exclusive zone of the output's bar with
    /// `Keep` (the dock's zone is not taken: the dock still reveals over the window).
    pub fn full_screen_area(
        &self,
        mode: FullScreenBar,
        output: &OutputKey,
        full: LogicalRect,
    ) -> LogicalRect {
        match mode {
            FullScreenBar::Reveal => full,
            FullScreenBar::Keep => {
                let bars = LayerBook {
                    layers: self
                        .layers
                        .iter()
                        .filter(|(_, spec)| {
                            spec.output == *output
                                && spec.role == ShellRoleTag::Assigned(ShellRole::Bar)
                        })
                        .map(|(key, spec)| (*key, spec.clone()))
                        .collect(),
                    pins: BTreeMap::new(),
                };
                bars.usable_area(output, full)
            }
        }
    }

    /// Where `layer` was put by hand, if it was.
    pub fn pin(&self, layer: &LayerKey) -> Option<LogicalPoint> {
        self.pins.get(layer).copied()
    }

    /// What the surface `layer` asked for.
    pub fn get(&self, layer: &LayerKey) -> Option<&LayerSpec> {
        self.layers.get(layer)
    }

    /// Every mapped surface, by key.
    pub fn iter(&self) -> impl Iterator<Item = (&LayerKey, &LayerSpec)> {
        self.layers.iter()
    }

    /// What `full` (an output's whole logical rectangle) leaves for windows once every layer
    /// surface on `output` that reserves an exclusive zone has taken it ([`LayerBook::arrange`]).
    /// Maximised windows take this rectangle too (A16), so a bar and a dock are never covered.
    pub fn usable_area(&self, output: &OutputKey, full: LogicalRect) -> LogicalRect {
        self.arrange(output, full).usable()
    }
}

/// Whether `spec` holds `role` for `output`: a bar or a dock is held once per output, every
/// other role once overall.
fn holds(spec: &LayerSpec, role: ShellRole, output: &OutputKey) -> bool {
    let here = match role {
        ShellRole::Bar | ShellRole::Dock => spec.output == *output,
        _ => true,
    };
    spec.role == ShellRoleTag::Assigned(role) && here
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::{LogicalPoint, LogicalSize};
    use crate::ids::OutputName;
    use crate::layer::{
        Anchors, ExclusiveZone, KeyboardInteractivity, LayerLevel, Margins, ShellRoleTag,
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
        let held = |c: char| edges.contains(c);
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
            size: crate::geometry::LogicalSize::ZERO,
            role: ShellRoleTag::Unassigned,
            owner: crate::layer::LayerOwner::Shell,
        }
    }

    #[test]
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

    #[test]
    fn a_bar_and_a_dock_are_held_once_per_output() {
        let held = |on: &str, edges: &str, role: ShellRole| LayerSpec {
            role: ShellRoleTag::Assigned(role),
            ..layer(on, edges, ExclusiveZone::Reserve(24), NO_MARGIN)
        };
        let book = LayerBook::empty()
            .with_layer(LayerKey(1), held("HEADLESS-1", "tlr", ShellRole::Bar))
            .with_layer(LayerKey(2), held("HEADLESS-1", "blr", ShellRole::Dock))
            .with_layer(LayerKey(3), held("HEADLESS-2", "tlr", ShellRole::Bar))
            .with_layer(LayerKey(4), held("HEADLESS-1", "tlr", ShellRole::Switcher));
        let (one, two) = (output("HEADLESS-1"), output("HEADLESS-2"));
        // name, role, the asking layer, its output, taken
        let cases = [
            (
                "a second bar on the same output",
                ShellRole::Bar,
                5,
                &one,
                true,
            ),
            ("a bar on the other output", ShellRole::Bar, 5, &two, true),
            (
                "a dock on the other output",
                ShellRole::Dock,
                5,
                &two,
                false,
            ),
            ("the holder itself", ShellRole::Bar, 1, &one, false),
            (
                "a second dock on the same output",
                ShellRole::Dock,
                5,
                &one,
                true,
            ),
            (
                "another role stays global",
                ShellRole::Switcher,
                5,
                &two,
                true,
            ),
        ];
        for (name, role, asking, on, taken) in cases {
            assert_eq!(book.role_taken(role, LayerKey(asking), on), taken, "{name}");
        }
        assert_eq!(book.role_holder_on(ShellRole::Bar, &two), Some(LayerKey(3)));
        assert_eq!(book.role_holder_on(ShellRole::Dock, &two), None);
        assert_eq!(
            book.role_holder_near(ShellRole::Dock, &two),
            Some(LayerKey(2)),
            "a dock on no such output falls back to any dock"
        );
        assert_eq!(
            book.role_holder_on(ShellRole::Dock, &one),
            Some(LayerKey(2))
        );
    }

    #[test]
    fn keep_mode_sizes_a_full_screen_window_below_the_bar_only() {
        let full = rect(0, 0, 1920, 1080);
        let book = LayerBook::empty()
            .with_layer(
                LayerKey(1),
                LayerSpec {
                    role: ShellRoleTag::Assigned(ShellRole::Bar),
                    ..layer("HEADLESS-1", "tlr", ExclusiveZone::Reserve(32), NO_MARGIN)
                },
            )
            .with_layer(
                LayerKey(2),
                LayerSpec {
                    role: ShellRoleTag::Assigned(ShellRole::Dock),
                    ..layer("HEADLESS-1", "blr", ExclusiveZone::Reserve(80), NO_MARGIN)
                },
            );
        let on = output("HEADLESS-1");
        assert_eq!(
            book.full_screen_area(FullScreenBar::Reveal, &on, full),
            full
        );
        assert_eq!(
            book.full_screen_area(FullScreenBar::Keep, &on, full),
            rect(0, 32, 1920, 1048)
        );
        assert_eq!(
            book.full_screen_area(FullScreenBar::Keep, &output("HEADLESS-2"), full),
            full,
            "another output's bar takes nothing here"
        );
    }
}
