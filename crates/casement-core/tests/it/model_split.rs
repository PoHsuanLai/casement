//! Split View through `Model::step`: entering from the window menu, choosing the other half,
//! the pick, a cancel, leaving, closing one window and the divider.

use crate::support::driver::{Op, Run, last, map, plug, request, run, run_from, unmap};
use crate::support::{headless, proven};
use casement_core::desktop::{DesktopRequest, WindowAsk};
use casement_core::focus::TokenCheck;
use casement_core::input::{DesktopCommand, SpaceStep};
use casement_core::shell::{ShellNote, ShellRequest, SplitChoices};
use casement_core::space::{SpaceContent, SplitPair};
use casement_core::window::{MenuEntry, SplitSide, WindowState};
use casement_core::{Effect, Event, ToplevelRequest, WindowId};
use ds_core::vocab::Fraction;

fn menu(window: u64, entry: MenuEntry) -> Op {
    Op::Event(Event::Shell(ShellRequest::WindowMenuPick {
        window: WindowId(window),
        entry,
    }))
}

fn shell(request: ShellRequest) -> Op {
    Op::Event(Event::Shell(request))
}

fn pick(window: u64) -> Op {
    shell(ShellRequest::SplitPick {
        window: WindowId(window),
    })
}

/// An output with window 1 (the one that splits) and then windows 2, 3, ... of the same app.
fn desktop(windows: u64) -> Vec<Op> {
    let mail = proven("org.quire.Mail");
    let first = map(1, &mail, TokenCheck::Valid);
    let others = (2..=windows).map(|n| map(n, &mail, TokenCheck::Missing));
    [plug("HEADLESS-1"), first]
        .into_iter()
        .chain(others)
        .collect()
}

fn then(mut ops: Vec<Op>, more: Vec<Op>) -> Run {
    ops.extend(more);
    run(ops)
}

/// The shell notes of the last step.
fn notes(run: &Run) -> Vec<ShellNote> {
    last(run)
        .iter()
        .filter_map(|effect| match effect {
            Effect::Shell(note) => Some(note.clone()),
            _ => None,
        })
        .collect()
}

/// The contents of every Space that is not a desktop.
fn kept(run: &Run) -> Vec<SpaceContent> {
    run.model
        .spaces()
        .iter()
        .map(|space| space.content.clone())
        .filter(|content| !matches!(content, SpaceContent::Desktop))
        .collect()
}

fn pair(leading: u64, trailing: u64, divider: u16) -> SpaceContent {
    SpaceContent::SplitView(SplitPair {
        leading: WindowId(leading),
        trailing: WindowId(trailing),
        divider: Fraction(divider),
    })
}

fn full(run: &Run, n: u64) -> bool {
    matches!(
        run.model.windows()[&WindowId(n)].state,
        WindowState::FullScreen { .. }
    )
}

fn choose(first: u64, side: SplitSide, candidates: &[u64]) -> ShellNote {
    ShellNote::SplitChoose(SplitChoices::new(
        WindowId(first),
        side,
        candidates.iter().map(|n| WindowId(*n)).collect(),
    ))
}

#[test]
fn entering_asks_the_shell_for_the_other_half_only_when_there_is_one() {
    let left = MenuEntry::TileLeftOfScreen;
    let right = MenuEntry::TileRightOfScreen;
    // A name, how many windows, the entry picked, the notes the shell hears.
    let cases = [
        ("alone: just full screen", 1, left, vec![]),
        (
            "one other, left",
            2,
            left,
            vec![choose(1, SplitSide::Left, &[2])],
        ),
        (
            "two others, right, the newer first",
            3,
            right,
            vec![choose(1, SplitSide::Right, &[3, 2])],
        ),
    ];
    for (name, count, entry, want) in cases {
        let got = then(desktop(count), vec![menu(1, entry)]);
        assert!(full(&got, 1), "{name}: the window is full screen");
        assert_eq!(notes(&got), want, "{name}");
        assert_eq!(
            kept(&got),
            vec![SpaceContent::FullScreen(WindowId(1))],
            "{name}"
        );
    }
}

#[test]
fn only_ordinary_visible_windows_can_be_picked() {
    let got = then(
        desktop(3),
        vec![
            request(3, ToplevelRequest::Minimise),
            menu(1, MenuEntry::TileLeftOfScreen),
        ],
    );
    assert_eq!(notes(&got), vec![choose(1, SplitSide::Left, &[2])]);
}

