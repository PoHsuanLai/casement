//! The multi-finger swipes: the router's and the vertical machine's tables, and the model
//! carrying a swipe through (the Spaces following the fingers, Mission Control opening).

use crate::support;

use casement_core::geometry::{LogicalPoint, LogicalRect, LogicalSize};
use casement_core::geometry::{SubPoint, SubPx};
use casement_core::input::gesture_route::{Axis, Drive, Fate, Route, route};
use casement_core::input::lift::{Intent, Lift, LiftIn, LiftOut, Verdict};
use casement_core::input::{
    DesktopCommand, DeviceClass, GestureEnd, GestureStep, InputEvent, InputKind, Origin,
};
use casement_core::overview::Overview;
use casement_core::settings::{CasementSettings, Enabled, GestureSettings, SwipeFingers};
use casement_core::space::{SpaceSeed, WorkspaceRequest};
use casement_core::time::TimerKey;
use casement_core::{Cause, DeviceId, Effect, Event, Model, PageMilli, SwipeOwner};
use ds_core::time::stamp::Stamp;
use support::{headless, ms};

fn by(dx: i32, dy: i32) -> GestureStep<SubPoint> {
    GestureStep::Update(SubPoint {
        x: SubPx(dx * 120),
        y: SubPx(dy * 120),
    })
}

const LIFT: GestureStep<SubPoint> = GestureStep::End(GestureEnd::Lifted);

fn begin(fingers: u8) -> GestureStep<SubPoint> {
    GestureStep::Begin { fingers }
}

fn settings(
    spaces: SwipeFingers,
    mission_control: SwipeFingers,
    natural: Enabled,
) -> GestureSettings {
    GestureSettings::new(spaces, mission_control, natural)
}

/// The route after each step of `steps`, and whose each was.
fn routed(steps: Vec<GestureStep<SubPoint>>, s: &GestureSettings) -> Vec<(Route, Fate)> {
    steps
        .into_iter()
        .scan(Route::Idle, |state, step| {
            let (next, fate) = route(*state, step, s);
            *state = next;
            Some((next, fate))
        })
        .collect()
}

#[test]
fn the_router_passes_other_finger_counts_and_takes_the_configured_ones() {
    let four = GestureSettings::defaults();
    let three_and_four = settings(SwipeFingers::Three, SwipeFingers::Four, Enabled::On);
    let off = settings(SwipeFingers::Off, SwipeFingers::Off, Enabled::On);
    // name, settings, fingers, whether the desktop takes it
    let cases = [
        ("two fingers are the client's", four, 2u8, false),
        ("four fingers by default", four, 4, true),
        ("three is not default", four, 3, false),
        ("three for the Spaces", three_and_four, 3, true),
        ("four for Mission Control", three_and_four, 4, true),
        ("five is nobody's", three_and_four, 5, false),
        ("off gives four to the client", off, 4, false),
    ];
    for (name, s, fingers, taken) in cases {
        let (_, fate) = route(Route::Idle, begin(fingers), &s);
        assert_eq!(matches!(fate, Fate::Take(_)), taken, "{name}");
    }
}

#[test]
fn the_router_locks_an_axis_then_drives_whole_pixels() {
    let s = GestureSettings::defaults();
    let steps = routed(
        vec![begin(4), by(3, 1), by(3, 1), by(10, 0), by(0, 0), LIFT],
        &s,
    );
    let fates: Vec<Fate> = steps.into_iter().map(|(_, fate)| fate).collect();
    assert_eq!(
        fates,
        vec![
            Fate::Take(vec![]),
            Fate::Take(vec![]),
            Fate::Take(vec![]),
            // 16 px right, 2 down: locked horizontal on the third step, with all 16 so far
            Fate::Take(vec![
                Drive::Began(Axis::Horizontal),
                Drive::Moved(Axis::Horizontal, 16)
            ]),
            Fate::Take(vec![]),
            Fate::Take(vec![Drive::Ended(Axis::Horizontal)]),
        ]
    );
}

