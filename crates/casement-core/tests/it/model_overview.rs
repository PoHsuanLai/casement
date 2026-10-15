//! Mission Control's life in the model: entering, open, leaving, closed, and what a pick does.
//! Every row runs with the invariants checked after each step.

use crate::support;

use casement_core::focus::TokenCheck;
use casement_core::input::{DesktopCommand, DeviceClass, Origin, SpaceStep};
use casement_core::lock::LockEvent;
use casement_core::overview::Overview;
use casement_core::shell::{ShellNote, ShellRequest};
use casement_core::time::TimerKey;
use casement_core::{Cause, ClientKey, Effect, Event, OverviewPick, ToplevelRequest, WindowId};
use casement_protocols::vocab::{OverviewMode, Toggle};
use support::driver::{Op, Run, last, map, plug, request, run, run_from, unmap};
use support::proven;

const COMMAND: Op = Op::Command(DesktopCommand::MissionControl);

fn settled() -> Op {
    Op::Event(Event::TimerFired(TimerKey::OverviewSettle))
}

fn asked(mode: OverviewMode, toggle: Toggle) -> Op {
    Op::Event(Event::Shell(ShellRequest::Overview { mode, toggle }))
}

/// Two windows of two apps on one output; window 2 is the key window.
fn two_windows() -> Run {
    run(vec![
        plug("HEADLESS-1"),
        map(1, &proven("org.example.Web"), TokenCheck::Valid),
        map(2, &proven("org.quire.Mail"), TokenCheck::Valid),
    ])
}

#[test]
fn the_overview_walks_entering_open_leaving_closed() {
    // name, ops after the windows are up, state after each
    let cases: Vec<(&str, Vec<Op>, Vec<Overview>)> = vec![
        (
            "the command opens and the next one leaves",
            vec![COMMAND, settled(), COMMAND, settled()],
            vec![
                Overview::Entering,
                Overview::Open,
                Overview::Leaving,
                Overview::Closed,
            ],
        ),
        (
            "leaving before it is open",
            vec![COMMAND, COMMAND, settled()],
            vec![Overview::Entering, Overview::Leaving, Overview::Closed],
        ),
        (
            "opening again while it leaves",
            vec![COMMAND, settled(), COMMAND, COMMAND, settled()],
            vec![
                Overview::Entering,
                Overview::Open,
                Overview::Leaving,
                Overview::Entering,
                Overview::Open,
            ],
        ),
        (
            "the shell opens and closes it",
            vec![
                asked(OverviewMode::MissionControl, Toggle::On),
                settled(),
                asked(OverviewMode::MissionControl, Toggle::On),
                asked(OverviewMode::MissionControl, Toggle::Off),
                settled(),
            ],
            vec![
                Overview::Entering,
                Overview::Open,
                Overview::Open,
                Overview::Leaving,
                Overview::Closed,
            ],
        ),
        (
            "the shell's other overviews change nothing",
            vec![
                asked(OverviewMode::AppExpose, Toggle::On),
                asked(OverviewMode::Desktop, Toggle::On),
                asked(OverviewMode::MissionControl, Toggle::Off),
            ],
            vec![Overview::Closed, Overview::Closed, Overview::Closed],
        ),
        (
            "a settle with nothing to settle",
            vec![settled()],
            vec![Overview::Closed],
        ),
    ];
    for (name, ops, want) in cases {
        let mut model = two_windows().model;
        let mut got = Vec::new();
        for op in ops {
            let step = run_from(model, vec![op]);
            got.push(step.model.overview());
            model = step.model;
        }
        assert_eq!(got, want, "{name}");
    }
}

#[test]
fn opening_relayouts_for_the_overview_and_asks_for_the_settle_timer() {
    let opened = run_from(two_windows().model, vec![COMMAND]);
    let effects = last(&opened);
    assert!(effects.contains(&Effect::Relayout(Cause::Overview)));
    assert!(effects.iter().any(|effect| matches!(
        effect,
        Effect::ArmTimer(due) if due.key == TimerKey::OverviewSettle
    )));
    assert!(effects.contains(&Effect::Shell(ShellNote::Overview(Overview::Entering))));
}

#[test]
fn with_no_output_there_is_nothing_to_open() {
    let after = run(vec![COMMAND]);
    assert_eq!(after.model.overview(), Overview::Closed);
    assert!(
        !last(&after)
            .iter()
            .any(|e| matches!(e, Effect::Relayout(_)))
    );
}

#[test]
fn a_pick_leaves_with_that_window_raised_and_keyed() {
    let open = run_from(two_windows().model, vec![COMMAND, settled()]);
    assert_eq!(open.model.focus().key, Some(WindowId(2)));
    let at = open.model.now().after(std::time::Duration::from_millis(1));
    let pick = OverviewPick {
        window: WindowId(1),
        origin: Origin::Physical(DeviceClass::Pointer),
    };
    let (model, effects) = open.model.close_overview(Some(pick), at);
    assert_eq!(model.check_invariants(), Ok(()));
    assert_eq!(model.overview(), Overview::Leaving);
    assert_eq!(model.focus().key, Some(WindowId(1)));
    assert!(effects.contains(&Effect::Raise(vec![WindowId(1)])));
    assert!(effects.contains(&Effect::Relayout(Cause::Overview)));
}