#[test]
fn a_pick_completes_the_split_view() {
    let cases = [
        ("left", MenuEntry::TileLeftOfScreen, pair(1, 2, 500)),
        ("right", MenuEntry::TileRightOfScreen, pair(2, 1, 500)),
    ];
    for (name, entry, want) in cases {
        let got = then(desktop(2), vec![menu(1, entry), pick(2)]);
        assert_eq!(kept(&got), vec![want], "{name}");
        assert!(
            full(&got, 1) && full(&got, 2),
            "{name}: both are full screen"
        );
        assert_eq!(
            notes(&got),
            vec![ShellNote::SplitChoiceEnd(WindowId(1))],
            "{name}"
        );
        let space = |n: u64| got.model.windows()[&WindowId(n)].place.clone();
        assert_eq!(space(1), space(2), "{name}: one Space");
    }
}

#[test]
fn a_cancel_leaves_the_first_window_alone_and_a_later_pick_does_nothing() {
    let cancelled = then(
        desktop(2),
        vec![
            menu(1, MenuEntry::TileLeftOfScreen),
            shell(ShellRequest::SplitCancel),
        ],
    );
    assert_eq!(
        notes(&cancelled),
        vec![ShellNote::SplitChoiceEnd(WindowId(1))]
    );
    assert_eq!(
        kept(&cancelled),
        vec![SpaceContent::FullScreen(WindowId(1))]
    );
    let late = then(
        desktop(2),
        vec![
            menu(1, MenuEntry::TileLeftOfScreen),
            shell(ShellRequest::SplitCancel),
            pick(2),
        ],
    );
    assert_eq!(kept(&late), vec![SpaceContent::FullScreen(WindowId(1))]);
    assert!(!full(&late, 2));
    let none = then(desktop(2), vec![pick(2)]);
    assert!(
        kept(&none).is_empty(),
        "a pick with no choosing does nothing"
    );
}

#[test]
fn a_pick_that_is_not_a_candidate_leaves_the_choosing_open() {
    let got = then(
        desktop(2),
        vec![
            menu(1, MenuEntry::TileLeftOfScreen),
            pick(9),
            pick(1),
            pick(2),
        ],
    );
    assert_eq!(kept(&got), vec![pair(1, 2, 500)]);
}

#[test]
fn the_first_window_leaving_ends_the_choosing() {
    let closed = then(
        desktop(2),
        vec![menu(1, MenuEntry::TileLeftOfScreen), unmap(1)],
    );
    assert_eq!(
        notes(&closed).first(),
        Some(&ShellNote::SplitChoiceEnd(WindowId(1)))
    );
    let left = then(
        desktop(2),
        vec![
            menu(1, MenuEntry::TileLeftOfScreen),
            menu(1, MenuEntry::ExitFullScreen),
            pick(2),
        ],
    );
    assert!(kept(&left).is_empty(), "no Split View, no full screen left");
}

#[test]
fn leaving_full_screen_on_either_window_ends_the_split_view() {
    for leaver in [1, 2] {
        let before = run(desktop(2));
        let got = then(
            desktop(2),
            vec![
                menu(1, MenuEntry::TileLeftOfScreen),
                pick(2),
                menu(leaver, MenuEntry::ExitFullScreen),
            ],
        );
        assert!(kept(&got).is_empty(), "window {leaver}: the Space is gone");
        for n in [1, 2] {
            let id = WindowId(n);
            assert_eq!(
                got.model.windows()[&id].state,
                before.model.windows()[&id].state,
                "window {leaver} left: window {n} is back where it was"
            );
            assert_eq!(
                got.model.windows()[&id].place,
                before.model.windows()[&id].place,
                "window {leaver} left: window {n} is back on the desktop"
            );
        }
    }
}

#[test]
fn closing_one_window_leaves_the_other_full_screen() {
    let cases = [(1, 2), (2, 1)];
    for (closed, kept_window) in cases {
        let got = then(
            desktop(2),
            vec![menu(1, MenuEntry::TileLeftOfScreen), pick(2), unmap(closed)],
        );
        assert_eq!(
            kept(&got),
            vec![SpaceContent::FullScreen(WindowId(kept_window))],
            "window {closed} closed"
        );
        assert!(full(&got, kept_window));
    }
}

#[test]
fn the_divider_moves_within_its_limits() {
    let cases = [
        ("to a share", 650, 650),
        ("not past the far edge", 990, 800),
        ("not past the near edge", 5, 200),
    ];
    for (name, asked, want) in cases {
        let got = then(
            desktop(2),
            vec![
                menu(1, MenuEntry::TileLeftOfScreen),
                pick(2),
                shell(ShellRequest::SplitDivide {
                    window: WindowId(2),
                    share: Fraction(asked),
                }),
            ],
        );
        assert_eq!(kept(&got), vec![pair(1, 2, want)], "{name}");
    }
    let alone = then(
        desktop(2),
        vec![shell(ShellRequest::SplitDivide {
            window: WindowId(1),
            share: Fraction(300),
        })],
    );
    assert!(kept(&alone).is_empty());
}

