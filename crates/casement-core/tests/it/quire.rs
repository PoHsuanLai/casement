//! The P2 surface of the model: which connection sees the `quire` globals (I7), what the desktop
//! manager's requests reach (`ToSpace`, `Assign`, `MoveTo` included) and the notes the model
//! publishes for `quire_desktop_v1`. The shell's requests are in `model_shell.rs`.

use crate::support::driver::{Op, last, map, plug, run, run_from};
use crate::support::{headless, proven};
use casement_core::app::AppVisibility;
use casement_core::desktop::{AppAsk, DesktopNote, DesktopRequest, SpaceAsk, WindowAsk};
use casement_core::focus::TokenCheck;
use casement_core::policy::{Advertise, ConnectionRole, GlobalKind, advertised};
use casement_core::space::SpaceIndex;
use casement_core::window::{SpacePlace, WindowState};
use casement_core::{CoreNote, Deferred, Effect, Event, WindowId};
use casement_protocols::vocab::{Assign, PlaceTarget};
use porter_core::SandboxFacts;

const GLOBALS: [GlobalKind; 3] = [
    GlobalKind::QuireDesktop,
    GlobalKind::QuireShell,
    GlobalKind::ToplevelCapture,
];

fn sandbox() -> ConnectionRole {
    ConnectionRole::Sandboxed(SandboxFacts::Flatpak {
        app: "org.example.App".to_owned(),
        instance: "1".to_owned(),
    })
}

/// (global, public, shell, agent, sandboxed): the I7 rows P2-Z adds.
#[test]
fn the_p2_globals_are_seen_by_the_right_connections() {
    use Advertise::{No, Yes};
    let rows = [
        (GlobalKind::QuireDesktop, No, Yes, Yes, No),
        (GlobalKind::QuireShell, No, Yes, No, No),
        (GlobalKind::ToplevelCapture, Yes, Yes, Yes, No),
    ];
    assert_eq!(rows.map(|row| row.0), GLOBALS);
    for (global, public, shell, agent, sandboxed) in rows {
        assert_eq!(
            advertised(global, &ConnectionRole::Public),
            public,
            "{global:?} public"
        );
        assert_eq!(
            advertised(global, &ConnectionRole::Shell),
            shell,
            "{global:?} shell"
        );
        assert_eq!(
            advertised(global, &ConnectionRole::Agent),
            agent,
            "{global:?} agent"
        );
        assert_eq!(
            advertised(global, &sandbox()),
            sandboxed,
            "{global:?} sandboxed"
        );
    }
}

fn desktop(request: DesktopRequest) -> Op {
    Op::Event(Event::Desktop(request))
}

fn window_ask(window: u64, ask: WindowAsk) -> Op {
    desktop(DesktopRequest::Window {
        window: WindowId(window),
        ask,
    })
}

#[test]
fn the_desktop_manager_reaches_the_windows_and_apps_it_names() {
    let mail = proven("org.quire.Mail");
    let base = run(vec![plug("HEADLESS-1"), map(1, &mail, TokenCheck::Missing)]);
    let closed = run_from(base.model.clone(), vec![window_ask(1, WindowAsk::Close)]);
    assert!(last(&closed).contains(&Effect::SendClose {
        window: WindowId(1)
    }));
    let filled = run_from(
        base.model.clone(),
        vec![window_ask(1, WindowAsk::Place(PlaceTarget::Fill))],
    );
    assert!(matches!(
        filled.model.windows()[&WindowId(1)].state,
        WindowState::Filled { .. }
    ));
    let hidden = run_from(
        base.model.clone(),
        vec![desktop(DesktopRequest::App {
            app: mail.clone(),
            ask: AppAsk::Hide,
        })],
    );
    assert!(matches!(
        hidden.model.apps()[&mail].visibility,
        AppVisibility::Hidden { .. }
    ));
    let missing = run_from(base.model, vec![window_ask(9, WindowAsk::Close)]);
    assert!(last(&missing).contains(&Effect::Note(CoreNote::UnknownWindow(WindowId(9)))));
}

#[test]
fn reopen_waits_for_its_machine() {
    let mail = proven("org.quire.Mail");
    let base = run(vec![plug("HEADLESS-1"), map(1, &mail, TokenCheck::Missing)]);
    let asked = run_from(
        base.model,
        vec![desktop(DesktopRequest::App {
            app: mail.clone(),
            ask: AppAsk::Reopen,
        })],
    );
    assert!(last(&asked).contains(&Effect::Note(CoreNote::Deferred(Deferred::Reopen))));
}

