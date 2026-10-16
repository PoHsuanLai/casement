//! Spaces and outputs through `Model::step`: a desktop per output, full-screen Spaces,
//! switching, an output leaving and returning.

use crate::support;

use casement_core::focus::TokenCheck;
use casement_core::geometry::{LogicalPoint, Transform};
use casement_core::input::{DesktopCommand, SpaceStep};
use casement_core::output::OutputPower;
use casement_core::space::SpaceContent;
use casement_core::window::{SpacePlace, WindowState};
use casement_core::{Cause, Effect, Event, ToplevelRequest, WindowId};
use casement_protocols::vocab::Toggle;
use porter_core::SpaceId;
use support::driver::{
    Op, Run, assert_same_world, last, map, plug, request, run, run_from, unmap, unplug,
};
use support::{display, headless, proven};

fn key(name: &str) -> casement_core::OutputKey {
    headless(name).key
}

fn full_screen(n: u64) -> Op {
    request(
        n,
        ToplevelRequest::FullScreen {
            toggle: Toggle::On,
            output: None,
        },
    )
}

fn leave_full_screen(n: u64) -> Op {
    request(
        n,
        ToplevelRequest::FullScreen {
            toggle: Toggle::Off,
            output: None,
        },
    )
}

/// The ids of `name`'s row, left to right.
fn row(run: &Run, name: &str) -> Vec<SpaceId> {
    run.model
        .spaces()
        .row(&key(name))
        .into_iter()
        .map(|space| space.id.clone())
        .collect()
}

fn shown(run: &Run, name: &str) -> SpaceId {
    run.model
        .spaces()
        .shown_on(&key(name))
        .expect("shown")
        .id
        .clone()
}

#[test]
fn a_second_output_is_placed_right_of_the_first_and_gets_its_own_desktop() {
    let run = run(vec![plug("HEADLESS-1"), plug("HEADLESS-2")]);
    let outputs = &run.model.outputs().0;
    assert_eq!(
        outputs[&key("HEADLESS-1")].config.position,
        LogicalPoint { x: 0, y: 0 }
    );
    assert_eq!(
        outputs[&key("HEADLESS-2")].config.position,
        LogicalPoint { x: 1920, y: 0 }
    );
    assert_ne!(shown(&run, "HEADLESS-1"), shown(&run, "HEADLESS-2"));
    let effects = last(&run);
    assert!(effects.contains(&Effect::RequestFrames(key("HEADLESS-2"))));
    assert!(effects.contains(&Effect::Relayout(Cause::OutputChange)));
}

#[test]
fn plugging_a_known_output_again_only_replaces_its_facts() {
    let first = run(vec![plug("HEADLESS-1")]);
    let again = run_from(first.model.clone(), vec![plug("HEADLESS-1")]);
    assert_same_world(&again.model, &first.model, "unchanged");
    assert_eq!(last(&again), []);
}

#[test]
fn a_display_with_no_mode_is_left_dark() {
    let run = run(vec![Op::Event(Event::OutputPlugged(display(
        "HEADLESS-1",
        vec![],
        (0, 0),
    )))]);
    assert_eq!(
        run.model.outputs().0[&key("HEADLESS-1")].config.power,
        OutputPower::Off
    );
}

#[test]
fn a_configured_output_replaces_its_configuration_and_lays_out_again() {
    let plugged = run(vec![plug("HEADLESS-1")]);
    let mut config = plugged.model.outputs().0[&key("HEADLESS-1")].config;
    config.transform = Transform::R90;
    let run = run_from(
        plugged.model,
        vec![Op::Event(Event::OutputConfigured {
            output: key("HEADLESS-1"),
            config,
        })],
    );
    assert_eq!(run.model.outputs().0[&key("HEADLESS-1")].config, config);
    assert!(last(&run).contains(&Effect::Relayout(Cause::OutputChange)));
    let unknown = run_from(
        run.model.clone(),
        vec![Op::Event(Event::OutputConfigured {
            output: key("HEADLESS-9"),
            config,
        })],
    );
    assert_same_world(&unknown.model, &run.model, "unchanged");
}

