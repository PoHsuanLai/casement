//! Layer surfaces through `Model::step`, and who holds the keyboard because of them. Every row
//! runs with the invariants checked after each step.

mod support;

use casement_core::focus::TokenCheck;
use casement_core::geometry::{LogicalPoint, LogicalRect, LogicalSize, Transform};
use casement_core::layer::{
    Anchor, Anchors, ExclusiveZone, KeyboardInteractivity, LayerLevel, LayerSpec, Margins,
    ShellRoleTag,
};

use casement_core::{Cause, Effect, Event, KeyboardTarget, LayerKey, OutputKey, WindowId};
use support::driver::{Op, Run, assert_same_world, last, map, plug, run, run_from, unplug};
use support::{headless, proven};

fn key(name: &str) -> OutputKey {
    headless(name).key
}

fn bar(output: &str, zone: ExclusiveZone) -> LayerSpec {
    LayerSpec {
        output: key(output),
        level: LayerLevel::Top,
        anchor: Anchors {
            top: Anchor::Held,
            bottom: Anchor::Free,
            left: Anchor::Held,
            right: Anchor::Held,
        },
        exclusive: zone,
        margin: Margins {
            top: 0,
            right: 0,
            bottom: 0,
            left: 0,
        },
        keyboard: KeyboardInteractivity::None,
        namespace: "bar".to_owned(),
        size: casement_core::geometry::LogicalSize::ZERO,
        role: ShellRoleTag::Unassigned,
    }
}

fn mapped(layer: u64, spec: LayerSpec) -> Op {
    Op::Event(Event::LayerMapped {
        layer: LayerKey(layer),
        spec,
    })
}

fn changed(layer: u64, spec: LayerSpec) -> Op {
    Op::Event(Event::LayerChanged {
        layer: LayerKey(layer),
        spec,
    })
}

fn unmapped(layer: u64) -> Op {
    Op::Event(Event::LayerUnmapped {
        layer: LayerKey(layer),
    })
}

/// A 1920x1080 output at the origin, as `headless` plugs it.
fn full() -> LogicalRect {
    LogicalRect::new(
        LogicalPoint { x: 0, y: 0 },
        LogicalSize { w: 1920, h: 1080 },
    )
}

fn usable(run: &Run, output: &str) -> LogicalRect {
    run.model.layers().usable_area(&key(output), full())
}

#[test]
fn a_bar_is_told_its_size_and_takes_room_from_windows() {
    let before = run(vec![plug("HEADLESS-1")]);
    let run = run_from(
        before.model,
        vec![mapped(1, bar("HEADLESS-1", ExclusiveZone::Reserve(28)))],
    );
    assert_eq!(
        usable(&run, "HEADLESS-1"),
        LogicalRect::new(
            LogicalPoint { x: 0, y: 28 },
            LogicalSize { w: 1920, h: 1052 }
        )
    );
    let effects = last(&run);
    assert!(effects.contains(&Effect::ConfigureLayer {
        layer: LayerKey(1),
        size: LogicalSize { w: 1920, h: 0 }
    }));
    assert!(effects.contains(&Effect::Relayout(Cause::Configure)));
    assert!(effects.contains(&Effect::RequestFrames(key("HEADLESS-1"))));
}

#[test]
fn a_layer_that_reserves_nothing_does_not_lay_windows_out_again() {
    let run = run(vec![
        plug("HEADLESS-1"),
        mapped(1, bar("HEADLESS-1", ExclusiveZone::Ignore)),
    ]);
    assert!(!last(&run).iter().any(|e| matches!(e, Effect::Relayout(_))));
    assert!(
        last(&run)
            .iter()
            .any(|e| matches!(e, Effect::ConfigureLayer { .. }))
    );
}

