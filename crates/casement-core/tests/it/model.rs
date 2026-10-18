//! Model::step routes events to the machines and keeps every invariant: the six rows the
//! freeze wrote. The machines have their own tables (`model_*.rs`), and `model_props.rs` runs
//! random sequences of everything.

use crate::support;

use casement_core::focus::{Focus, TokenCheck};
use casement_core::lock::{LockEvent, LockHolder, LockReply, LockState};
use casement_core::settings::CasementSettings;
use casement_core::space::{SpaceContent, SpaceSeed};
use casement_core::{ClientKey, Effect, Event, KeyboardTarget, Model, WindowId};
use casement_protocols::vocab::Attention;
use support::driver::{Op, fresh, last, map, plug, run};
use support::{headless, ms, proven};

/// A new model: invariants hold, and it is an empty, unlocked desktop that holds its settings
/// and epoch.
#[test]
fn a_new_model_is_an_empty_desktop_that_keeps_its_invariants() {
    let model = Model::new(
        CasementSettings::defaults(),
        ms(5),
        SpaceSeed::from_entropy([1; 16]),
    );
    assert_eq!(model.check_invariants(), Ok(()), "invariants");
    assert!(model.windows().is_empty(), "no windows");
    assert!(model.apps().is_empty(), "no apps");
    assert!(model.outputs().0.is_empty(), "no outputs");
    assert_eq!(model.lock(), &LockState::Unlocked, "unlocked");
    assert_eq!(model.focus(), &Focus::none(), "no focus");
    assert_eq!(model.timers().next(), None, "no timers");
    assert_eq!(model.settings(), &CasementSettings::defaults(), "settings");
    assert_eq!(model.now(), ms(5), "epoch");
}

#[test]
fn the_pointer_starts_at_the_centre_of_the_primary_output() {
    use casement_core::geometry::LogicalPoint;
    assert_eq!(fresh().pointer_start(), None, "no output, no start");
    let run = run(vec![plug("HEADLESS-1")]);
    assert_eq!(
        run.model.pointer_start(),
        Some(LogicalPoint { x: 960, y: 540 })
    );
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
    assert_eq!(
        run.model.lock(),
        &LockState::LockedWithoutSurface {
            holder: LockHolder::Client(ClientKey(7))
        }
    );
    assert!(last(&run).contains(&Effect::Lock(LockReply::ShowCover(key))));
}