#[test]
fn full_screen_takes_a_space_at_the_end_of_the_row_and_leaving_ends_it() {
    let mail = proven("org.quire.Mail");
    let entered = run(vec![
        plug("HEADLESS-1"),
        map(1, &mail, TokenCheck::Valid),
        full_screen(1),
    ]);
    let ids = row(&entered, "HEADLESS-1");
    assert_eq!(ids.len(), 2);
    let space = &ids[1];
    assert_eq!(
        entered.model.spaces().get(space).map(|s| s.content.clone()),
        Some(SpaceContent::FullScreen(WindowId(1)))
    );
    assert_eq!(shown(&entered, "HEADLESS-1"), *space, "the Space is shown");
    assert_eq!(
        entered.model.windows()[&WindowId(1)].place,
        SpacePlace::On(space.clone())
    );
    assert!(last(&entered).contains(&Effect::Relayout(Cause::Configure)));

    let left = run_from(entered.model, vec![leave_full_screen(1)]);
    assert_eq!(row(&left, "HEADLESS-1"), vec![ids[0].clone()]);
    assert_eq!(shown(&left, "HEADLESS-1"), ids[0]);
    assert_eq!(
        left.model.windows()[&WindowId(1)].place,
        SpacePlace::On(ids[0].clone())
    );
    assert_eq!(
        left.model.windows()[&WindowId(1)].state,
        WindowState::New(support::driver::mapped(1, &mail, TokenCheck::Valid).first_size)
    );
}

#[test]
fn a_full_screen_window_unmapping_ends_its_space_and_shows_the_desktop() {
    let mail = proven("org.quire.Mail");
    let run = run(vec![
        plug("HEADLESS-1"),
        map(1, &mail, TokenCheck::Valid),
        map(2, &mail, TokenCheck::Valid),
        full_screen(2),
        unmap(2),
    ]);
    assert_eq!(row(&run, "HEADLESS-1").len(), 1);
    assert_eq!(shown(&run, "HEADLESS-1"), row(&run, "HEADLESS-1")[0]);
    assert_eq!(run.model.focus().key, Some(WindowId(1)));
}

#[test]
fn a_full_screen_space_that_is_not_shown_ending_leaves_the_shown_space_alone() {
    let (mail, web) = (proven("org.quire.Mail"), proven("org.example.Web"));
    let run = run(vec![
        plug("HEADLESS-1"),
        map(1, &mail, TokenCheck::Valid),
        full_screen(1),
        Op::Command(DesktopCommand::SwitchSpace(SpaceStep::Previous)),
        map(2, &web, TokenCheck::Valid),
        unmap(1),
    ]);
    assert_eq!(row(&run, "HEADLESS-1").len(), 1);
    assert_eq!(run.model.focus().key, Some(WindowId(2)));
}

#[test]
fn a_window_that_maps_while_a_full_screen_space_is_shown_goes_to_the_desktop_and_follows() {
    let (mail, web) = (proven("org.quire.Mail"), proven("org.example.Web"));
    let run = run(vec![
        plug("HEADLESS-1"),
        map(1, &mail, TokenCheck::Valid),
        full_screen(1),
        map(2, &web, TokenCheck::Valid),
    ]);
    let ids = row(&run, "HEADLESS-1");
    assert_eq!(
        run.model.windows()[&WindowId(2)].place,
        SpacePlace::On(ids[0].clone())
    );
    assert_eq!(
        shown(&run, "HEADLESS-1"),
        ids[0],
        "the Space switches to it"
    );
    assert_eq!(run.model.focus().key, Some(WindowId(2)));
}