#[test]
fn layer_sizes_follow_their_anchors_and_margins() {
    let held = |a: bool| if a { Anchor::Held } else { Anchor::Free };
    let cases = [
        (
            "both ends of both axes",
            [true, true, true, true],
            (10, 20, 30, 40),
            (1860, 1040),
        ),
        (
            "left and right only",
            [false, false, true, true],
            (0, 0, 0, 0),
            (1920, 0),
        ),
        (
            "one corner",
            [true, false, true, false],
            (0, 0, 0, 0),
            (0, 0),
        ),
        (
            "top and bottom only",
            [true, true, false, false],
            (5, 0, 5, 0),
            (0, 1070),
        ),
    ];
    for (name, [top, bottom, left, right], (mt, mr, mb, ml), (w, h)) in cases {
        let spec = LayerSpec {
            anchor: Anchors {
                top: held(top),
                bottom: held(bottom),
                left: held(left),
                right: held(right),
            },
            margin: Margins {
                top: mt,
                right: mr,
                bottom: mb,
                left: ml,
            },
            ..bar("HEADLESS-1", ExclusiveZone::Ignore)
        };
        let run = run(vec![plug("HEADLESS-1"), mapped(1, spec)]);
        assert!(
            last(&run).contains(&Effect::ConfigureLayer {
                layer: LayerKey(1),
                size: LogicalSize { w, h }
            }),
            "{name}"
        );
    }
}

#[test]
fn changing_and_unmapping_a_layer_give_the_room_back() {
    let base = run(vec![
        plug("HEADLESS-1"),
        mapped(1, bar("HEADLESS-1", ExclusiveZone::Reserve(28))),
    ]);
    let grown = run_from(
        base.model.clone(),
        vec![changed(1, bar("HEADLESS-1", ExclusiveZone::Reserve(40)))],
    );
    assert_eq!(usable(&grown, "HEADLESS-1").origin.y, 40);
    assert!(last(&grown).contains(&Effect::Relayout(Cause::Configure)));

    let gone = run_from(base.model.clone(), vec![unmapped(1)]);
    assert_eq!(usable(&gone, "HEADLESS-1"), full());
    assert!(last(&gone).contains(&Effect::Relayout(Cause::Configure)));

    let never = run_from(
        base.model.clone(),
        vec![
            unmapped(9),
            changed(9, bar("HEADLESS-1", ExclusiveZone::Reserve(9))),
        ],
    );
    assert_same_world(&never.model, &base.model, "unknown layers");
}

#[test]
fn a_layer_on_an_output_that_is_not_plugged_is_not_mapped() {
    let base = run(vec![plug("HEADLESS-1")]);
    let run = run_from(
        base.model.clone(),
        vec![mapped(1, bar("HEADLESS-2", ExclusiveZone::Reserve(28)))],
    );
    assert_same_world(&run.model, &base.model, "unplugged output");
    assert_eq!(last(&run), []);
}

#[test]
fn unplugging_an_output_takes_its_layers_with_it() {
    let run = run(vec![
        plug("HEADLESS-1"),
        plug("HEADLESS-2"),
        mapped(1, bar("HEADLESS-1", ExclusiveZone::Reserve(28))),
        mapped(2, bar("HEADLESS-2", ExclusiveZone::Reserve(28))),
        unplug("HEADLESS-2"),
    ]);
    let layers: Vec<_> = run.model.layers().iter().map(|(k, _)| *k).collect();
    assert_eq!(layers, vec![LayerKey(1)]);
}

