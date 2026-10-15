//! The pictures of Spaces that are not shown, and the two ways a window dropped in Mission
//! Control changes Space: onto a Space of another output and onto a new Space.

use crate::support;

use casement_core::OutputKey;
use casement_core::desktop::{DesktopRequest, WindowAsk};
use casement_core::focus::TokenCheck;
use casement_core::window::{SpacePlace, WindowState};
use casement_core::{Event, Model, WindowId};
use casement_protocols::vocab::{Assign, PlaceTarget};
use porter_core::SpaceId;
use support::driver::{Op, map, plug, run, run_from};
use support::{headless, proven};

fn ask(window: u64, ask: WindowAsk) -> Op {
    Op::Event(Event::Desktop(DesktopRequest::Window {
        window: WindowId(window),
        ask,
    }))
}

fn create_space(output: &OutputKey) -> Op {
    Op::Event(Event::Desktop(DesktopRequest::CreateSpace {
        output: output.clone(),
    }))
}

fn row(model: &Model, output: &OutputKey) -> Vec<SpaceId> {
    model
        .spaces()
        .row(output)
        .into_iter()
        .map(|space| space.id.clone())
        .collect()
}

fn sorted(mut ids: Vec<WindowId>) -> Vec<u64> {
    ids.sort();
    ids.into_iter().map(|id| id.0).collect()
}

/// Windows 1, 2 and 3 on HEADLESS-1, which has a second Space; window 2 sits on the second.
fn three_windows_on_two_spaces() -> (Model, SpaceId, SpaceId) {
    let key = headless("HEADLESS-1").key;
    let mail = proven("org.quire.Mail");
    let made = run(vec![
        plug("HEADLESS-1"),
        map(1, &mail, TokenCheck::Missing),
        map(2, &mail, TokenCheck::Missing),
        map(3, &mail, TokenCheck::Missing),
        create_space(&key),
    ]);
    let spaces = row(&made.model, &key);
    let (shown, hidden) = (spaces[0].clone(), spaces[1].clone());
    let moved = run_from(made.model, vec![ask(2, WindowAsk::ToSpace(hidden.clone()))]);
    (moved.model, shown, hidden)
}

#[test]
fn a_picture_shows_the_windows_of_a_space_whether_or_not_it_is_shown() {
    let (model, shown, hidden) = three_windows_on_two_spaces();
    assert_eq!(sorted(model.pictured(&shown)), vec![1, 3], "the shown one");
    assert_eq!(sorted(model.pictured(&hidden)), vec![2], "the hidden one");
    let nowhere = support::space_id("s-nowhere");
    assert!(
        model.pictured(&nowhere).is_empty(),
        "a Space that is not there"
    );
}

#[test]
fn a_picture_leaves_out_the_dock_and_adds_the_windows_on_every_space() {
    let (model, shown, hidden) = three_windows_on_two_spaces();
    let after = run_from(
        model,
        vec![
            ask(3, WindowAsk::Minimise),
            ask(1, WindowAsk::Assign(Assign::AllSpaces)),
        ],
    )
    .model;
    assert_eq!(sorted(after.pictured(&shown)), vec![1], "3 is in the dock");
    assert_eq!(
        sorted(after.pictured(&hidden)),
        vec![1, 2],
        "1 is on every Space"
    );
}

#[test]
fn a_window_dropped_on_a_new_space_gets_a_space_of_its_own() {
    let key = headless("HEADLESS-1").key;
    let made = run(vec![
        plug("HEADLESS-1"),
        map(1, &proven("org.quire.Mail"), TokenCheck::Missing),
    ]);
    let before = row(&made.model, &key);
    assert_eq!(before.len(), 1);
    let dropped = run_from(made.model, vec![ask(1, WindowAsk::ToNewSpace(key.clone()))]).model;
    let after = row(&dropped, &key);
    assert_eq!(after.len(), 2, "a Space was made");
    assert_eq!(after[0], before[0], "at the end of the row");
    assert_eq!(
        dropped.windows()[&WindowId(1)].place,
        SpacePlace::On(after[1].clone())
    );
    assert_eq!(dropped.pictured(&after[1]), vec![WindowId(1)]);
    assert_eq!(
        dropped
            .spaces()
            .shown_on(&key)
            .map(|space| space.id.clone()),
        Some(before[0].clone()),
        "Mission Control stays on the Space it was on"
    );
}

#[test]
fn a_drop_on_a_new_space_of_an_output_that_is_gone_changes_nothing() {
    let key = headless("HEADLESS-1").key;
    let gone = headless("HEADLESS-9").key;
    let made = run(vec![
        plug("HEADLESS-1"),
        map(1, &proven("org.quire.Mail"), TokenCheck::Missing),
    ]);
    let after = run_from(
        made.model.clone(),
        vec![ask(1, WindowAsk::ToNewSpace(gone.clone()))],
    )
    .model;
    assert_eq!(row(&after, &key).len(), 1);
    assert!(
        row(&after, &gone).is_empty(),
        "no Space is made for nothing"
    );
    assert_eq!(after.windows(), made.model.windows());
}

#[test]
fn a_window_dropped_on_another_outputs_space_keeps_its_place_about_the_corner() {
    let made = run(vec![
        plug("HEADLESS-1"),
        plug("HEADLESS-2"),
        map(1, &proven("org.quire.Mail"), TokenCheck::Missing),
        ask(1, WindowAsk::Place(PlaceTarget::Centre)),
    ]);
    let model = made.model;
    let start = model.windows()[&WindowId(1)].clone();
    let WindowState::Floating(was) = start.state else {
        panic!("a centred window floats: {:?}", start.state);
    };
    let from = model.output_of(&start).expect("an output");
    let to = [headless("HEADLESS-1").key, headless("HEADLESS-2").key]
        .into_iter()
        .find(|key| *key != from)
        .expect("the other output");
    let corner = |key: &OutputKey| model.outputs().0[key].config.logical_rect().origin;
    let (a, b) = (corner(&from), corner(&to));
    let target = row(&model, &to)[0].clone();
    let after = run_from(model, vec![ask(1, WindowAsk::ToSpace(target.clone()))]).model;
    let landed = &after.windows()[&WindowId(1)];
    assert_eq!(landed.place, SpacePlace::On(target));
    let WindowState::Floating(now) = landed.state else {
        panic!("still floating: {:?}", landed.state);
    };
    assert_eq!(
        now.origin.x - was.origin.x,
        b.x - a.x,
        "carried across in x"
    );
    assert_eq!(
        now.origin.y - was.origin.y,
        b.y - a.y,
        "carried across in y"
    );
    assert_eq!(now.size, was.size);
}