#[test]
fn the_router_carries_the_sub_pixel_remainder() {
    let s = GestureSettings::defaults();
    let half = GestureStep::Update(SubPoint {
        x: SubPx(180),
        y: SubPx(0),
    });
    let (state, _) = route(Route::Idle, begin(4), &s);
    let (state, _) = route(state, by(10, 0), &s);
    let (state, first) = route(state, half, &s);
    let (_, second) = route(state, half, &s);
    assert_eq!(first, Fate::Take(vec![Drive::Moved(Axis::Horizontal, 1)]));
    assert_eq!(second, Fate::Take(vec![Drive::Moved(Axis::Horizontal, 2)]));
}

#[test]
fn natural_direction_off_reverses_the_spaces_but_not_mission_control() {
    let s = settings(SwipeFingers::Four, SwipeFingers::Four, Enabled::Off);
    let sideways = routed(vec![begin(4), by(20, 0)], &s);
    assert_eq!(
        sideways[1].1,
        Fate::Take(vec![
            Drive::Began(Axis::Horizontal),
            Drive::Moved(Axis::Horizontal, -20)
        ])
    );
    let upward = routed(vec![begin(4), by(0, -20)], &s);
    assert_eq!(
        upward[1].1,
        Fate::Take(vec![
            Drive::Began(Axis::Vertical),
            Drive::Moved(Axis::Vertical, -20)
        ])
    );
}

#[test]
fn an_axis_nobody_wants_at_that_count_is_swallowed_to_its_end() {
    let s = settings(SwipeFingers::Three, SwipeFingers::Four, Enabled::On);
    let steps = routed(vec![begin(3), by(0, -30), by(0, -30), LIFT], &s);
    assert_eq!(steps[1].0, Route::Dead);
    assert!(steps.iter().all(|(_, fate)| *fate == Fate::Take(vec![])));
    assert_eq!(steps[3].0, Route::Idle);
}

#[test]
fn a_cancelled_swipe_is_cancelled() {
    let s = GestureSettings::defaults();
    let steps = routed(
        vec![begin(4), by(30, 0), GestureStep::End(GestureEnd::Cancelled)],
        &s,
    );
    assert_eq!(
        steps[2].1,
        Fate::Take(vec![Drive::Cancelled(Axis::Horizontal)])
    );
}

/// The vertical machine over `(ms, dy)` moves, then a lift at `lift_at`.
fn lifted(intent: Intent, moves: &[(u64, i32)], lift_at: u64) -> (Vec<LiftOut>, Verdict) {
    let (lift, _) = Lift::Idle.step(LiftIn::Began(intent), Stamp(0));
    let (lift, outs) = moves
        .iter()
        .fold((lift, Vec::new()), |(lift, mut outs), (at, dy)| {
            let (next, more) = lift.step(LiftIn::Moved { dy: *dy }, Stamp(*at));
            outs.extend(more);
            (next, outs)
        });
    let (_, done) = lift.step(LiftIn::Ended, Stamp(lift_at));
    let verdict = match done.last() {
        Some(LiftOut::Done(verdict)) => *verdict,
        other => panic!("no verdict: {other:?}"),
    };
    (outs, verdict)
}

#[test]
fn the_lift_commits_by_distance_and_speed() {
    // name, intent, moves (ms, dy), lift time, verdict
    let cases = [
        (
            "past half, slow, opens",
            Intent::Open,
            vec![(200, -120), (400, -120)],
            1000,
            Verdict::Open,
        ),
        (
            "under half, slow, stays",
            Intent::Open,
            vec![(200, -60), (400, -60)],
            1000,
            Verdict::Stay,
        ),
        (
            "under half, flicked, opens",
            Intent::Open,
            vec![(10, -30), (20, -30), (30, -30)],
            40,
            Verdict::Open,
        ),
        (
            "past half, flicked back, stays",
            Intent::Open,
            vec![(500, -300), (550, 20), (600, 30)],
            620,
            Verdict::Stay,
        ),
        (
            "downward never opens",
            Intent::Open,
            vec![(100, 200)],
            1000,
            Verdict::Stay,
        ),
        (
            "past half down closes",
            Intent::Close,
            vec![(200, 120), (400, 120)],
            1000,
            Verdict::Close,
        ),
        (
            "a little down stays open",
            Intent::Close,
            vec![(300, 40)],
            1000,
            Verdict::Stay,
        ),
    ];
    for (name, intent, moves, at, want) in cases {
        let (_, verdict) = lifted(intent, &moves, at);
        assert_eq!(verdict, want, "{name}");
    }
}

