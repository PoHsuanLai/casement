//! Model::step routes events to the machines and keeps every invariant: the six rows the
//! freeze wrote. The machines have their own tables (`model_*.rs`), and `model_props.rs` runs
//! random sequences of everything.

mod support;

use casement_core::focus::TokenCheck;
use casement_core::lock::{LockEvent, LockReply, LockState};
use casement_core::space::SpaceContent;
use casement_core::{ClientKey, Effect, Event, KeyboardTarget, WindowId};
use casement_protocols::vocab::Attention;
use support::driver::{Op, fresh, last, map, plug, run};
use support::{headless, proven};

#[test]
fn a_new_model_keeps_its_invariants() {
    assert_eq!(fresh().check_invariants(), Ok(()));
}

#[test]
fn a_plugged_output_shows_a_fresh_desktop() {
    let key = headless("HEADLESS-1").key;
    let run = run(vec![plug("HEADLESS-1")]);
    assert!(run.model.outputs().0.contains_key(&key));
    let row = run.model.spaces().row(&key);
    assert_eq!(row.len(), 1);
    assert_eq!(row[0].content, SpaceContent::Desktop);
    assert_eq!(
        run.model.spaces().shown_on(&key).map(|s| &s.id),
        Some(&row[0].id)
    );
    assert!(
        last(&run)
            .iter()
            .any(|e| matches!(e, Effect::ApplyOutput { output, .. } if *output == key))
    );
}

#[test]
fn a_window_mapped_with_a_valid_token_takes_the_keyboard() {
    let mail = proven("org.quire.Mail");
    let run = run(vec![plug("HEADLESS-1"), map(1, &mail, TokenCheck::Valid)]);
    assert!(run.model.windows().contains_key(&WindowId(1)));
    assert_eq!(run.model.focus().key, Some(WindowId(1)));
    assert_eq!(run.model.focus().active, Some(mail));
    let effects = last(&run);
    assert!(effects.contains(&Effect::Keyboard(KeyboardTarget::Window(WindowId(1)))));
    assert!(
        effects
            .iter()
            .any(|e| matches!(e, Effect::Configure { window, .. } if *window == WindowId(1)))
    );
}

#[test]
fn a_window_without_a_token_does_not_steal_focus() {
    let mail = proven("org.quire.Mail");
    let web = proven("org.example.Web");
    let run = run(vec![
        plug("HEADLESS-1"),
        map(1, &mail, TokenCheck::Valid),
        map(2, &web, TokenCheck::Missing),
    ]);
    assert_eq!(run.model.focus().key, Some(WindowId(1)));
    let effects = last(&run);
    assert!(effects.contains(&Effect::Attention {
        app: web,
        level: Attention::Informational
    }));
    assert!(!effects.iter().any(|e| matches!(e, Effect::Keyboard(_))));
}

#[test]
fn unmapping_the_key_window_moves_the_keyboard() {
    let mail = proven("org.quire.Mail");
    let run = run(vec![
        plug("HEADLESS-1"),
        map(1, &mail, TokenCheck::Valid),
        map(2, &mail, TokenCheck::Valid),
        Op::Event(Event::ToplevelUnmapped {
            window: WindowId(2),
        }),
    ]);
    assert!(!run.model.windows().contains_key(&WindowId(2)));
    assert_eq!(run.model.focus().key, Some(WindowId(1)));
    assert!(last(&run).contains(&Effect::Keyboard(KeyboardTarget::Window(WindowId(1)))));
}

#[test]
fn a_lock_whose_locker_dies_stays_covered() {
    let key = headless("HEADLESS-1").key;
    let run = run(vec![
        plug("HEADLESS-1"),
        Op::Event(Event::Lock(LockEvent::Requested {
            client: ClientKey(7),
        })),
        Op::Event(Event::Lock(LockEvent::SurfaceCommitted {
            output: key.clone(),
        })),
        Op::Event(Event::Lock(LockEvent::LockerGone)),
    ]);
    assert_eq!(run.model.lock(), &LockState::LockedWithoutSurface);
    assert!(last(&run).contains(&Effect::Lock(LockReply::ShowCover(key))));
}
