//! The events the edge's wiring added: window listing effects, a dialog hint that changes
//! after the map, and an interactive drag. Every row runs with the invariants checked after
//! each step.

mod support;

use casement_core::focus::TokenCheck;
use casement_core::geometry::{LogicalPoint, LogicalRect, LogicalSize};
use casement_core::window::{Parent, WindowState, WindowTitle};
use casement_core::{Cause, Effect, Event, MappedToplevel, ToplevelMeta, WindowId};
use casement_protocols::vocab::Modality;
use support::driver::{Op, last, map, mapped, plug, run, unmap};
use support::proven;

#[test]
fn a_window_is_listed_when_it_maps_relisted_when_it_is_named_and_unlisted_when_it_goes() {
    let mail = proven("org.quire.Mail");
    let run = run(vec![plug("HEADLESS-1"), map(1, &mail, TokenCheck::Valid)]);
    assert!(last(&run).contains(&Effect::ListWindow(WindowId(1))));
    let run = support::driver::run_from(
        run.model,
        vec![Op::Event(Event::ToplevelMeta {
            window: WindowId(1),
            meta: ToplevelMeta::Title(WindowTitle("Inbox".to_owned())),
        })],
    );
    assert!(last(&run).contains(&Effect::ListWindow(WindowId(1))));
    let run = support::driver::run_from(run.model, vec![unmap(1)]);
    assert!(last(&run).contains(&Effect::UnlistWindow(WindowId(1))));
}

#[test]
fn a_dialog_hint_set_after_the_map_opens_and_closes_a_sheet() {
    let mail = proven("org.quire.Mail");
    let child = MappedToplevel {
        parent: Parent::Window(WindowId(1)),
        modality: Modality::Modeless,
        ..mapped(2, &mail, TokenCheck::Missing)
    };
    let hint = |modality| {
        Op::Event(Event::ToplevelModality {
            window: WindowId(2),
            modality,
        })
    };
    let run = run(vec![
        plug("HEADLESS-1"),
        map(1, &mail, TokenCheck::Valid),
        Op::Event(Event::ToplevelMapped(child)),
        hint(Modality::WindowModal),
    ]);
    assert_eq!(
        run.model.windows()[&WindowId(2)].modality,
        Modality::WindowModal
    );
    assert_eq!(
        run.model.focus().key,
        Some(WindowId(2)),
        "a window that now blocks its parent is its sheet and takes the key"
    );
    let run = support::driver::run_from(run.model, vec![hint(Modality::Modeless)]);
    assert_eq!(
        run.model.windows()[&WindowId(2)].modality,
        Modality::Modeless
    );
    assert_eq!(
        run.model.focus().key,
        Some(WindowId(1)),
        "the sheet closed: the parent has the key"
    );
    let unknown = support::driver::run_from(run.model, vec![hint(Modality::None)]);
    assert!(unknown.model.check_invariants().is_ok());
}

#[test]
fn a_drag_floats_the_window_where_the_pointer_took_it() {
    let mail = proven("org.quire.Mail");
    let there = LogicalRect::new(
        LogicalPoint { x: 100, y: 50 },
        LogicalSize { w: 500, h: 400 },
    );
    let run = run(vec![
        plug("HEADLESS-1"),
        map(1, &mail, TokenCheck::Valid),
        Op::Event(Event::WindowDragged {
            window: WindowId(1),
            rect: there,
        }),
    ]);
    assert_eq!(
        run.model.windows()[&WindowId(1)].state,
        WindowState::Floating(there)
    );
    assert!(last(&run).contains(&Effect::Relayout(Cause::Drag)));
    let again = support::driver::run_from(
        run.model,
        vec![Op::Event(Event::WindowDragged {
            window: WindowId(1),
            rect: there,
        })],
    );
    assert!(
        !last(&again)
            .iter()
            .any(|effect| matches!(effect, Effect::Relayout(_))),
        "a drag that moves nothing lays nothing out"
    );
}