#[test]
fn the_lift_reports_how_open_it_is_and_never_past_the_ends() {
    let (outs, _) = lifted(Intent::Open, &[(10, -100), (20, -900)], 500);
    let progress: Vec<i32> = outs
        .into_iter()
        .filter_map(|out| match out {
            LiftOut::Progress(PageMilli(p)) => Some(p),
            LiftOut::Done(_) => None,
        })
        .collect();
    assert_eq!(progress, vec![250, 1000]);
    let (outs, _) = lifted(Intent::Close, &[(10, 100)], 500);
    assert_eq!(outs, vec![LiftOut::Progress(PageMilli(750))]);
}

const T0: u64 = 1000;

/// One output with three Spaces, the first shown.
fn three_spaces(gestures: GestureSettings) -> Model {
    let settings = CasementSettings {
        gestures,
        ..CasementSettings::defaults()
    };
    let output = out();
    let made = |name: &str| {
        Event::Workspace(WorkspaceRequest::Create {
            output: output.clone(),
            name: name.to_owned(),
        })
    };
    let events = [
        Event::OutputPlugged(headless("HEADLESS-1")),
        made("two"),
        made("three"),
    ];
    let model = events.into_iter().zip(1..).fold(
        Model::new(settings, ms(0), SpaceSeed::from_entropy([9; 16])),
        |model, (event, at)| model.step(event, ms(at)).0,
    );
    let first = model
        .spaces()
        .row(&output)
        .first()
        .map(|space| space.id.clone())
        .expect("a first Space");
    model
        .step(Event::Workspace(WorkspaceRequest::Activate(first)), ms(10))
        .0
}

/// `model` after a swipe step at `at` ms, and its effects.
fn swipe(model: Model, at: u64, step: GestureStep<SubPoint>) -> (Model, Vec<Effect>) {
    let event = InputEvent {
        origin: Origin::Physical(DeviceClass::Touchpad),
        device: DeviceId(1),
        at: ms(at),
        kind: InputKind::Swipe(step),
    };
    model.step(Event::Input(event), ms(at))
}

/// `model` after all of `steps`, each at its own ms.
fn swiped(model: Model, steps: Vec<(u64, GestureStep<SubPoint>)>) -> (Model, Vec<Effect>) {
    steps
        .into_iter()
        .fold((model, Vec::new()), |(model, mut effects), (at, step)| {
            let (next, more) = swipe(model, at, step);
            effects.extend(more);
            (next, effects)
        })
}

fn shown_index(model: &Model) -> usize {
    let output = headless("HEADLESS-1").key;
    let shown = model.spaces().shown_on(&output).expect("shown").id.clone();
    model
        .spaces()
        .row(&output)
        .iter()
        .position(|space| space.id == shown)
        .expect("in the row")
}

fn out() -> casement_core::OutputKey {
    headless("HEADLESS-1").key
}

