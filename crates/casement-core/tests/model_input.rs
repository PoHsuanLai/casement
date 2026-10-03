//! Presses, desktop commands, timers, settings and states through the model's entry points.
//! Every row runs with the invariants checked after each step.

mod support;

use casement_core::app::AppLife;
use casement_core::focus::{HitPart, PressRoute, TokenCheck};
use casement_core::input::{DesktopCommand, DeviceClass, Origin};
use casement_core::settings::{CasementSettings, Enabled};
use casement_core::window::WindowState;
use casement_core::{CoreNote, Effect, Event, ToplevelRequest, WindowId};
use casement_protocols::vocab::MacMods;
use support::driver::{Op, Run, assert_same_world, last, map, plug, press, request, run, run_from};
use support::proven;

fn two_apps() -> Run {
    let (mail, web) = (proven("org.quire.Mail"), proven("org.example.Web"));
    run(vec![
        plug("HEADLESS-1"),
        map(1, &web, TokenCheck::Valid),
        map(2, &mail, TokenCheck::Valid),
        map(3, &mail, TokenCheck::Valid),
    ])
}

#[test]
fn presses_are_routed_before_any_client_sees_them() {
    let physical = Origin::Physical(DeviceClass::Pointer);
    let on = |n: u64, part: HitPart, mods: MacMods| {
        Op::Press(casement_core::focus::Press {
            window: WindowId(n),
            part,
            mods,
            origin: physical,
        })
    };
    let cases = [
        (
            "the key window delivers",
            on(3, HitPart::Content, MacMods::NONE),
            PressRoute::Deliver,
        ),
        (
            "an inactive window's content activates and swallows",
            on(1, HitPart::Content, MacMods::NONE),
            PressRoute::ActivateRaiseSwallow,
        ),
        (
            "its titlebar activates and delivers",
            on(1, HitPart::Titlebar, MacMods::NONE),
            PressRoute::ActivateRaiseDeliver,
        ),
        (
            "command-click delivers without activating",
            on(1, HitPart::Content, MacMods::COMMAND),
            PressRoute::DeliverWithoutActivation,
        ),
        (
            "command-drag on a titlebar moves without activating",
            on(1, HitPart::Titlebar, MacMods::COMMAND),
            PressRoute::MoveWithoutActivation,
        ),
    ];
    for (name, press, route) in cases {
        let run = run_from(two_apps().model, vec![press]);
        assert!(last(&run).contains(&Effect::Route(route)), "{name}");
        let activated = matches!(
            route,
            PressRoute::ActivateRaiseSwallow | PressRoute::ActivateRaiseDeliver
        );
        assert_eq!(
            run.model.focus().key == Some(WindowId(1)),
            activated,
            "{name}"
        );
    }
}

#[test]
fn a_press_activating_a_window_raises_it_and_moves_the_keyboard() {
    let run = run_from(two_apps().model, vec![press(1)]);
    let effects = last(&run);
    assert!(effects.contains(&Effect::Raise(vec![WindowId(1)])));
    assert!(
        effects.contains(&Effect::Keyboard(casement_core::KeyboardTarget::Window(
            WindowId(1)
        )))
    );
    assert_eq!(
        run.model.windows()[&WindowId(1)].emphasis,
        casement_protocols::vocab::Emphasis::Key
    );
    assert_eq!(
        run.model.windows()[&WindowId(3)].emphasis,
        casement_protocols::vocab::Emphasis::Inactive
    );
}

#[test]
fn a_press_behind_a_sheet_rings_the_bell() {
    let mail = proven("org.quire.Mail");
    let run = run(vec![
        plug("HEADLESS-1"),
        map(1, &mail, TokenCheck::Valid),
        support::driver::map_sheet(2, &mail, 1),
        press(1),
    ]);
    let effects = last(&run);
    assert!(effects.contains(&Effect::Route(PressRoute::SwallowWithBell)));
    assert!(effects.contains(&Effect::Bell { app: mail }));
}

#[test]
fn a_press_on_a_window_nobody_can_see_is_ignored() {
    let mail = proven("org.quire.Mail");
    let hidden = run(vec![
        plug("HEADLESS-1"),
        map(1, &mail, TokenCheck::Valid),
        map(2, &proven("org.example.Web"), TokenCheck::Valid),
        Op::Command(DesktopCommand::HideActive),
    ]);
    let after = run_from(hidden.model.clone(), vec![press(2), press(9)]);
    assert_same_world(&after.model, &hidden.model, "unchanged");
    assert_eq!(last(&after), []);
}

#[test]
fn the_desktop_commands_that_close_minimise_and_place_the_key_window() {
    let run_cmd = |command| run_from(two_apps().model, vec![Op::Command(command)]);
    let close = run_cmd(DesktopCommand::CloseKey);
    assert_eq!(
        last(&close),
        [Effect::SendClose {
            window: WindowId(3)
        }]
    );

    let minimise = run_cmd(DesktopCommand::MinimiseKey);
    assert!(matches!(
        minimise.model.windows()[&WindowId(3)].state,
        WindowState::Minimised { .. }
    ));
    assert_eq!(minimise.model.focus().key, Some(WindowId(2)));

    let place = run_cmd(DesktopCommand::Place(
        casement_protocols::vocab::PlaceTarget::Right,
    ));
    assert!(matches!(
        place.model.windows()[&WindowId(3)].state,
        WindowState::Tiled {
            zone: casement_protocols::vocab::TileZone::Right,
            ..
        }
    ));
}

