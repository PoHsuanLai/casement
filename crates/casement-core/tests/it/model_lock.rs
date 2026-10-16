//! The session lock through `Model::step`, and what it gates. Every row runs with the
//! invariants checked after each step.

use crate::support;

use casement_core::desktop::{DesktopRequest, WindowAsk};
use casement_core::focus::TokenCheck;
use casement_core::input::{InputTarget, InputUse, Origin, Reach, Refusal};
use casement_core::layer::{
    Anchor, Anchors, ExclusiveZone, KeyboardInteractivity, LayerLevel, LayerSpec, Margins,
    ShellRoleTag,
};
use casement_core::lock::{LockEvent, LockReply, LockState, LockSurfaces};
use casement_core::policy::ConnectionRole;
use casement_core::space::WorkspaceRequest;
use casement_core::time::TimerKey;
use casement_core::window::WlrToplevelRequest;
use casement_core::{
    ClientKey, Effect, Event, KeyboardTarget, LayerKey, OutputKey, ToplevelRequest, WindowId,
};
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

/// Client `n` connecting through a socket of `role`.
fn connected(n: u64, role: ConnectionRole) -> Op {
    Op::Event(Event::ClientConnected {
        client: ClientKey(n),
        peer: porter_core::PeerIdentity::Unproven,
        role,
    })
}