#[test]
fn an_exclusive_layer_takes_the_keyboard_on_the_same_step_and_gives_it_back() {
    let mail = proven("org.quire.Mail");
    let launcher = LayerSpec {
        level: LayerLevel::Overlay,
        keyboard: KeyboardInteractivity::Exclusive,
        ..bar("HEADLESS-1", ExclusiveZone::Ignore)
    };
    let base = run(vec![plug("HEADLESS-1"), map(1, &mail, TokenCheck::Valid)]);
    assert_eq!(
        base.model.keyboard_target(),
        KeyboardTarget::Window(WindowId(1))
    );

    let shown = run_from(base.model, vec![mapped(1, launcher)]);
    assert_eq!(
        shown.model.keyboard_target(),
        KeyboardTarget::Layer(LayerKey(1))
    );
    assert!(last(&shown).contains(&Effect::Keyboard(KeyboardTarget::Layer(LayerKey(1)))));
    assert_eq!(
        shown.model.focus().key,
        Some(WindowId(1)),
        "focus did not move"
    );

    let gone = run_from(shown.model, vec![unmapped(1)]);
    assert!(last(&gone).contains(&Effect::Keyboard(KeyboardTarget::Window(WindowId(1)))));
}

#[test]
fn among_exclusive_layers_the_highest_level_then_the_newest_holds_the_keyboard() {
    let exclusive = |level| LayerSpec {
        level,
        keyboard: KeyboardInteractivity::Exclusive,
        ..bar("HEADLESS-1", ExclusiveZone::Ignore)
    };
    let on_demand = LayerSpec {
        level: LayerLevel::Overlay,
        keyboard: KeyboardInteractivity::OnDemand,
        ..bar("HEADLESS-1", ExclusiveZone::Ignore)
    };
    let cases = [
        (
            "a Bottom layer never",
            vec![(1, exclusive(LayerLevel::Bottom))],
            KeyboardTarget::Nothing,
        ),
        (
            "on demand takes the keyboard on mapping when no window holds it",
            vec![(1, on_demand)],
            KeyboardTarget::Layer(LayerKey(1)),
        ),
        (
            "the newer of one level",
            vec![
                (1, exclusive(LayerLevel::Top)),
                (2, exclusive(LayerLevel::Top)),
            ],
            KeyboardTarget::Layer(LayerKey(2)),
        ),
        (
            "Overlay over a newer Top",
            vec![
                (1, exclusive(LayerLevel::Overlay)),
                (2, exclusive(LayerLevel::Top)),
            ],
            KeyboardTarget::Layer(LayerKey(1)),
        ),
    ];
    for (name, layers, want) in cases {
        let mut ops = vec![plug("HEADLESS-1")];
        ops.extend(layers.into_iter().map(|(k, spec)| mapped(k, spec)));
        assert_eq!(run(ops).model.keyboard_target(), want, "{name}");
    }
}

/// A surface held to all four edges that asks for zone 0 (a notification centre).
fn centre(output: &str) -> LayerSpec {
    LayerSpec {
        anchor: Anchors {
            top: Anchor::Held,
            bottom: Anchor::Held,
            left: Anchor::Held,
            right: Anchor::Held,
        },
        level: LayerLevel::Overlay,
        namespace: "centre".to_owned(),
        size: casement_core::geometry::LogicalSize::ZERO,
        ..bar(output, ExclusiveZone::Respect)
    }
}

fn configured(layer: u64, w: u32, h: u32) -> Effect {
    Effect::ConfigureLayer {
        layer: LayerKey(layer),
        size: LogicalSize { w, h },
    }
}

#[test]
fn a_zone_zero_layer_is_sized_inside_the_usable_area() {
    let run = run(vec![
        plug("HEADLESS-1"),
        mapped(1, bar("HEADLESS-1", ExclusiveZone::Reserve(28))),
        mapped(2, centre("HEADLESS-1")),
    ]);
    assert!(last(&run).contains(&configured(2, 1920, 1052)));
}

#[test]
fn a_zone_zero_layer_follows_the_zones_that_come_change_and_go() {
    let base = run(vec![
        plug("HEADLESS-1"),
        mapped(2, centre("HEADLESS-1")),
        mapped(3, centre("HEADLESS-1")),
    ]);
    assert!(last(&base).contains(&configured(3, 1920, 1080)));
    let steps = [
        (
            "a bar appears",
            mapped(1, bar("HEADLESS-1", ExclusiveZone::Reserve(28))),
            1052,
        ),
        (
            "the bar grows",
            changed(1, bar("HEADLESS-1", ExclusiveZone::Reserve(40))),
            1040,
        ),
        ("the bar goes", unmapped(1), 1080),
    ];
    let mut model = base.model;
    for (name, op, height) in steps {
        let run = run_from(model, vec![op]);
        for layer in [2, 3] {
            assert!(
                last(&run).contains(&configured(layer, 1920, height)),
                "{name}: layer {layer}"
            );
        }
        model = run.model;
    }
}