fn command(command: DesktopCommand) -> Op {
    Op::Command(command)
}

#[test]
fn the_key_command_tiles_the_key_window_like_the_menu_entry() {
    let cases = [
        (
            "left",
            DesktopCommand::SplitTile(SplitSide::Left),
            SplitSide::Left,
        ),
        (
            "right",
            DesktopCommand::SplitTile(SplitSide::Right),
            SplitSide::Right,
        ),
    ];
    for (name, wanted, side) in cases {
        let got = then(desktop(2), vec![command(wanted)]);
        // The second window mapped is the key window, so it is the one tiled.
        assert!(full(&got, 2), "{name}");
        assert_eq!(notes(&got), vec![choose(2, side, &[1])], "{name}");
    }
    let nobody = then(
        vec![plug("HEADLESS-1")],
        vec![command(DesktopCommand::SplitTile(SplitSide::Left))],
    );
    assert!(kept(&nobody).is_empty());
}

#[test]
fn only_windows_of_the_desktop_the_first_came_from_can_be_picked() {
    let mail = proven("org.quire.Mail");
    let mut ops = desktop(2);
    ops.extend([
        Op::Event(Event::Desktop(DesktopRequest::CreateSpace {
            output: headless("HEADLESS-1").key,
        })),
        command(DesktopCommand::SwitchSpace(SpaceStep::Next)),
        map(3, &mail, TokenCheck::Missing),
        command(DesktopCommand::SwitchSpace(SpaceStep::Previous)),
    ]);
    let before = run(ops.clone());
    let windows = before.model.windows();
    assert_ne!(
        windows[&WindowId(2)].place,
        windows[&WindowId(3)].place,
        "window 3 is on another desktop"
    );
    let got = then(ops, vec![menu(1, MenuEntry::TileLeftOfScreen)]);
    assert_eq!(notes(&got), vec![choose(1, SplitSide::Left, &[2])]);
}

/// cas-wave "Real-run checks: Split View" (the picker shows on the chosen output only): the
/// candidates are the windows of the first window's own output, so the shell raises the picker
/// there and nowhere else; a window on the other output is not offered.
#[test]
fn only_windows_of_the_output_the_first_is_on_can_be_picked() {
    let mail = proven("org.quire.Mail");
    let made = run(vec![
        plug("HEADLESS-1"),
        plug("HEADLESS-2"),
        map(1, &mail, TokenCheck::Valid),
        map(2, &mail, TokenCheck::Missing),
        map(3, &mail, TokenCheck::Missing),
    ]);
    let model = made.model;
    let here = model
        .output_of(&model.windows()[&WindowId(1)])
        .expect("an output");
    let other = [headless("HEADLESS-1").key, headless("HEADLESS-2").key]
        .into_iter()
        .find(|key| *key != here)
        .expect("the other output");
    let target = model.spaces().row(&other)[0].id.clone();
    let moved = run_from(
        model,
        vec![Op::Event(Event::Desktop(DesktopRequest::Window {
            window: WindowId(3),
            ask: WindowAsk::ToSpace(target),
        }))],
    )
    .model;
    let got = run_from(moved, vec![menu(1, MenuEntry::TileLeftOfScreen)]);
    assert_eq!(notes(&got), vec![choose(1, SplitSide::Left, &[2])]);
}

#[test]
fn showing_another_space_while_choosing_cancels_the_choice() {
    let mut ops = desktop(2);
    ops.push(menu(1, MenuEntry::TileLeftOfScreen));
    let choosing = run(ops.clone());
    assert_eq!(notes(&choosing), vec![choose(1, SplitSide::Left, &[2])]);
    let away = then(
        ops.clone(),
        vec![command(DesktopCommand::SwitchSpace(SpaceStep::Previous))],
    );
    assert_eq!(notes(&away), vec![ShellNote::SplitChoiceEnd(WindowId(1))]);
    assert_eq!(kept(&away), vec![SpaceContent::FullScreen(WindowId(1))]);
    assert!(full(&away, 1));
    // The choice is over: a pick now changes nothing.
    let mut late = ops;
    late.extend([
        command(DesktopCommand::SwitchSpace(SpaceStep::Previous)),
        pick(2),
    ]);
    let late = run(late);
    assert_eq!(kept(&late), vec![SpaceContent::FullScreen(WindowId(1))]);
    assert!(!full(&late, 2));
}
