//! The session lock through `Model::step`, and what it gates. Every row runs with the
//! invariants checked after each step.

use crate::support;

use casement_core::focus::TokenCheck;
use casement_core::input::{InputTarget, InputUse, Origin, Reach, Refusal};
use casement_core::layer::{
    Anchor, Anchors, ExclusiveZone, KeyboardInteractivity, LayerLevel, LayerSpec, Margins,
    ShellRoleTag,
};
use casement_core::lock::{LockEvent, LockReply, LockState, LockSurfaces};
use casement_core::time::TimerKey;
use casement_core::{ClientKey, Effect, Event, KeyboardTarget, LayerKey, OutputKey, WindowId};
use support::driver::{Op, assert_same_world, last, map, plug, run, run_from, unplug};
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

fn lock(event: LockEvent) -> Op {
    Op::Event(Event::Lock(event))
}

fn requested() -> Op {
    lock(LockEvent::Requested {
        client: ClientKey(7),
    })
}

fn committed(output: &str) -> Op {
    lock(LockEvent::SurfaceCommitted {
        output: key(output),
    })
}

fn cover(output: &str) -> Effect {
    Effect::Lock(LockReply::ShowCover(key(output)))
}

fn uncover(output: &str) -> Effect {
    Effect::Lock(LockReply::HideCover(key(output)))
}

#[test]
fn a_locker_that_never_makes_a_surface_is_not_waited_for_forever() {
    // ext-session-lock: the compositor may wait for the first lock surface before `locked`, but
    // "must impose a reasonable time limit"; the covers are up, so nothing can be seen.
    let base = run(vec![plug("HEADLESS-1"), requested()]);
    let due = last(&base)
        .iter()
        .find_map(|effect| match effect {
            Effect::ArmTimer(due) if due.key == TimerKey::LockWait => Some(due.clone()),
            _ => None,
        })
        .expect("the wait for a lock surface is bounded by a timer");
    assert!(
        !last(&base).contains(&Effect::Lock(LockReply::SendLocked)),
        "not yet"
    );
    let late = run_from(base.model, vec![Op::Event(Event::TimerFired(due.key))]);
    assert!(last(&late).contains(&Effect::Lock(LockReply::SendLocked)));
    assert_eq!(
        late.model.lock(),
        &LockState::Locked {
            surfaces: LockSurfaces::Pending
        }
    );
    // A surface that comes after is still taken in.
    let after = run_from(late.model, vec![committed("HEADLESS-1")]);
    assert!(last(&after).contains(&uncover("HEADLESS-1")));
    assert!(!last(&after).contains(&Effect::Lock(LockReply::SendLocked)));
}

#[test]
fn the_wait_ends_with_the_first_surface_and_a_stale_timer_changes_nothing() {
    let base = run(vec![
        plug("HEADLESS-1"),
        requested(),
        committed("HEADLESS-1"),
    ]);
    assert!(last(&base).contains(&Effect::CancelTimer(TimerKey::LockWait)));
    let stale = run_from(
        base.model.clone(),
        vec![Op::Event(Event::TimerFired(TimerKey::LockWait))],
    );
    assert_same_world(&stale.model, &base.model, "a timer for a wait that is over");
    assert!(!last(&stale).contains(&Effect::Lock(LockReply::SendLocked)));
}

