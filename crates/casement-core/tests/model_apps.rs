//! Apps through the model's entry points: hiding, unhiding and activating (rule S5). Every row
//! runs with the invariants checked after each step.

mod support;

use casement_core::app::AppVisibility;
use casement_core::focus::TokenCheck;
use casement_core::input::DesktopCommand;
use casement_core::settings::{CasementSettings, Enabled};
use casement_core::{CoreNote, Effect, Event, ToplevelRequest, WindowId};
use casement_protocols::vocab::{ActorKind, Attention, Toggle};
use support::driver::{Op, Run, last, map, plug, request, run, run_from};
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
fn hiding_the_active_app_activates_the_previous_one_and_unhiding_by_activation_restores() {
    let (mail, web) = (proven("org.quire.Mail"), proven("org.example.Web"));
    let hidden = run_from(
        two_apps().model,
        vec![Op::Command(DesktopCommand::HideActive)],
    );
    assert!(matches!(
        hidden.model.apps()[&mail].visibility,
        AppVisibility::Hidden { ref restore_order } if *restore_order == vec![WindowId(2), WindowId(3)]
    ));
    assert_eq!(hidden.model.focus().active, Some(web.clone()));
    assert_eq!(hidden.model.focus().key, Some(WindowId(1)));
    assert!(last(&hidden).contains(&Effect::Relayout(casement_core::Cause::Configure)));

    let again = run_from(
        hidden.model,
        vec![Op::Activate(mail.clone(), ActorKind::User)],
    );
    assert_eq!(again.model.apps()[&mail].visibility, AppVisibility::Shown);
    assert_eq!(again.model.focus().active, Some(mail));
    assert_eq!(again.model.focus().key, Some(WindowId(3)));
}

#[test]
fn hiding_when_nothing_else_is_visible_leaves_nothing_active() {
    let mail = proven("org.quire.Mail");
    let run = run(vec![
        plug("HEADLESS-1"),
        map(1, &mail, TokenCheck::Valid),
        Op::Command(DesktopCommand::HideActive),
    ]);
    assert_eq!(run.model.focus().active, None);
    assert_eq!(run.model.focus().key, None);
}

#[test]
fn hide_others_hides_every_app_but_the_active_one() {
    let (mail, web) = (proven("org.quire.Mail"), proven("org.example.Web"));
    let run = run_from(
        two_apps().model,
        vec![Op::Command(DesktopCommand::HideOthers)],
    );
    assert!(matches!(
        run.model.apps()[&web].visibility,
        AppVisibility::Hidden { .. }
    ));
    assert_eq!(run.model.apps()[&mail].visibility, AppVisibility::Shown);
    assert_eq!(run.model.focus().active, Some(mail));
}

#[test]
fn an_app_a_third_party_or_an_unknown_actor_cannot_move_focus() {
    let web = proven("org.example.Web");
    for by in [ActorKind::App, ActorKind::ThirdParty, ActorKind::Unknown] {
        let before = two_apps();
        let run = run_from(before.model.clone(), vec![Op::Activate(web.clone(), by)]);
        assert_eq!(run.model.focus(), before.model.focus(), "{by:?}");
        assert!(
            last(&run).contains(&Effect::Attention {
                app: web.clone(),
                level: Attention::Informational
            }),
            "{by:?}"
        );
    }
}

#[test]
fn activating_an_app_brings_its_windows_forward_and_keys_its_latest() {
    let web = proven("org.example.Web");
    let run = run_from(
        two_apps().model,
        vec![Op::Activate(web.clone(), ActorKind::User)],
    );
    assert_eq!(run.model.focus().active, Some(web));
    assert_eq!(run.model.focus().key, Some(WindowId(1)));
    assert!(last(&run).contains(&Effect::Raise(vec![WindowId(1)])));
    let top = run
        .model
        .windows()
        .values()
        .max_by_key(|w| w.stack)
        .map(|w| w.id);
    assert_eq!(
        top,
        Some(WindowId(1)),
        "the raise reached the stacking order"
    );
}