#[test]
fn a_far_swipe_left_follows_the_fingers_then_commits_to_the_next_space() {
    let model = three_spaces(GestureSettings::defaults());
    assert_eq!(shown_index(&model), 0);
    let (model, _) = swiped(
        model,
        vec![
            (T0, begin(4)),
            (T0 + 100, by(-60, 0)),
            (T0 + 200, by(-60, 0)),
            (T0 + 300, by(-60, 0)),
            (T0 + 400, by(-60, 0)),
        ],
    );
    assert_eq!(model.swipe_owner(), SwipeOwner::Desktop);
    let strip = model.space_strip(ms(T0 + 400)).expect("the row follows");
    assert_eq!(strip.position, PageMilli(600), "240 px of 400 is 0.6 page");
    assert_eq!(shown_index(&model), 0, "nothing is shown until the lift");
    let (model, effects) = swipe(model, T0 + 500, LIFT);
    assert_eq!(shown_index(&model), 1);
    assert!(
        effects
            .iter()
            .any(|e| matches!(e, Effect::ArmTimer(due) if due.key == TimerKey::SpaceFinish(out())))
    );
    let mid = model.space_strip(ms(T0 + 560)).expect("sliding");
    assert!(mid.position > PageMilli(600) && mid.position < PageMilli(1000));
    let (model, _) = model.step(
        Event::TimerFired(TimerKey::SpaceFinish(out())),
        ms(T0 + 900),
    );
    assert_eq!(model.space_strip(ms(T0 + 900)), None, "at rest");
}

#[test]
fn a_short_slow_swipe_snaps_back() {
    let model = three_spaces(GestureSettings::defaults());
    let (model, _) = swiped(
        model,
        vec![
            (T0, begin(4)),
            (T0 + 300, by(-50, 0)),
            (T0 + 600, by(-50, 0)),
            (T0 + 900, LIFT),
        ],
    );
    assert_eq!(shown_index(&model), 0);
}

#[test]
fn the_row_rubber_bands_past_the_first_space() {
    let model = three_spaces(GestureSettings::defaults());
    let (model, _) = swiped(model, vec![(T0, begin(4)), (T0 + 10, by(400, 0))]);
    let strip = model.space_strip(ms(T0 + 10)).expect("following");
    assert_eq!(strip.position, PageMilli(-250), "a quarter page at most");
    let (model, _) = swipe(model, T0 + 500, LIFT);
    assert_eq!(shown_index(&model), 0, "it comes back");
}

#[test]
fn natural_direction_off_moves_the_spaces_against_the_fingers() {
    let model = three_spaces(settings(
        SwipeFingers::Four,
        SwipeFingers::Four,
        Enabled::Off,
    ));
    let (model, _) = swiped(
        model,
        vec![
            (T0, begin(4)),
            (T0 + 20, by(80, 0)),
            (T0 + 40, by(80, 0)),
            (T0 + 60, by(80, 0)),
            (T0 + 70, LIFT),
        ],
    );
    assert_eq!(shown_index(&model), 1);
}

#[test]
fn a_swipe_of_another_finger_count_stays_the_clients() {
    let model = three_spaces(GestureSettings::defaults());
    let (model, _) = swiped(model, vec![(T0, begin(2)), (T0 + 10, by(-300, 0))]);
    assert_eq!(model.swipe_owner(), SwipeOwner::Client);
    assert_eq!(model.space_strip(ms(T0 + 10)), None);
}

#[test]
fn a_swipe_up_opens_mission_control_and_one_down_closes_it() {
    let model = three_spaces(GestureSettings::defaults());
    let (model, _) = swiped(
        model,
        vec![
            (T0, begin(4)),
            (T0 + 10, by(0, -150)),
            (T0 + 20, by(0, -50)),
        ],
    );
    assert_eq!(model.overview_progress(), Some(PageMilli(500)));
    assert_eq!(model.overview(), Overview::Closed);
    let (model, _) = swipe(model, T0 + 300, LIFT);
    assert_eq!(model.overview_progress(), None);
    assert_eq!(model.overview(), Overview::Entering);
    let (model, _) = model.step(Event::TimerFired(TimerKey::OverviewSettle), ms(T0 + 800));
    assert_eq!(model.overview(), Overview::Open);
    let (model, _) = swiped(
        model,
        vec![
            (T0 + 1000, begin(4)),
            (T0 + 1010, by(0, 120)),
            (T0 + 1020, by(0, 120)),
            (T0 + 1500, LIFT),
        ],
    );
    assert_eq!(model.overview(), Overview::Leaving);
}

