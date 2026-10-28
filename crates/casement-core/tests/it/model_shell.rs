//! The shell's requests through `Model::step` (P2-Z): which layer surface holds a role, and where
//! a dock's tiles are. Owned by P2-Q2 (the policy for a minimise target the dock and a
//! window-list client both set).

use crate::support::driver::{Op, last, map, plug, run, run_from};
use crate::support::{headless, proven, rect};
use casement_core::focus::TokenCheck;
use casement_core::geometry::{LogicalPoint, LogicalSize};
use casement_core::layer::{
    Anchors, BlurStrength, ExclusiveZone, KeyboardInteractivity, LayerLevel, LayerSpec, Margins,
    ShellRoleTag,
};
use casement_core::reveal::FullScreenBar;
use casement_core::shell::{ShellNote, ShellRequest};
use casement_core::window::WlrToplevelRequest;
use casement_core::{Cause, Effect, Event, LayerKey, WindowId};
use casement_protocols::vocab::{Motion, ShellRole};

fn layer(layer_key: u64, spec_role: ShellRoleTag) -> Op {
    layer_on("HEADLESS-1", layer_key, spec_role)
}

fn layer_on(output: &str, layer_key: u64, spec_role: ShellRoleTag) -> Op {
    let spec = LayerSpec {
        output: headless(output).key,
        level: LayerLevel::Top,
        anchor: Anchors {
            top: false,
            bottom: true,
            left: false,
            right: false,
        },
        exclusive: ExclusiveZone::Ignore,
        margin: Margins {
            top: 0,
            right: 0,
            bottom: 0,
            left: 0,
        },
        keyboard: KeyboardInteractivity::None,
        namespace: "dock".to_owned(),
        size: LogicalSize { w: 400, h: 80 },
        role: spec_role,
        owner: casement_core::layer::LayerOwner::Shell,
    };
    Op::Event(Event::LayerMapped {
        layer: LayerKey(layer_key),
        spec,
    })
}

fn shell(request: ShellRequest) -> Op {
    Op::Event(Event::Shell(request))
}

#[test]
fn a_shell_role_is_held_by_one_surface() {
    let first = shell(ShellRequest::Role {
        layer: LayerKey(1),
        role: ShellRole::Dock,
    });
    let second = shell(ShellRequest::Role {
        layer: LayerKey(2),
        role: ShellRole::Dock,
    });
    let taken = run(vec![
        plug("HEADLESS-1"),
        layer(1, ShellRoleTag::Unassigned),
        layer(2, ShellRoleTag::Unassigned),
        first,
    ]);
    assert!(last(&taken).contains(&Effect::Shell(ShellNote::Role(
        LayerKey(1),
        ShellRole::Dock
    ))));
    assert_eq!(
        taken.model.layers().role_holder(ShellRole::Dock),
        Some(LayerKey(1))
    );
    let again = run_from(taken.model, vec![second]);
    assert_eq!(
        again.model.layers().role_holder(ShellRole::Dock),
        Some(LayerKey(1)),
        "a role another surface holds is not given twice"
    );
    assert!(last(&again).is_empty());
}

#[test]
fn a_bar_and_a_dock_are_held_once_per_output() {
    let role = |layer: u64, role: ShellRole| {
        shell(ShellRequest::Role {
            layer: LayerKey(layer),
            role,
        })
    };
    let held = run(vec![
        plug("HEADLESS-1"),
        plug("HEADLESS-2"),
        layer_on("HEADLESS-1", 1, ShellRoleTag::Unassigned),
        layer_on("HEADLESS-2", 2, ShellRoleTag::Unassigned),
        layer_on("HEADLESS-1", 3, ShellRoleTag::Unassigned),
        role(1, ShellRole::Bar),
        role(2, ShellRole::Bar),
    ]);
    let layers = held.model.layers();
    let (one, two) = (headless("HEADLESS-1").key, headless("HEADLESS-2").key);
    assert_eq!(
        layers.role_holder_on(ShellRole::Bar, &one),
        Some(LayerKey(1))
    );
    assert_eq!(
        layers.role_holder_on(ShellRole::Bar, &two),
        Some(LayerKey(2))
    );
    assert_eq!(layers.role_holder_near(ShellRole::Dock, &two), None);

    let again = run_from(held.model, vec![role(3, ShellRole::Bar)]);
    assert_eq!(
        again.model.layers().role_holder_on(ShellRole::Bar, &one),
        Some(LayerKey(1)),
        "a second bar on the same output is not given the role"
    );
    assert!(last(&again).is_empty());
}

#[test]
fn the_shell_chooses_the_bar_in_full_screen_and_the_motion_the_last_one_wins() {
    let kept = run(vec![
        plug("HEADLESS-1"),
        shell(ShellRequest::FullScreenBar(FullScreenBar::Keep)),
    ]);
    assert_eq!(kept.model.full_screen_bar(), FullScreenBar::Keep);
    assert!(
        last(&kept).contains(&Effect::Relayout(Cause::Configure)),
        "the full-screen windows are laid out again"
    );
    let same = run_from(
        kept.model,
        vec![shell(ShellRequest::FullScreenBar(FullScreenBar::Keep))],
    );
    assert!(last(&same).is_empty(), "the same choice changes nothing");
    let back = run_from(
        same.model,
        vec![shell(ShellRequest::FullScreenBar(FullScreenBar::Reveal))],
    );
    assert_eq!(back.model.full_screen_bar(), FullScreenBar::Reveal);
    assert!(last(&back).contains(&Effect::Relayout(Cause::Configure)));

    assert!(!back.model.reduced_motion(), "full until told otherwise");
    let reduced = run_from(
        back.model,
        vec![shell(ShellRequest::Motion(Motion::Reduced))],
    );
    assert!(reduced.model.reduced_motion());
    let full = run_from(
        reduced.model,
        vec![shell(ShellRequest::Motion(Motion::Full))],
    );
    assert!(!full.model.reduced_motion());
}