/// Mail has a window on the desktop, a full-screen window of its own on another Space, and
/// the person is looking at the desktop in Web.
fn app_on_another_space() -> Run {
    let (mail, web) = (proven("org.quire.Mail"), proven("org.example.Web"));
    run(vec![
        plug("HEADLESS-1"),
        map(1, &mail, TokenCheck::Valid),
        request(
            1,
            ToplevelRequest::FullScreen {
                toggle: Toggle::On,
                output: None,
            },
        ),
        Op::Command(DesktopCommand::SwitchSpace(
            casement_core::input::SpaceStep::Previous,
        )),
        map(2, &web, TokenCheck::Valid),
    ])
}

#[test]
fn activating_an_app_whose_windows_are_on_another_space_switches_to_it() {
    let mail = proven("org.quire.Mail");
    let before = app_on_another_space();
    assert_eq!(before.model.focus().active, Some(proven("org.example.Web")));
    let key = support::headless("HEADLESS-1").key;
    let desktop = before
        .model
        .spaces()
        .shown_on(&key)
        .expect("shown")
        .id
        .clone();
    let run = run_from(
        before.model,
        vec![Op::Activate(mail.clone(), ActorKind::User)],
    );
    let shown = run.model.spaces().shown_on(&key).expect("shown").id.clone();
    assert_ne!(shown, desktop, "S5: the Space switched");
    assert_eq!(run.model.focus().active, Some(mail));
    assert_eq!(run.model.focus().key, Some(WindowId(1)));
    assert!(last(&run).contains(&Effect::Relayout(casement_core::Cause::SpaceSwitch)));
}

#[test]
fn with_switching_off_the_app_is_active_and_keyless_and_the_space_stays() {
    let mail = proven("org.quire.Mail");
    let before = app_on_another_space();
    let key = support::headless("HEADLESS-1").key;
    let desktop = before
        .model
        .spaces()
        .shown_on(&key)
        .expect("shown")
        .id
        .clone();
    let mut settings = CasementSettings::defaults();
    settings.spaces.switch_on_activate = Enabled::Off;
    let run = run_from(
        before.model,
        vec![
            Op::Event(Event::SettingsChanged(Box::new(settings))),
            Op::Activate(mail.clone(), ActorKind::User),
        ],
    );
    assert_eq!(
        run.model.spaces().shown_on(&key).map(|s| &s.id),
        Some(&desktop)
    );
    assert_eq!(run.model.focus().active, Some(mail));
    assert_eq!(run.model.focus().key, None);
}

#[test]
fn a_window_activating_itself_on_another_space_follows_the_setting_too() {
    let before = app_on_another_space();
    let key = support::headless("HEADLESS-1").key;
    let desktop = before
        .model
        .spaces()
        .shown_on(&key)
        .expect("shown")
        .id
        .clone();
    let on = run_from(
        before.model.clone(),
        vec![request(1, ToplevelRequest::Activate(TokenCheck::Valid))],
    );
    assert_ne!(
        on.model.spaces().shown_on(&key).map(|s| &s.id),
        Some(&desktop)
    );
    assert_eq!(on.model.focus().key, Some(WindowId(1)));

    let mut settings = CasementSettings::defaults();
    settings.spaces.switch_on_activate = Enabled::Off;
    let off = run_from(
        before.model,
        vec![
            Op::Event(Event::SettingsChanged(Box::new(settings))),
            request(1, ToplevelRequest::Activate(TokenCheck::Valid)),
        ],
    );
    assert_eq!(
        off.model.spaces().shown_on(&key).map(|s| &s.id),
        Some(&desktop)
    );
    assert_eq!(
        off.model.focus().key,
        Some(WindowId(2)),
        "the key stays where it can be seen"
    );
    assert!(last(&off).contains(&Effect::Note(CoreNote::RefusedActivation(WindowId(1)))));
}