#[test]
fn the_lock_runs_from_request_to_unlock() {
    let mail = proven("org.quire.Mail");
    let base = run(vec![
        plug("HEADLESS-1"),
        plug("HEADLESS-2"),
        map(1, &mail, TokenCheck::Valid),
    ]);
    let (a, b) = ("HEADLESS-1", "HEADLESS-2");
    let asked = run_from(base.model, vec![requested()]);
    assert!(matches!(asked.model.lock(), LockState::Locking { .. }));
    let effects = last(&asked);
    assert!(effects.contains(&cover(a)) && effects.contains(&cover(b)));
    assert!(
        effects.contains(&Effect::Keyboard(KeyboardTarget::Nothing)),
        "the window loses it"
    );

    let first = run_from(asked.model, vec![committed(a)]);
    assert_eq!(
        first.model.lock(),
        &LockState::Locked {
            surfaces: LockSurfaces::Covering(vec![key(a)])
        }
    );
    let effects = last(&first);
    assert!(effects.contains(&Effect::Lock(LockReply::SendLocked)));
    assert!(effects.contains(&uncover(a)));
    assert!(effects.contains(&Effect::Keyboard(KeyboardTarget::LockSurface(key(a)))));

    let second = run_from(first.model, vec![committed(b), committed(b)]);
    assert_eq!(
        second.model.lock(),
        &LockState::Locked {
            surfaces: LockSurfaces::Covering(vec![key(a), key(b)])
        }
    );
    assert_eq!(last(&second), [], "a repeat commit says nothing");

    let open = run_from(second.model, vec![lock(LockEvent::Unlocked)]);
    assert_eq!(open.model.lock(), &LockState::Unlocked);
    assert!(last(&open).contains(&Effect::Keyboard(KeyboardTarget::Window(WindowId(1)))));
    assert!(
        !last(&open)
            .iter()
            .any(|e| matches!(e, Effect::Lock(LockReply::ShowCover(_))))
    );
}

#[test]
fn unlocking_takes_down_only_the_covers_that_were_up() {
    let run = run(vec![
        plug("HEADLESS-1"),
        plug("HEADLESS-2"),
        requested(),
        committed("HEADLESS-1"),
        lock(LockEvent::Unlocked),
    ]);
    assert_eq!(
        last(&run)
            .iter()
            .filter(|e| matches!(e, Effect::Lock(LockReply::HideCover(_))))
            .count(),
        1
    );
    assert!(last(&run).contains(&uncover("HEADLESS-2")));
}

#[test]
fn a_dead_locker_leaves_the_cover_and_another_locker_takes_over() {
    let base = run(vec![
        plug("HEADLESS-1"),
        requested(),
        committed("HEADLESS-1"),
        lock(LockEvent::LockerGone),
    ]);
    assert_eq!(base.model.lock(), &LockState::LockedWithoutSurface);
    assert_eq!(
        last(&base),
        [
            cover("HEADLESS-1"),
            Effect::Keyboard(KeyboardTarget::Nothing)
        ]
    );

    let again = run_from(base.model.clone(), vec![lock(LockEvent::LockerGone)]);
    assert_eq!(last(&again), [], "a second death changes nothing");

    let taken = run_from(base.model, vec![requested()]);
    assert_eq!(
        taken.model.lock(),
        &LockState::Locked {
            surfaces: LockSurfaces::Pending
        }
    );
    assert_eq!(last(&taken), [Effect::Lock(LockReply::SendLocked)]);
}

#[test]
fn a_second_request_while_locked_is_finished_and_changes_nothing() {
    let locked = run(vec![
        plug("HEADLESS-1"),
        requested(),
        committed("HEADLESS-1"),
    ]);
    let again = run_from(locked.model.clone(), vec![requested()]);
    assert_same_world(&again.model, &locked.model, "second lock request");
    assert_eq!(last(&again), [Effect::Lock(LockReply::Finished)]);
}

#[test]
fn a_request_with_no_output_locks_at_once() {
    let run = run(vec![requested()]);
    assert_eq!(
        run.model.lock(),
        &LockState::Locked {
            surfaces: LockSurfaces::Pending
        }
    );
    assert_eq!(last(&run), [Effect::Lock(LockReply::SendLocked)]);
}

#[test]
fn lock_events_while_unlocked_or_for_unknown_outputs_do_nothing() {
    let base = run(vec![plug("HEADLESS-1")]);
    for event in [
        committed("HEADLESS-1"),
        lock(LockEvent::Unlocked),
        lock(LockEvent::LockerGone),
    ] {
        let run = run_from(base.model.clone(), vec![event.clone()]);
        assert_same_world(&run.model, &base.model, &format!("{event:?}"));
        assert_eq!(last(&run), [], "{event:?}");
    }
    let locking = run_from(base.model, vec![requested()]);
    let unknown = run_from(locking.model.clone(), vec![committed("HEADLESS-9")]);
    assert_same_world(&unknown.model, &locking.model, "unknown output");
}