#[test]
fn a_new_space_is_made_on_the_named_output() {
    let key = headless("HEADLESS-1").key;
    let made = run(vec![
        plug("HEADLESS-1"),
        desktop(DesktopRequest::CreateSpace {
            output: key.clone(),
        }),
    ]);
    assert_eq!(made.model.spaces().row(&key).len(), 2);
}

#[test]
fn a_window_moves_to_the_space_named() {
    let mail = proven("org.quire.Mail");
    let key = headless("HEADLESS-1").key;
    let made = run(vec![
        plug("HEADLESS-1"),
        map(1, &mail, TokenCheck::Missing),
        desktop(DesktopRequest::CreateSpace {
            output: key.clone(),
        }),
    ]);
    let other = made
        .model
        .spaces()
        .row(&key)
        .into_iter()
        .map(|space| space.id.clone())
        .find(|id| made.model.windows()[&WindowId(1)].place != SpacePlace::On(id.clone()))
        .expect("a second Space");
    let moved = run_from(
        made.model,
        vec![window_ask(1, WindowAsk::ToSpace(other.clone()))],
    );
    assert_eq!(
        moved.model.windows()[&WindowId(1)].place,
        SpacePlace::On(other)
    );
}

#[test]
fn a_window_is_assigned_to_every_space_or_this_one() {
    let mail = proven("org.quire.Mail");
    let all = run(vec![
        plug("HEADLESS-1"),
        map(1, &mail, TokenCheck::Missing),
        window_ask(1, WindowAsk::Assign(Assign::AllSpaces)),
    ]);
    assert_eq!(
        all.model.windows()[&WindowId(1)].place,
        SpacePlace::AllSpaces
    );
    let here = run_from(
        all.model,
        vec![window_ask(1, WindowAsk::Assign(Assign::ThisSpace))],
    );
    assert!(matches!(
        here.model.windows()[&WindowId(1)].place,
        SpacePlace::On(_)
    ));
}

#[test]
fn a_space_moves_within_its_row() {
    let key = headless("HEADLESS-1").key;
    let made = run(vec![
        plug("HEADLESS-1"),
        desktop(DesktopRequest::CreateSpace {
            output: key.clone(),
        }),
    ]);
    let second = made.model.spaces().row(&key)[1].id.clone();
    let moved = run_from(
        made.model,
        vec![desktop(DesktopRequest::Space {
            space: second.clone(),
            ask: SpaceAsk::MoveTo(SpaceIndex(0)),
        })],
    );
    assert_eq!(moved.model.spaces().row(&key)[0].id, second);
}

#[test]
fn a_mapped_window_and_its_app_are_published() {
    let mail = proven("org.quire.Mail");
    let mapped = run(vec![plug("HEADLESS-1"), map(1, &mail, TokenCheck::Missing)]);
    let effects = last(&mapped);
    assert!(effects.contains(&Effect::Desktop(DesktopNote::App(mail))));
    assert!(effects.contains(&Effect::Desktop(DesktopNote::Window(WindowId(1)))));
    // Mapped without an activation token, the window does not take the key: nothing active
    // changed, so nothing active is published.
    assert!(!effects.contains(&Effect::Desktop(DesktopNote::Active)));
    let unmapped = run_from(
        mapped.model,
        vec![Op::Event(Event::ToplevelUnmapped {
            window: WindowId(1),
        })],
    );
    assert!(
        last(&unmapped).contains(&Effect::Desktop(DesktopNote::Removed(
            casement_core::desktop::DesktopRef::Window(WindowId(1))
        )))
    );
}

#[test]
fn a_step_publishes_exactly_what_changed() {
    let mail = proven("org.quire.Mail");
    let mapped = run(vec![plug("HEADLESS-1"), map(1, &mail, TokenCheck::Missing)]);
    let notes: Vec<DesktopNote> = last(&mapped)
        .iter()
        .filter_map(|effect| match effect {
            Effect::Desktop(note) => Some(note.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(
        notes,
        vec![DesktopNote::App(mail), DesktopNote::Window(WindowId(1))],
        "a Space is not named when it did not change, the active pair is not named when the map \
         took no key (no activation token), and the notes come in the order a client needs"
    );
}

#[test]
fn a_change_of_placement_within_one_state_is_published() {
    let mail = proven("org.quire.Mail");
    let left = run(vec![
        plug("HEADLESS-1"),
        map(1, &mail, TokenCheck::Missing),
        window_ask(1, WindowAsk::Place(PlaceTarget::Left)),
    ]);
    let right = run_from(
        left.model,
        vec![window_ask(1, WindowAsk::Place(PlaceTarget::Right))],
    );
    assert!(last(&right).contains(&Effect::PublishToplevel(WindowId(1))));
}