#[test]
fn a_minimise_target_is_local_to_the_dock_and_stored_globally() {
    let mail = proven("org.quire.Mail");
    let placed = run(vec![
        plug("HEADLESS-1"),
        layer(1, ShellRoleTag::Unassigned),
        Op::Event(Event::LayerPlaced {
            layer: LayerKey(1),
            at: LogicalPoint { x: 100, y: 900 },
        }),
        map(1, &mail, TokenCheck::Missing),
        shell(ShellRequest::MinimiseTarget {
            dock: LayerKey(1),
            window: WindowId(1),
            local: rect(10, 5, 64, 64),
        }),
    ]);
    let stored = placed.model.minimise_target(WindowId(1)).expect("a target");
    assert_eq!(stored.rect, rect(110, 905, 64, 64));
    assert!(
        last(&placed).contains(&Effect::Shell(ShellNote::MinimiseTarget(
            WindowId(1),
            rect(110, 905, 64, 64)
        )))
    );
    let cleared = run_from(
        placed.model,
        vec![shell(ShellRequest::ClearMinimiseTarget {
            window: WindowId(1),
        })],
    );
    assert!(cleared.model.minimise_target(WindowId(1)).is_none());
}

#[test]
fn the_docks_target_wins_over_a_window_list_clients_while_a_dock_holds_a_role() {
    let mail = proven("org.quire.Mail");
    let wlr_tile = |x: i32| {
        Op::Event(Event::WlrToplevel {
            window: WindowId(1),
            request: WlrToplevelRequest::MinimiseRect(rect(x, 900, 64, 64)),
        })
    };
    let before_dock = run(vec![
        plug("HEADLESS-1"),
        map(1, &mail, TokenCheck::Missing),
        wlr_tile(700),
    ]);
    assert_eq!(
        before_dock
            .model
            .minimise_target(WindowId(1))
            .map(|t| t.rect),
        Some(rect(700, 900, 64, 64)),
        "with no dock the window-list client's rectangle is the target"
    );
    let docked = run_from(
        before_dock.model,
        vec![
            layer(1, ShellRoleTag::Unassigned),
            Op::Event(Event::LayerPlaced {
                layer: LayerKey(1),
                at: LogicalPoint { x: 100, y: 900 },
            }),
            shell(ShellRequest::Role {
                layer: LayerKey(1),
                role: ShellRole::Dock,
            }),
            shell(ShellRequest::MinimiseTarget {
                dock: LayerKey(1),
                window: WindowId(1),
                local: rect(10, 5, 64, 64),
            }),
            wlr_tile(300),
        ],
    );
    assert_eq!(
        docked.model.minimise_target(WindowId(1)).map(|t| t.rect),
        Some(rect(110, 905, 64, 64)),
        "the dock's tile stays the target when the window-list client asks later"
    );
}

#[test]
fn a_window_minimises_toward_its_tile_else_the_bottom_centre_of_its_output() {
    let mail = proven("org.quire.Mail");
    let destination = |model: &casement_core::Model| {
        let window = model.windows()[&WindowId(1)].clone();
        model.minimise_destination(&window).map(|t| t.rect)
    };
    let bare = run(vec![plug("HEADLESS-1"), map(1, &mail, TokenCheck::Missing)]);
    assert_eq!(
        destination(&bare.model),
        Some(rect(960, 1080, 0, 0)),
        "no tile: a point at the bottom centre"
    );
    let docked = run_from(
        bare.model,
        vec![
            layer(1, ShellRoleTag::Unassigned),
            Op::Event(Event::LayerPlaced {
                layer: LayerKey(1),
                at: LogicalPoint { x: 100, y: 900 },
            }),
            shell(ShellRequest::MinimiseTarget {
                dock: LayerKey(1),
                window: WindowId(1),
                local: rect(10, 5, 64, 64),
            }),
        ],
    );
    assert_eq!(
        destination(&docked.model),
        Some(rect(110, 905, 64, 64)),
        "the dock's tile"
    );
    let cleared = run_from(
        docked.model,
        vec![shell(ShellRequest::ClearMinimiseTarget {
            window: WindowId(1),
        })],
    );
    assert_eq!(
        destination(&cleared.model),
        Some(rect(960, 1080, 0, 0)),
        "a cleared tile falls back to the bottom centre"
    );
}

/// Each shell role's blur strength; every role has a row, and a layer surface with no role (a
/// client's own) keeps today's one look.
#[test]
fn a_shell_role_picks_the_strength_of_the_blur_behind_it() {
    use casement_core::layer::BlurStrength::{Light, Regular, Strong};
    use casement_protocols::vocab::WireCode;
    let table = [
        (ShellRole::Bar, Regular),
        (ShellRole::Dock, Regular),
        (ShellRole::Launcher, Strong),
        (ShellRole::Switcher, Regular),
        (ShellRole::Overview, Regular),
        (ShellRole::Notifications, Regular),
        (ShellRole::Osd, Light),
        (ShellRole::ControlCentre, Strong),
        (ShellRole::Wallpaper, Regular),
        (ShellRole::Widget, Regular),
    ];
    assert_eq!(table.len(), ShellRole::ALL.len(), "a row for every role");
    for (role, strength) in table {
        assert_eq!(
            BlurStrength::of(ShellRoleTag::Assigned(role)),
            strength,
            "{role:?}"
        );
    }
    assert_eq!(BlurStrength::of(ShellRoleTag::Unassigned), Regular);
    assert!(Light < Regular && Regular < Strong, "lightest first");
}