#[test]
fn leaving_without_a_pick_keeps_the_key_window() {
    let open = run_from(two_windows().model, vec![COMMAND, settled()]);
    let at = open.model.now().after(std::time::Duration::from_millis(1));
    let (model, _) = open.model.close_overview(None, at);
    assert_eq!(model.overview(), Overview::Leaving);
    assert_eq!(model.focus().key, Some(WindowId(2)));
}

#[test]
fn a_pick_while_closed_or_leaving_does_nothing() {
    let pick = OverviewPick {
        window: WindowId(1),
        origin: Origin::Physical(DeviceClass::Pointer),
    };
    let closed = two_windows().model;
    let at = closed.now().after(std::time::Duration::from_millis(1));
    let (same, effects) = closed.clone().close_overview(Some(pick), at);
    assert_eq!(same.overview(), Overview::Closed);
    assert_eq!(same.focus(), closed.focus());
    assert!(effects.is_empty());
    let leaving = run_from(closed, vec![COMMAND, COMMAND]).model;
    let (again, effects) = leaving.clone().close_overview(Some(pick), at);
    assert_eq!(again.overview(), Overview::Leaving);
    assert_eq!(again.focus(), leaving.focus());
    assert!(effects.is_empty());
}

/// Window 1 on the desktop, window 2 full screen on a Space of its own; the desktop is shown.
fn desktop_and_full_screen() -> Run {
    let mut ops = vec![
        plug("HEADLESS-1"),
        map(1, &proven("org.example.Web"), TokenCheck::Valid),
        map(2, &proven("org.quire.Mail"), TokenCheck::Valid),
        request(
            2,
            ToplevelRequest::FullScreen {
                toggle: Toggle::On,
                output: None,
            },
        ),
    ];
    ops.push(Op::Command(DesktopCommand::SwitchSpace(
        SpaceStep::Previous,
    )));
    run(ops)
}

/// What a close leaves behind: the state, and the three effects of a leave.
fn assert_left(after: &Run, name: &str) {
    assert_eq!(after.model.overview(), Overview::Leaving, "{name}");
    let effects = last(after);
    assert!(
        effects.contains(&Effect::Relayout(Cause::Overview)),
        "{name}"
    );
    assert!(
        effects.iter().any(|effect| matches!(
            effect,
            Effect::ArmTimer(due) if due.key == TimerKey::OverviewSettle
        )),
        "{name}"
    );
    assert!(
        effects.contains(&Effect::Shell(ShellNote::Overview(Overview::Leaving))),
        "{name}"
    );
}

#[test]
fn a_space_switch_closes_the_overview_with_no_pick() {
    for settle in [false, true] {
        let name = if settle { "open" } else { "entering" };
        let mut ops = vec![COMMAND];
        if settle {
            ops.push(settled());
        }
        let open = run_from(desktop_and_full_screen().model, ops);
        let after = run_from(
            open.model,
            vec![Op::Command(DesktopCommand::SwitchSpace(SpaceStep::Next))],
        );
        assert_left(&after, name);
    }
}

#[test]
fn a_switch_to_the_space_already_shown_leaves_the_overview_open() {
    let open = run_from(desktop_and_full_screen().model, vec![COMMAND, settled()]);
    let after = run_from(
        open.model,
        vec![Op::Command(DesktopCommand::SwitchSpace(SpaceStep::Number(
            1,
        )))],
    );
    assert_eq!(after.model.overview(), Overview::Open);
}

#[test]
fn the_session_locking_closes_the_overview_with_no_pick() {
    let open = run_from(two_windows().model, vec![COMMAND, settled()]);
    let key = open.model.focus().key;
    let after = run_from(
        open.model,
        vec![Op::Event(Event::Lock(LockEvent::Requested {
            client: ClientKey(7),
        }))],
    );
    assert_left(&after, "lock");
    assert_eq!(after.model.focus().key, key, "no window was picked");
}

#[test]
fn the_last_window_of_the_shown_space_closing_closes_the_overview() {
    let open = run_from(two_windows().model, vec![COMMAND, settled()]);
    let first = run_from(open.model, vec![unmap(2)]);
    assert_eq!(first.model.overview(), Overview::Open, "one window is left");
    let second = run_from(first.model, vec![unmap(1)]);
    assert_left(&second, "the last window");
}

#[test]
fn a_window_closing_on_a_space_that_is_not_shown_changes_nothing() {
    // Window 2 is full screen on another Space; the shown desktop keeps window 1.
    let open = run_from(desktop_and_full_screen().model, vec![COMMAND, settled()]);
    let after = run_from(open.model, vec![unmap(2)]);
    assert_eq!(after.model.overview(), Overview::Open);
}

#[test]
fn a_window_arriving_while_it_is_open_is_added_and_the_overview_stays() {
    let open = run_from(two_windows().model, vec![COMMAND, settled()]);
    let after = run_from(
        open.model,
        vec![map(3, &proven("org.example.Notes"), TokenCheck::Missing)],
    );
    assert_eq!(after.model.overview(), Overview::Open);
    assert!(after.model.windows().contains_key(&WindowId(3)));
    assert!(last(&after).contains(&Effect::Relayout(Cause::Open)));
    assert!(
        !last(&after)
            .iter()
            .any(|effect| matches!(effect, Effect::Shell(ShellNote::Overview(_)))),
        "the shell hears nothing: the overview did not change"
    );
}