/// Each window lives in the column of its Space in the row; the renderer lays the row out by it.
#[test]
fn each_window_has_the_column_of_its_space() {
    let mail = proven("org.quire.Mail");
    let run = run(vec![
        plug("HEADLESS-1"),
        map(1, &mail, TokenCheck::Valid),
        map(2, &mail, TokenCheck::Valid),
        full_screen(2),
        map(3, &mail, TokenCheck::Valid),
        full_screen(3),
    ]);
    let columns = run.model.row_columns(&key("HEADLESS-1"));
    let mut taken: Vec<u32> = columns.values().copied().collect();
    taken.sort_unstable();
    assert_eq!(taken, vec![0, 1, 2], "three Spaces, a window on each");
    assert_eq!(columns[&WindowId(1)], 0, "the desktop is first");
    assert!(run.model.row_columns(&key("HEADLESS-2")).is_empty());
}

#[test]
fn switching_walks_the_row_and_stops_at_its_ends() {
    let mail = proven("org.quire.Mail");
    let base = run(vec![
        plug("HEADLESS-1"),
        map(1, &mail, TokenCheck::Valid),
        map(2, &mail, TokenCheck::Valid),
        full_screen(2),
        map(3, &mail, TokenCheck::Valid),
        full_screen(3),
    ]);
    let ids = row(&base, "HEADLESS-1");
    assert_eq!(ids.len(), 3);
    let cases = [
        ("previous", SpaceStep::Previous, 1usize),
        ("number 1", SpaceStep::Number(1), 0),
        ("number 2", SpaceStep::Number(2), 1),
        ("next from the end stays", SpaceStep::Next, 2),
        ("number 4 is no Space", SpaceStep::Number(4), 2),
        ("number 0 is no Space", SpaceStep::Number(0), 2),
    ];
    for (name, step, want) in cases {
        let run = run_from(
            base.model.clone(),
            vec![Op::Command(DesktopCommand::SwitchSpace(step))],
        );
        assert_eq!(shown(&run, "HEADLESS-1"), ids[want], "{name}");
    }
    let walked = run_from(
        base.model.clone(),
        vec![
            Op::Command(DesktopCommand::SwitchSpace(SpaceStep::Previous)),
            Op::Command(DesktopCommand::SwitchSpace(SpaceStep::Previous)),
            Op::Command(DesktopCommand::SwitchSpace(SpaceStep::Previous)),
            Op::Command(DesktopCommand::SwitchSpace(SpaceStep::Next)),
        ],
    );
    assert_eq!(shown(&walked, "HEADLESS-1"), ids[1]);
    let moved = last(&walked);
    assert!(moved.contains(&Effect::Relayout(Cause::SpaceSwitch)));
    assert!(moved.contains(&Effect::RequestFrames(key("HEADLESS-1"))));
}

#[test]
fn switching_to_a_desktop_without_the_key_window_moves_the_key_to_what_is_there() {
    let (mail, web) = (proven("org.quire.Mail"), proven("org.example.Web"));
    let run = run(vec![
        plug("HEADLESS-1"),
        map(1, &web, TokenCheck::Valid),
        map(2, &mail, TokenCheck::Valid),
        full_screen(2),
        Op::Command(DesktopCommand::SwitchSpace(SpaceStep::Previous)),
    ]);
    assert_eq!(run.model.focus().active, Some(web));
    assert_eq!(run.model.focus().key, Some(WindowId(1)));
}

#[test]
fn unplugging_an_output_moves_its_spaces_and_windows_to_the_one_left() {
    let (mail, web) = (proven("org.quire.Mail"), proven("org.example.Web"));
    let before = run(vec![
        plug("HEADLESS-1"),
        plug("HEADLESS-2"),
        map(1, &mail, TokenCheck::Valid),
        map(2, &web, TokenCheck::Valid),
    ]);
    let second_desktop = shown(&before, "HEADLESS-2");
    let after = run_from(before.model, vec![unplug("HEADLESS-2")]);
    assert!(!after.model.outputs().0.contains_key(&key("HEADLESS-2")));
    let ids = row(&after, "HEADLESS-1");
    assert_eq!(ids.len(), 2);
    assert_eq!(ids[1], second_desktop, "it joins the end of the row");
    assert_eq!(
        after
            .model
            .spaces()
            .shown_on(&key("HEADLESS-1"))
            .map(|s| s.position.0),
        Some(0),
        "the survivor keeps showing its own"
    );
    assert!(last(&after).contains(&Effect::Relayout(Cause::OutputChange)));
}