#[test]
fn an_output_plugged_or_unplugged_while_locked_is_covered_or_forgotten() {
    let locked = run(vec![
        plug("HEADLESS-1"),
        requested(),
        committed("HEADLESS-1"),
        plug("HEADLESS-2"),
    ]);
    assert!(last(&locked).contains(&cover("HEADLESS-2")));
    let gone = run_from(locked.model, vec![unplug("HEADLESS-1")]);
    assert_eq!(
        gone.model.lock(),
        &LockState::Locked {
            surfaces: LockSurfaces::Pending
        }
    );
}

#[test]
fn while_locked_no_window_or_layer_hears_the_keyboard() {
    let mail = proven("org.quire.Mail");
    let launcher = LayerSpec {
        level: LayerLevel::Overlay,
        keyboard: KeyboardInteractivity::Exclusive,
        ..bar("HEADLESS-1", ExclusiveZone::Ignore)
    };
    let run = run(vec![
        plug("HEADLESS-1"),
        requested(),
        map(1, &mail, TokenCheck::Valid),
        mapped(1, launcher),
    ]);
    assert_eq!(
        run.model.focus().key,
        Some(WindowId(1)),
        "focus still follows"
    );
    assert!(!run.effects.iter().flatten().any(|e| matches!(
        e,
        Effect::Keyboard(KeyboardTarget::Window(_) | KeyboardTarget::Layer(_))
    )));
    let open = run_from(run.model, vec![lock(LockEvent::Unlocked)]);
    assert_eq!(
        open.model.keyboard_target(),
        KeyboardTarget::Layer(LayerKey(1))
    );
}

#[test]
fn presses_and_commands_do_nothing_while_locked() {
    let mail = proven("org.quire.Mail");
    let base = run(vec![
        plug("HEADLESS-1"),
        map(1, &mail, TokenCheck::Valid),
        map(2, &proven("org.example.Web"), TokenCheck::Valid),
        requested(),
    ]);
    let ops = [
        support::driver::press(1),
        Op::Activate(mail, casement_protocols::vocab::ActorKind::User),
        Op::Command(casement_core::input::DesktopCommand::HideActive),
    ];
    for op in ops {
        let run = run_from(base.model.clone(), vec![op.clone()]);
        assert_same_world(&run.model, &base.model, &format!("{op:?}"));
        assert_eq!(last(&run), [], "{op:?}");
    }
}

#[test]
fn reach_asks_the_input_table_with_the_models_lock() {
    use casement_core::input::DeviceClass;
    let physical = Origin::Physical(DeviceClass::Keyboard);
    let virtual_keyboard = Origin::Virtual(DeviceClass::Keyboard);
    let open = run(vec![plug("HEADLESS-1")]);
    let locked = run_from(open.model.clone(), vec![requested()]);
    let window = InputTarget::Window(WindowId(1));
    let cases = [
        (
            "open, a window",
            &open.model,
            virtual_keyboard,
            window,
            Reach::Allowed,
        ),
        (
            "locked, a window",
            &locked.model,
            physical,
            window,
            Reach::Refused(Refusal::Locked),
        ),
        (
            "locked, the lock surface",
            &locked.model,
            physical,
            InputTarget::LockSurface,
            Reach::Allowed,
        ),
        (
            "locked, a virtual keyboard at the lock surface",
            &locked.model,
            virtual_keyboard,
            InputTarget::LockSurface,
            Reach::Refused(Refusal::Locked),
        ),
    ];
    for (name, model, origin, target, want) in cases {
        assert_eq!(model.reach(&origin, &target, InputUse::Key), want, "{name}");
    }
}

#[test]
fn input_events_change_nothing() {
    let base = run(vec![
        plug("HEADLESS-1"),
        map(1, &proven("org.quire.Mail"), TokenCheck::Valid),
    ]);
    let key_press = Event::Input(casement_core::input::InputEvent {
        origin: Origin::Test,
        device: casement_core::DeviceId(1),
        at: support::ms(5),
        kind: casement_core::input::InputKind::Key {
            code: casement_core::input::KeyCode(30),
            edge: casement_core::input::Edge::Press,
        },
    });
    let run = run_from(base.model.clone(), vec![Op::Event(key_press)]);
    assert_same_world(&run.model, &base.model, "input");
    assert_eq!(last(&run), []);
}
