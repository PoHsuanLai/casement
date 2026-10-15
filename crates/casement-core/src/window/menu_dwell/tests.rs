use super::*;
use crate::ids::WindowId;

fn at(ms: u64) -> Mono {
    Mono(Duration::from_millis(ms))
}

fn zoom(window: u64) -> LightAt {
    LightAt {
        window: WindowId(window),
        light: Light::Zoom,
    }
}

fn close(window: u64) -> LightAt {
    LightAt {
        window: WindowId(window),
        light: Light::Close,
    }
}

const T: DwellTiming = DwellTiming::MAC;

fn waiting(window: u64, since: u64) -> Dwell {
    Dwell::Waiting {
        on: zoom(window),
        since: at(since),
    }
}

#[test]
fn the_dwell_machine_steps_by_table() {
    use DwellInput as I;
    use DwellOutcome as O;
    let cases: Vec<(&str, Dwell, DwellInput, Dwell, DwellOutcome)> = vec![
        (
            "arriving on green arms the timer",
            Dwell::Idle,
            I::Over(Some(zoom(1)), at(1000)),
            waiting(1, 1000),
            O::Arm(at(1500)),
        ),
        (
            "arriving on red does nothing",
            Dwell::Idle,
            I::Over(Some(close(1)), at(1000)),
            Dwell::Idle,
            O::Nothing,
        ),
        (
            "moving within the light keeps the first arrival",
            waiting(1, 1000),
            I::Over(Some(zoom(1)), at(1200)),
            waiting(1, 1000),
            O::Nothing,
        ),
        (
            "leaving cancels the timer",
            waiting(1, 1000),
            I::Over(None, at(1200)),
            Dwell::Idle,
            O::Cancel,
        ),
        (
            "sliding onto another light of the group cancels",
            waiting(1, 1000),
            I::Over(Some(close(1)), at(1200)),
            Dwell::Idle,
            O::Cancel,
        ),
        (
            "another window's green light restarts the wait",
            waiting(1, 1000),
            I::Over(Some(zoom(2)), at(1200)),
            waiting(2, 1200),
            O::Arm(at(1700)),
        ),
        (
            "a tick before the rest has passed is stale",
            waiting(1, 1000),
            I::Tick(at(1499)),
            waiting(1, 1000),
            O::Nothing,
        ),
        (
            "a tick at the rest shows the menu",
            waiting(1, 1000),
            I::Tick(at(1500)),
            Dwell::Shown(zoom(1)),
            O::Show(zoom(1)),
        ),
        (
            "a tick long after still shows it",
            waiting(1, 1000),
            I::Tick(at(9000)),
            Dwell::Shown(zoom(1)),
            O::Show(zoom(1)),
        ),
        (
            "a tick while idle is nothing",
            Dwell::Idle,
            I::Tick(at(9000)),
            Dwell::Idle,
            O::Nothing,
        ),
        (
            "resting on after the menu showed does not ask again",
            Dwell::Shown(zoom(1)),
            I::Over(Some(zoom(1)), at(3000)),
            Dwell::Shown(zoom(1)),
            O::Nothing,
        ),
        (
            "leaving after the menu showed rearms the intent",
            Dwell::Shown(zoom(1)),
            I::Over(None, at(3000)),
            Dwell::Idle,
            O::Nothing,
        ),
        (
            "a press while waiting cancels the timer",
            waiting(1, 1000),
            I::Pressed,
            Dwell::Idle,
            O::Cancel,
        ),
        (
            "a click spends the light until the pointer leaves",
            Dwell::Shown(zoom(1)),
            I::Pressed,
            Dwell::Spent(zoom(1)),
            O::Nothing,
        ),
        (
            "a spent light stays quiet under the pointer",
            Dwell::Spent(zoom(1)),
            I::Over(Some(zoom(1)), at(4000)),
            Dwell::Spent(zoom(1)),
            O::Nothing,
        ),
        (
            "a spent light is fresh once left",
            Dwell::Spent(zoom(1)),
            I::Over(None, at(4000)),
            Dwell::Idle,
            O::Nothing,
        ),
        (
            "a lost window forgets a wait",
            waiting(1, 1000),
            I::Lost,
            Dwell::Idle,
            O::Cancel,
        ),
        (
            "a lost window forgets a shown menu quietly",
            Dwell::Shown(zoom(1)),
            I::Lost,
            Dwell::Idle,
            O::Nothing,
        ),
    ];
    for (name, from, input, want, outcome) in cases {
        assert_eq!(from.step(T, input), (want, outcome), "{name}");
    }
}

#[test]
fn the_rest_is_data() {
    let slow = DwellTiming {
        rest: Duration::from_secs(2),
    };
    let (dwell, out) = Dwell::Idle.step(slow, DwellInput::Over(Some(zoom(1)), at(0)));
    assert_eq!(out, DwellOutcome::Arm(at(2000)));
    assert_eq!(
        dwell.step(slow, DwellInput::Tick(at(1500))).1,
        DwellOutcome::Nothing
    );
    assert_eq!(DwellTiming::default(), DwellTiming::MAC);
}