#[test]
fn the_key_window_on_the_unplugged_output_gives_the_key_to_what_is_still_shown() {
    let (mail, web) = (proven("org.quire.Mail"), proven("org.example.Web"));
    let on_second = request(
        2,
        ToplevelRequest::FullScreen {
            toggle: Toggle::On,
            output: Some(key("HEADLESS-2")),
        },
    );
    let before = run(vec![
        plug("HEADLESS-1"),
        plug("HEADLESS-2"),
        map(1, &web, TokenCheck::Valid),
        map(2, &mail, TokenCheck::Valid),
        on_second,
    ]);
    assert_eq!(before.model.focus().key, Some(WindowId(2)));
    let fullscreen_space = shown(&before, "HEADLESS-2");
    let after = run_from(before.model, vec![unplug("HEADLESS-2")]);
    assert_eq!(row(&after, "HEADLESS-1").last(), Some(&fullscreen_space));
    assert_ne!(
        shown(&after, "HEADLESS-1"),
        fullscreen_space,
        "it is not shown there"
    );
    assert_eq!(after.model.focus().key, Some(WindowId(1)));
    assert_eq!(after.model.focus().active, Some(web));
}

#[test]
fn the_last_output_leaving_makes_every_window_wait_and_the_next_takes_them_in() {
    let (mail, bar) = (proven("org.quire.Mail"), proven("org.quire.Bar"));
    let shell = Op::Event(Event::ToplevelMapped(casement_core::MappedToplevel {
        role: casement_protocols::vocab::WindowRole::Shell,
        ..support::driver::mapped(3, &bar, TokenCheck::Missing)
    }));
    let before = run(vec![
        plug("HEADLESS-1"),
        map(1, &mail, TokenCheck::Valid),
        map(2, &mail, TokenCheck::Valid),
        full_screen(2),
        shell,
    ]);
    let gone = run_from(before.model, vec![unplug("HEADLESS-1")]);
    assert_eq!(gone.model.spaces().iter().count(), 0);
    for window in gone.model.windows().values() {
        assert_eq!(window.place, SpacePlace::AllSpaces, "{:?}", window.id);
        assert_eq!(window.home, None, "{:?}", window.id);
    }
    assert!(
        matches!(
            gone.model.windows()[&WindowId(2)].state,
            WindowState::New(_)
        ),
        "full screen ended with its Space"
    );
    assert_eq!(gone.model.focus().key, Some(WindowId(2)), "the key stays");

    let back = run_from(gone.model, vec![plug("HEADLESS-1")]);
    let desktop = shown(&back, "HEADLESS-1");
    assert_eq!(
        back.model.windows()[&WindowId(1)].place,
        SpacePlace::On(desktop.clone())
    );
    assert_eq!(
        back.model.windows()[&WindowId(2)].place,
        SpacePlace::On(desktop)
    );
    assert_eq!(
        back.model.windows()[&WindowId(3)].place,
        SpacePlace::AllSpaces
    );
    assert_eq!(
        back.model.windows()[&WindowId(3)].home,
        Some(key("HEADLESS-1"))
    );
}

#[test]
fn unplugging_an_unknown_output_does_nothing() {
    let first = run(vec![plug("HEADLESS-1")]);
    let again = run_from(first.model.clone(), vec![unplug("HEADLESS-2")]);
    assert_same_world(&again.model, &first.model, "unchanged");
    assert_eq!(last(&again), []);
}