#[test]
fn zones_that_do_not_move_a_layer_do_not_reconfigure_it() {
    let base = run(vec![
        plug("HEADLESS-1"),
        mapped(2, bar("HEADLESS-1", ExclusiveZone::Ignore)),
        plug("HEADLESS-2"),
        mapped(3, centre("HEADLESS-2")),
    ]);
    let run = run_from(
        base.model,
        vec![mapped(1, bar("HEADLESS-1", ExclusiveZone::Reserve(28)))],
    );
    let others: Vec<_> = last(&run)
        .iter()
        .filter(|e| matches!(e, Effect::ConfigureLayer { layer, .. } if *layer != LayerKey(1)))
        .collect();
    assert!(others.is_empty(), "{others:?}");
}

fn dock(output: &str, zone: ExclusiveZone) -> LayerSpec {
    LayerSpec {
        anchor: Anchors {
            top: Anchor::Held,
            bottom: Anchor::Held,
            left: Anchor::Held,
            right: Anchor::Free,
        },
        namespace: "dock".to_owned(),
        size: casement_core::geometry::LogicalSize::ZERO,
        ..bar(output, zone)
    }
}

#[test]
fn an_exclusive_layer_is_sized_in_what_the_earlier_zones_left() {
    // The bar is arranged first (key order within a level): the dock is sized below it.
    let bar_first = run(vec![
        plug("HEADLESS-1"),
        mapped(1, bar("HEADLESS-1", ExclusiveZone::Reserve(28))),
        mapped(2, dock("HEADLESS-1", ExclusiveZone::Reserve(64))),
    ]);
    assert!(last(&bar_first).contains(&configured(2, 0, 1080 - 28)));
    // The dock is arranged first: the bar already mapped is told it is narrower.
    let dock_first = run_from(
        run(vec![
            plug("HEADLESS-1"),
            mapped(2, bar("HEADLESS-1", ExclusiveZone::Reserve(28))),
        ])
        .model,
        vec![mapped(1, dock("HEADLESS-1", ExclusiveZone::Reserve(64)))],
    );
    assert!(last(&dock_first).contains(&configured(1, 0, 1080)));
    assert!(last(&dock_first).contains(&configured(2, 1920 - 64, 0)));
    // A zone that grows tells those arranged after it, and the usable area follows both.
    let grown = run_from(
        dock_first.model,
        vec![changed(1, dock("HEADLESS-1", ExclusiveZone::Reserve(100)))],
    );
    assert!(last(&grown).contains(&configured(2, 1920 - 100, 0)));
    assert_eq!(
        usable(&grown, "HEADLESS-1"),
        LogicalRect::new(
            LogicalPoint { x: 100, y: 28 },
            LogicalSize { w: 1820, h: 1052 }
        )
    );
}

#[test]
fn a_layer_is_told_its_size_again_when_its_output_changes() {
    let plugged = run(vec![
        plug("HEADLESS-1"),
        mapped(1, bar("HEADLESS-1", ExclusiveZone::Reserve(28))),
    ]);
    let mut config = plugged.model.outputs().0[&key("HEADLESS-1")].config;
    config.transform = Transform::R90;
    let rotated = run_from(
        plugged.model,
        vec![Op::Event(Event::OutputConfigured {
            output: key("HEADLESS-1"),
            config,
        })],
    );
    assert!(last(&rotated).contains(&configured(1, 1080, 0)));
}
