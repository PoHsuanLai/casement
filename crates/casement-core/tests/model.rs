//! Model::step routes events to the machines and keeps every invariant (what P0-F1b-model
//! makes pass; the T0 property runs random event sequences).

mod support;

use casement_core::app::AppKey;
use casement_core::focus::TokenCheck;
use casement_core::geometry::LogicalSize;
use casement_core::lock::{LockEvent, LockReply};
use casement_core::settings::CasementSettings;
use casement_core::space::{SpaceContent, SpaceSeed};
use casement_core::window::{MaxSize, Parent, SizeLimits};
use casement_core::{ClientKey, Effect, Event, KeyboardTarget, MappedToplevel, Model, WindowId};
use casement_protocols::vocab::{Attention, Modality, WindowRole};
use proptest::prelude::*;
use support::{claimed, headless, ms, proven};

fn fresh() -> Model {
    Model::new(
        CasementSettings::defaults(),
        ms(0),
        SpaceSeed::from_entropy([9; 16]),
    )
}

fn mapped(n: u64, app: &AppKey, activation: TokenCheck) -> Event {
    Event::ToplevelMapped(MappedToplevel {
        window: WindowId(n),
        client: ClientKey(n),
        app: app.clone(),
        claimed_id: claimed("test"),
        role: WindowRole::Document,
        modality: Modality::None,
        parent: Parent::None,
        limits: SizeLimits {
            min: LogicalSize { w: 0, h: 0 },
            max: MaxSize::Unbounded,
        },
        first_size: LogicalSize { w: 800, h: 600 },
        activation,
    })
}

/// Steps `events` in order at 1 ms apart, checking the invariants after each.
fn run(events: Vec<Event>) -> (Model, Vec<Vec<Effect>>) {
    events.into_iter().enumerate().fold(
        (fresh(), Vec::new()),
        |(model, mut effects), (n, event)| {
            let (model, out) = model.step(event, ms(n as u64 + 1));
            assert_eq!(model.check_invariants(), Ok(()), "after event {n}");
            effects.push(out);
            (model, effects)
        },
    )
}

#[test]
#[ignore = "P0-F1b-model"]
fn a_new_model_keeps_its_invariants() {
    assert_eq!(fresh().check_invariants(), Ok(()));
}

#[test]
#[ignore = "P0-F1b-model"]
fn a_plugged_output_shows_a_fresh_desktop() {
    let facts = headless("HEADLESS-1");
    let key = facts.key.clone();
    let (model, effects) = run(vec![Event::OutputPlugged(facts)]);
    assert!(model.outputs().0.contains_key(&key));
    let row = model.spaces().row(&key);
    assert_eq!(row.len(), 1);
    assert_eq!(row[0].content, SpaceContent::Desktop);
    assert_eq!(
        model.spaces().shown_on(&key).map(|s| &s.id),
        Some(&row[0].id)
    );
    assert!(
        effects[0]
            .iter()
            .any(|e| matches!(e, Effect::ApplyOutput { output, .. } if *output == key))
    );
}

#[test]
#[ignore = "P0-F1b-model"]
fn a_window_mapped_with_a_valid_token_takes_the_keyboard() {
    let mail = proven("org.quire.Mail");
    let (model, effects) = run(vec![
        Event::OutputPlugged(headless("HEADLESS-1")),
        mapped(1, &mail, TokenCheck::Valid),
    ]);
    assert!(model.windows().contains_key(&WindowId(1)));
    assert_eq!(model.focus().key, Some(WindowId(1)));
    assert_eq!(model.focus().active, Some(mail));
    assert!(effects[1].contains(&Effect::Keyboard(KeyboardTarget::Window(WindowId(1)))));
    assert!(
        effects[1]
            .iter()
            .any(|e| matches!(e, Effect::Configure { window, .. } if *window == WindowId(1)))
    );
}

#[test]
#[ignore = "P0-F1b-model"]
fn a_window_without_a_token_does_not_steal_focus() {
    let mail = proven("org.quire.Mail");
    let web = proven("org.example.Web");
    let (model, effects) = run(vec![
        Event::OutputPlugged(headless("HEADLESS-1")),
        mapped(1, &mail, TokenCheck::Valid),
        mapped(2, &web, TokenCheck::Missing),
    ]);
    assert_eq!(model.focus().key, Some(WindowId(1)));
    assert!(effects[2].contains(&Effect::Attention {
        app: web,
        level: Attention::Informational
    }));
    assert!(!effects[2].iter().any(|e| matches!(e, Effect::Keyboard(_))));
}

#[test]
#[ignore = "P0-F1b-model"]
fn unmapping_the_key_window_moves_the_keyboard() {
    let mail = proven("org.quire.Mail");
    let (model, effects) = run(vec![
        Event::OutputPlugged(headless("HEADLESS-1")),
        mapped(1, &mail, TokenCheck::Valid),
        mapped(2, &mail, TokenCheck::Valid),
        Event::ToplevelUnmapped {
            window: WindowId(2),
        },
    ]);
    assert!(!model.windows().contains_key(&WindowId(2)));
    assert_eq!(model.focus().key, Some(WindowId(1)));
    assert!(effects[3].contains(&Effect::Keyboard(KeyboardTarget::Window(WindowId(1)))));
}

#[test]
#[ignore = "P0-F1b-model"]
fn a_lock_whose_locker_dies_stays_covered() {
    let output = headless("HEADLESS-1");
    let key = output.key.clone();
    let (model, effects) = run(vec![
        Event::OutputPlugged(output),
        Event::Lock(LockEvent::Requested {
            client: ClientKey(7),
        }),
        Event::Lock(LockEvent::SurfaceCommitted {
            output: key.clone(),
        }),
        Event::Lock(LockEvent::LockerGone),
    ]);
    assert_eq!(
        model.lock(),
        &casement_core::lock::LockState::LockedWithoutSurface
    );
    assert!(effects[3].contains(&Effect::Lock(LockReply::ShowCover(key))));
}

/// A small alphabet of events over two outputs, three apps and six windows.
fn event() -> impl Strategy<Value = Event> {
    let app = prop_oneof![
        Just(proven("org.example.A")),
        Just(proven("org.example.B")),
        Just(AppKey::Claimed(claimed("c")))
    ];
    let token = prop_oneof![
        Just(TokenCheck::Valid),
        Just(TokenCheck::Stale),
        Just(TokenCheck::Missing)
    ];
    prop_oneof![
        prop_oneof![Just("HEADLESS-1"), Just("HEADLESS-2")]
            .prop_map(|name| Event::OutputPlugged(headless(name))),
        prop_oneof![Just("HEADLESS-1"), Just("HEADLESS-2")]
            .prop_map(|name| Event::OutputUnplugged(headless(name).key)),
        (1u64..7, app, token).prop_map(|(n, app, token)| mapped(n, &app, token)),
        (1u64..7).prop_map(|n| Event::ToplevelUnmapped {
            window: WindowId(n)
        }),
        (1u64..7).prop_map(|n| Event::ClientGone {
            client: ClientKey(n)
        }),
    ]
}

proptest! {
    #[test]
    #[ignore = "P0-F1b-model"]
    fn random_sequences_keep_every_invariant(events in proptest::collection::vec(event(), 0..40)) {
        let (model, _) = run(events.clone());
        let (again, _) = run(events);
        prop_assert_eq!(model, again, "step is deterministic");
    }
}