/// Client `n` asking to lock.
fn requested_by(n: u64) -> Op {
    lock(LockEvent::Requested {
        client: ClientKey(n),
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

/// CASC-13: while the session is locked (or being locked), nothing a window, a window-list
/// client, a workspace client or the shell's desktop manager asks reaches the desktop. The same
/// asks change it when the session is open, so the lock is what stops them.
#[test]
fn nothing_asked_of_the_desktop_gets_through_a_lock() {
    let mail = proven("org.quire.Mail");
    let one = key("HEADLESS-1");
    let open = run(vec![
        plug("HEADLESS-1"),
        map(1, &mail, TokenCheck::Valid),
        map(2, &mail, TokenCheck::Valid),
    ]);
    let asks = [
        (
            "a window's own request",
            Event::ToplevelRequest {
                window: WindowId(1),
                request: ToplevelRequest::Minimise,
            },
        ),
        (
            "a window-list client",
            Event::WlrToplevel {
                window: WindowId(1),
                request: WlrToplevelRequest::Close,
            },
        ),
        (
            "a workspace client",
            Event::Workspace(WorkspaceRequest::Create {
                output: one.clone(),
                name: String::new(),
            }),
        ),
        (
            "the desktop manager",
            Event::Desktop(DesktopRequest::Window {
                window: WindowId(1),
                ask: WindowAsk::Close,
            }),
        ),
    ];
    let lock_states = [
        ("being locked", vec![requested()]),
        ("locked", vec![requested(), committed("HEADLESS-1")]),
        (
            "locked, the locker gone",
            vec![
                requested(),
                committed("HEADLESS-1"),
                lock(LockEvent::LockerGone),
            ],
        ),
    ];
    for (ask, event) in asks {
        let ran = run_from(open.model.clone(), vec![Op::Event(event.clone())]);
        assert!(!last(&ran).is_empty(), "{ask} does something when open");
        for (state, steps) in &lock_states {
            let locked = run_from(open.model.clone(), steps.clone());
            let ran = run_from(locked.model.clone(), vec![Op::Event(event.clone())]);
            assert_same_world(&ran.model, &locked.model, &format!("{ask}, {state}"));
            assert_eq!(last(&ran), [], "{ask}, {state}");
        }
    }
}

/// CASC-17: the keyboard goes to the lock surface of the output the person works on (the key
/// window's, else the first lit one), not to whichever output sorts first.
#[test]
fn the_keyboard_goes_to_the_lock_surface_of_the_output_in_use() {
    use casement_core::output::{OutputChangeSet, OutputPower};
    let (a, b) = ("HEADLESS-1", "HEADLESS-2");
    let both = run(vec![
        plug(a),
        plug(b),
        requested(),
        committed(a),
        committed(b),
    ]);
    assert_eq!(
        both.model.keyboard_target(),
        KeyboardTarget::LockSurface(key(a))
    );
    let dark = run_from(
        both.model,
        vec![Op::Event(Event::OutputRequested {
            output: key(a),
            change: OutputChangeSet::none().with_power(OutputPower::Off),
        })],
    );
    assert_eq!(
        dark.model.keyboard_target(),
        KeyboardTarget::LockSurface(key(b)),
        "the lit output hears the keys"
    );
    assert!(last(&dark).contains(&Effect::Keyboard(KeyboardTarget::LockSurface(key(b)))));
}

/// CASW-11: window-list clients are told what is open again when the lock begins (they hear no
/// titles) and when it ends (they hear them back), for every window.
#[test]
fn locking_and_unlocking_list_every_window_again() {
    let mail = proven("org.quire.Mail");
    let open = run(vec![
        plug("HEADLESS-1"),
        map(1, &mail, TokenCheck::Valid),
        map(2, &mail, TokenCheck::Valid),
    ]);
    let relisted = |run: &support::driver::Run| {
        [WindowId(1), WindowId(2)].map(|id| {
            let effects = last(run);
            (
                effects.contains(&Effect::ListWindow(id)),
                effects.contains(&Effect::PublishToplevel(id)),
            )
        })
    };
    let locking = run_from(open.model, vec![requested()]);
    assert_eq!(relisted(&locking), [(true, true), (true, true)], "locking");
    let opened = run_from(locking.model, vec![lock(LockEvent::Unlocked)]);
    assert_eq!(relisted(&opened), [(true, true), (true, true)], "unlocking");
    let abandoned = run_from(
        run_from(opened.model, vec![requested()]).model,
        vec![lock(LockEvent::LockerGone)],
    );
    assert_eq!(
        relisted(&abandoned),
        [(true, true), (true, true)],
        "abandoned"
    );
}

fn surface_gone(output: &str) -> Op {
    lock(LockEvent::SurfaceGone {
        output: key(output),
    })
}

/// CASC-06, CASW-12: when a live locker's surface on one output goes, that output is covered
/// again at once; the last one going leaves the lock waiting for a surface; a surface that
/// comes back is taken in.
#[test]
fn a_lock_surface_that_goes_puts_the_cover_back() {
    let (a, b) = ("HEADLESS-1", "HEADLESS-2");
    let both = run(vec![
        plug(a),
        plug(b),
        requested(),
        committed(a),
        committed(b),
    ]);
    let one = run_from(both.model.clone(), vec![surface_gone(a)]);
    assert_eq!(
        one.model.lock(),
        &LockState::Locked {
            surfaces: LockSurfaces::Covering(vec![key(b)])
        }
    );
    assert!(last(&one).contains(&cover(a)));
    assert!(!last(&one).contains(&cover(b)));
    assert!(last(&one).contains(&Effect::Keyboard(KeyboardTarget::LockSurface(key(b)))));

    let none = run_from(one.model, vec![surface_gone(b)]);
    assert_eq!(
        none.model.lock(),
        &LockState::Locked {
            surfaces: LockSurfaces::Pending
        }
    );
    assert!(last(&none).contains(&cover(b)));

    let back = run_from(none.model, vec![committed(a)]);
    assert!(last(&back).contains(&uncover(a)));
    assert_eq!(
        back.model.lock(),
        &LockState::Locked {
            surfaces: LockSurfaces::Covering(vec![key(a)])
        }
    );

    // An output the locker never covered, a repeat, and a lock that is not held change nothing.
    for (name, model, op) in [
        ("never covered", &both.model, surface_gone("HEADLESS-9")),
        ("not held", &run(vec![plug(a)]).model, surface_gone(a)),
        (
            "still waiting",
            &run(vec![plug(a), requested()]).model,
            surface_gone(a),
        ),
    ] {
        let run = run_from(model.clone(), vec![op]);
        assert_same_world(&run.model, model, name);
        assert_eq!(last(&run), [], "{name}");
    }
}

/// CASC-05: a locker that dies before `locked` was sent never locked anything: the covers come
/// down, the wait ends and the session is open. One that dies after it leaves the session
/// locked.
#[test]
fn a_locker_that_dies_before_locked_leaves_the_session_open() {
    let locking = run(vec![plug("HEADLESS-1"), plug("HEADLESS-2"), requested()]);
    assert!(matches!(locking.model.lock(), LockState::Locking { .. }));
    let gone = run_from(locking.model, vec![lock(LockEvent::LockerGone)]);
    assert_eq!(gone.model.lock(), &LockState::Unlocked);
    let effects = last(&gone);
    assert!(effects.contains(&uncover("HEADLESS-1")));
    assert!(effects.contains(&uncover("HEADLESS-2")));
    assert!(effects.contains(&Effect::CancelTimer(TimerKey::LockWait)));
    // The timer that was armed is stale now and changes nothing.
    let stale = run_from(
        gone.model.clone(),
        vec![Op::Event(Event::TimerFired(TimerKey::LockWait))],
    );
    assert_same_world(&stale.model, &gone.model, "the wait that was cancelled");
    // With the first surface committed, `locked` was sent: that locker's death keeps the cover.
    let locked = run(vec![
        plug("HEADLESS-1"),
        requested(),
        committed("HEADLESS-1"),
        lock(LockEvent::LockerGone),
    ]);
    assert_eq!(locked.model.lock(), &LockState::LockedWithoutSurface);
}

/// CASW-02: while the locker is gone, only the shell or the client that held the lock may take
/// it over; any other client is refused and the session stays covered.
#[test]
fn only_the_shell_or_the_same_client_takes_a_lock_over() {
    let orphaned = run(vec![
        plug("HEADLESS-1"),
        connected(7, ConnectionRole::Public),
        connected(8, ConnectionRole::Public),
        connected(9, ConnectionRole::Shell),
        requested_by(7),
        committed("HEADLESS-1"),
        lock(LockEvent::LockerGone),
    ]);
    assert_eq!(orphaned.model.lock(), &LockState::LockedWithoutSurface);
    let cases = [
        ("another public client", 8, false),
        ("a client nobody introduced", 99, false),
        ("the client that held the lock", 7, true),
        ("the shell", 9, true),
    ];
    for (name, client, taken) in cases {
        let run = run_from(orphaned.model.clone(), vec![requested_by(client)]);
        match taken {
            true => {
                assert_eq!(
                    run.model.lock(),
                    &LockState::Locked {
                        surfaces: LockSurfaces::Pending
                    },
                    "{name}"
                );
                assert_eq!(last(&run), [Effect::Lock(LockReply::SendLocked)], "{name}");
            }
            false => {
                assert_same_world(&run.model, &orphaned.model, name);
                assert_eq!(last(&run), [Effect::Lock(LockReply::Finished)], "{name}");
            }
        }
    }
}

/// CASW-02: a refused takeover leaves the session covered, and the refused client is not the
/// holder afterwards either: a later request from it is refused again.
#[test]
fn a_refused_takeover_does_not_become_the_holder() {
    let refused = run(vec![
        plug("HEADLESS-1"),
        connected(7, ConnectionRole::Public),
        connected(8, ConnectionRole::Public),
        requested_by(7),
        lock(LockEvent::LockerGone),
        requested_by(8),
        requested_by(8),
    ]);
    assert_eq!(refused.model.lock(), &LockState::LockedWithoutSurface);
    assert_eq!(last(&refused), [Effect::Lock(LockReply::Finished)]);
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
    let base = run(vec![plug("HEADLESS-1")]);
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