/// A short swipe up drew the windows part-way: they fly back from there (Mission Control shows
/// as leaving, with a laid-out-again scene, until they land) and it ends closed.
#[test]
fn a_short_swipe_up_snaps_back_and_leaves_mission_control_closed() {
    let model = three_spaces(GestureSettings::defaults());
    let (model, _) = swiped(model, vec![(T0, begin(4)), (T0 + 300, by(0, -60))]);
    assert_eq!(model.overview_progress(), Some(PageMilli(150)));
    let (model, effects) = swipe(model, T0 + 900, LIFT);
    assert_eq!(model.overview(), Overview::Leaving);
    assert!(effects.contains(&Effect::Relayout(Cause::Overview)));
    assert!(
        effects
            .iter()
            .any(|e| matches!(e, Effect::ArmTimer(due) if due.key == TimerKey::OverviewSettle))
    );
    let (model, _) = model.step(Event::TimerFired(TimerKey::OverviewSettle), ms(T0 + 1300));
    assert_eq!(model.overview(), Overview::Closed);
}

/// Nothing was drawn moved when the fingers never left the ends: no snap-back, no relayout.
#[test]
fn a_swipe_that_never_moved_the_windows_changes_nothing() {
    let model = three_spaces(GestureSettings::defaults());
    let (model, _) = swiped(model, vec![(T0, begin(4)), (T0 + 10, by(0, 12))]);
    assert_eq!(model.overview_progress(), Some(PageMilli(0)));
    let (model, effects) = swipe(model, T0 + 50, GestureStep::End(GestureEnd::Cancelled));
    assert_eq!(model.overview(), Overview::Closed);
    assert!(!effects.contains(&Effect::Relayout(Cause::Overview)));
}

/// A swipe down that stops short of closing returns the windows to their slots.
#[test]
fn a_short_swipe_down_returns_the_windows_to_their_slots() {
    let model = three_spaces(GestureSettings::defaults());
    let (model, _) = model.command(DesktopCommand::MissionControl, ms(T0));
    let (model, _) = model.step(Event::TimerFired(TimerKey::OverviewSettle), ms(T0 + 400));
    assert_eq!(model.overview(), Overview::Open);
    let (model, _) = swiped(model, vec![(T0 + 1000, begin(4)), (T0 + 1300, by(0, 60))]);
    let (model, effects) = swipe(model, T0 + 1900, LIFT);
    assert_eq!(model.overview(), Overview::Open, "still up");
    assert!(effects.contains(&Effect::Relayout(Cause::Overview)));
}

#[test]
fn mission_control_opened_by_a_key_is_not_opened_again_by_a_swipe() {
    let model = three_spaces(GestureSettings::defaults());
    let (model, _) = model.command(DesktopCommand::MissionControl, ms(T0));
    assert_eq!(model.overview(), Overview::Entering);
    let (model, _) = swiped(
        model,
        vec![(T0 + 10, begin(4)), (T0 + 20, by(0, -300)), (T0 + 30, LIFT)],
    );
    assert_eq!(model.overview(), Overview::Entering);
}

/// The row's first Space starts where the position says, a page apart per thousand.
#[test]
fn the_row_starts_where_the_position_says() {
    let usable = LogicalRect::new(
        LogicalPoint { x: 10, y: 24 },
        LogicalSize { w: 1000, h: 700 },
    );
    let model = three_spaces(GestureSettings::defaults());
    let (model, _) = swiped(model, vec![(T0, begin(4)), (T0 + 10, by(-100, 0))]);
    let strip = model.space_strip(ms(T0 + 10)).expect("following");
    assert_eq!(strip.position, PageMilli(250));
    // 10 px in, a quarter of 1000 px back: 1200 - 30 000 sub-pixels.
    assert_eq!(strip.first_x(usable).0, 1_200 - 30_000);
}