#[test]
fn quitting_asks_every_window_of_the_active_app_to_close() {
    let run = run_from(
        two_apps().model,
        vec![Op::Command(DesktopCommand::QuitActive)],
    );
    let closes: Vec<_> = last(&run)
        .iter()
        .filter_map(|e| match e {
            Effect::SendClose { window } => Some(*window),
            _ => None,
        })
        .collect();
    assert_eq!(closes, vec![WindowId(2), WindowId(3)]);
}

#[test]
fn commands_with_nothing_to_act_on_do_nothing() {
    let empty = run(vec![plug("HEADLESS-1")]);
    for command in [
        DesktopCommand::CloseKey,
        DesktopCommand::MinimiseKey,
        DesktopCommand::HideActive,
        DesktopCommand::HideOthers,
        DesktopCommand::QuitActive,
        DesktopCommand::ForceQuitPanel,
        DesktopCommand::Place(casement_protocols::vocab::PlaceTarget::Fill),
    ] {
        let after = run_from(empty.model.clone(), vec![Op::Command(command)]);
        assert_same_world(&after.model, &empty.model, &format!("{command:?}"));
        assert_eq!(last(&after), [], "{command:?}");
    }
}

#[test]
fn an_unanswered_ping_marks_the_app_not_responding_and_a_pong_clears_it() {
    let mail = proven("org.quire.Mail");
    let base = run(vec![plug("HEADLESS-1"), map(1, &mail, TokenCheck::Valid)]);
    let ping = casement_core::time::TimerKey::Ping(casement_core::ClientKey(1));
    let stuck = run_from(base.model, vec![Op::Event(Event::TimerFired(ping.clone()))]);
    assert!(matches!(
        stuck.model.apps()[&mail].life,
        AppLife::NotResponding { .. }
    ));
    let answered = run_from(
        stuck.model,
        vec![Op::Event(Event::Pong {
            client: casement_core::ClientKey(1),
        })],
    );
    assert_eq!(answered.model.apps()[&mail].life, AppLife::Running);
    assert!(last(&answered).contains(&Effect::CancelTimer(ping)));
}

#[test]
fn a_timer_for_something_gone_is_noted() {
    let base = run(vec![plug("HEADLESS-1")]);
    let ping = casement_core::time::TimerKey::Ping(casement_core::ClientKey(4));
    let after = run_from(
        base.model.clone(),
        vec![Op::Event(Event::TimerFired(ping.clone()))],
    );
    assert_same_world(&after.model, &base.model, "unchanged");
    assert_eq!(last(&after), [Effect::Note(CoreNote::StaleTimer(ping))]);
    let repeat = run_from(
        base.model.clone(),
        vec![Op::Event(Event::TimerFired(
            casement_core::time::TimerKey::SwitcherDelay,
        ))],
    );
    assert_eq!(
        last(&repeat),
        [],
        "a timer the model does not track is the edge's"
    );
}

#[test]
fn settings_replace_and_lay_out_again_only_when_they_differ() {
    let base = run(vec![plug("HEADLESS-1")]);
    let same = run_from(
        base.model.clone(),
        vec![Op::Event(Event::SettingsChanged(Box::new(
            CasementSettings::defaults(),
        )))],
    );
    assert_same_world(&same.model, &base.model, "unchanged");
    assert_eq!(last(&same), []);

    let mut settings = CasementSettings::defaults();
    settings.spaces.switch_on_activate = Enabled::Off;
    let changed = run_from(
        base.model,
        vec![Op::Event(Event::SettingsChanged(Box::new(
            settings.clone(),
        )))],
    );
    assert_eq!(changed.model.settings(), &settings);
    assert!(last(&changed).contains(&Effect::Relayout(casement_core::Cause::Configure)));
}

#[test]
fn states_follow_the_record_and_focus() {
    use casement_core::{Activation, Held};
    let run = run_from(
        two_apps().model,
        vec![request(
            3,
            ToplevelRequest::Place {
                target: casement_protocols::vocab::PlaceTarget::TopLeft,
                serial: casement_core::InputSerial(1),
            },
        )],
    );
    let key = run.model.states_of(WindowId(3)).expect("window");
    assert_eq!(key.activation, Activation::Active);
    assert_eq!((key.tiled.top, key.tiled.left), (Held::Yes, Held::Yes));
    assert_eq!((key.tiled.bottom, key.tiled.right), (Held::No, Held::No));
    assert_eq!(key.maximised, Held::No);
    let other = run.model.states_of(WindowId(1)).expect("window");
    assert_eq!(other.activation, Activation::Inactive);
    assert_eq!(run.model.states_of(WindowId(9)), None);
}
