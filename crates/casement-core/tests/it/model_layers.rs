//! Layer surfaces through `Model::step`, and who holds the keyboard because of them. Every row
//! runs with the invariants checked after each step.

use crate::support;

use casement_core::focus::TokenCheck;
use casement_core::geometry::{LogicalPoint, LogicalRect, LogicalSize, Transform};
use casement_core::layer::{
    Anchors, ExclusiveZone, KeyboardInteractivity, LayerLevel, LayerOwner, LayerSpec, Margins,
    ShellRoleTag,
};

use casement_core::lock::LockEvent;
use casement_core::{
    Cause, ClientKey, Effect, Event, KeyboardTarget, LayerKey, OutputKey, WindowId,
};
use support::driver::{Op, Run, assert_same_world, last, map, plug, run, run_from, unplug};
use support::{headless, proven};

fn key(name: &str) -> OutputKey {
    headless(name).key
}

/// A bar of the shell's own: the keyboard and zone tables below are about its layers; another
/// client's are `client(..)`.
fn bar(output: &str, zone: ExclusiveZone) -> LayerSpec {
    LayerSpec {
        output: key(output),
        level: LayerLevel::Top,
        anchor: Anchors {
            top: true,
            bottom: false,
            left: true,
            right: true,
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
        owner: LayerOwner::Shell,
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
                top,
                bottom,
                left,
                right,
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
            top: true,
            bottom: true,
            left: true,
            right: true,
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
            top: true,
            bottom: true,
            left: true,
            right: false,
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

/// One step of a keyboard ordering case.
enum Step {
    /// An op through the driver.
    Do(Op),
    /// A press on layer `n`.
    Click(u64),
}

fn panel(level: LayerLevel, keyboard: KeyboardInteractivity) -> LayerSpec {
    LayerSpec {
        level,
        keyboard,
        ..bar("HEADLESS-1", ExclusiveZone::Respect)
    }
}

/// A panel a person just opened takes the keyboard when it maps, over a key window, unless an
/// exclusive layer or the lock holds it; it gives it back to the key window when it unmaps.
#[test]
fn an_on_demand_panel_takes_the_keyboard_on_mapping_and_gives_it_back() {
    let mail = proven("org.quire.Mail");
    let on_demand = |level| panel(level, KeyboardInteractivity::OnDemand);
    let exclusive = panel(LayerLevel::Overlay, KeyboardInteractivity::Exclusive);
    let window = || KeyboardTarget::Window(WindowId(1));
    let layer = |n| KeyboardTarget::Layer(LayerKey(n));
    let cases: Vec<(&str, Vec<Step>, KeyboardTarget)> = vec![
        (
            "maps over a key window and takes the keyboard",
            vec![Step::Do(mapped(1, on_demand(LayerLevel::Top)))],
            layer(1),
        ),
        (
            "unmaps and the key window has it back",
            vec![
                Step::Do(mapped(1, on_demand(LayerLevel::Top))),
                Step::Do(unmapped(1)),
            ],
            window(),
        ),
        (
            "a newer panel takes it, and its unmap returns it to the key window",
            vec![
                Step::Do(mapped(1, on_demand(LayerLevel::Top))),
                Step::Do(mapped(2, on_demand(LayerLevel::Overlay))),
                Step::Do(unmapped(2)),
            ],
            window(),
        ),
        (
            "a wallpaper that asks on demand does not steal it",
            vec![Step::Do(mapped(1, on_demand(LayerLevel::Background)))],
            window(),
        ),
        (
            "a layer that asks for none does not take it",
            vec![Step::Do(mapped(
                1,
                panel(LayerLevel::Top, KeyboardInteractivity::None),
            ))],
            window(),
        ),
        (
            "an exclusive layer already holding it keeps it",
            vec![
                Step::Do(mapped(1, exclusive.clone())),
                Step::Do(mapped(2, on_demand(LayerLevel::Top))),
            ],
            layer(1),
        ),
        (
            "an exclusive layer that maps later takes it from the panel",
            vec![
                Step::Do(mapped(1, on_demand(LayerLevel::Top))),
                Step::Do(mapped(2, exclusive.clone())),
            ],
            layer(2),
        ),
        (
            "the panel has it again when the exclusive layer unmaps",
            vec![
                Step::Do(mapped(1, on_demand(LayerLevel::Top))),
                Step::Do(mapped(2, exclusive)),
                Step::Do(unmapped(2)),
            ],
            layer(1),
        ),
        (
            "a panel that maps under the lock does not take it",
            vec![
                Step::Do(Op::Event(Event::Lock(LockEvent::Requested {
                    client: ClientKey(7),
                }))),
                Step::Do(mapped(1, on_demand(LayerLevel::Top))),
                Step::Do(Op::Event(Event::Lock(LockEvent::Unlocked))),
            ],
            window(),
        ),
        (
            "a click on a panel that never took it gives it the keyboard",
            vec![
                Step::Do(mapped(1, on_demand(LayerLevel::Background))),
                Step::Click(1),
            ],
            layer(1),
        ),
        (
            "a click on a layer that asks for none changes nothing",
            vec![
                Step::Do(mapped(
                    1,
                    panel(LayerLevel::Top, KeyboardInteractivity::None),
                )),
                Step::Click(1),
            ],
            window(),
        ),
    ];
    for (name, steps, want) in cases {
        let base = run(vec![plug("HEADLESS-1"), map(1, &mail, TokenCheck::Valid)]);
        let model = steps
            .into_iter()
            .fold(base.model, |model, step| match step {
                Step::Do(op) => run_from(model, vec![op]).model,
                Step::Click(n) => {
                    let at = model.now().after(std::time::Duration::from_millis(1));
                    let (model, _) = model.press_layer(LayerKey(n), at);
                    model
                }
            });
        assert_eq!(model.keyboard_target(), want, "{name}");
    }
}

/// `spec` from another client's connection instead of the shell's.
fn client(spec: LayerSpec) -> LayerSpec {
    LayerSpec {
        owner: LayerOwner::Client,
        ..spec
    }
}

/// CASC-02: a layer surface of another client never takes the keyboard from the key window by
/// mapping, however it asks and however high it sits; a press on it gives it the keyboard, and
/// the shell's exclusive layer outranks even a pressed one.
#[test]
fn another_clients_layer_takes_the_keyboard_only_when_pressed() {
    let mail = proven("org.quire.Mail");
    let window = KeyboardTarget::Window(WindowId(1));
    let layer = |n| KeyboardTarget::Layer(LayerKey(n));
    let asks = [
        (
            "exclusive overlay",
            LayerLevel::Overlay,
            KeyboardInteractivity::Exclusive,
        ),
        (
            "exclusive top",
            LayerLevel::Top,
            KeyboardInteractivity::Exclusive,
        ),
        (
            "on demand overlay",
            LayerLevel::Overlay,
            KeyboardInteractivity::OnDemand,
        ),
        (
            "on demand top",
            LayerLevel::Top,
            KeyboardInteractivity::OnDemand,
        ),
    ];
    for (name, level, keyboard) in asks {
        let base = run(vec![plug("HEADLESS-1"), map(1, &mail, TokenCheck::Valid)]);
        let shown = run_from(base.model, vec![mapped(1, client(panel(level, keyboard)))]);
        assert_eq!(
            shown.model.keyboard_target(),
            window,
            "{name}: mapping takes nothing"
        );
        assert!(
            !last(&shown)
                .iter()
                .any(|e| matches!(e, Effect::Keyboard(_))),
            "{name}: no keyboard effect"
        );
        let at = shown.model.now().after(std::time::Duration::from_millis(1));
        let (pressed, effects) = shown.model.press_layer(LayerKey(1), at);
        assert_eq!(
            pressed.keyboard_target(),
            layer(1),
            "{name}: a press gives it"
        );
        assert!(effects.contains(&Effect::Keyboard(layer(1))), "{name}");

        // The shell's exclusive layer outranks the pressed one, and its unmap gives it back.
        let at = pressed.now().after(std::time::Duration::from_millis(1));
        let (outranked, _) = pressed.step(
            Event::LayerMapped {
                layer: LayerKey(2),
                spec: panel(LayerLevel::Top, KeyboardInteractivity::Exclusive),
            },
            at,
        );
        assert_eq!(
            outranked.keyboard_target(),
            layer(2),
            "{name}: the shell's wins"
        );
    }
}
