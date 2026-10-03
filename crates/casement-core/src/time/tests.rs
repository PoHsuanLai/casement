//! The clock's conformance cases (ds-core's `time/tests.rs`, restated for a value clock) and
//! the stop-order properties 11 §2.4 and SP3 rely on.

use super::*;
use crate::ids::{DeviceId, OutputKey, OutputName};
use crate::output::{EdidFacts, EdidSerial};
use ds_core::time::stamp::Stamp;
use std::time::Duration;

fn ms(n: u64) -> Mono {
    Mono(Duration::from_millis(n))
}

fn repeat(device: u32) -> TimerKey {
    TimerKey::KeyRepeat(DeviceId(device))
}

fn output(name: &str) -> OutputKey {
    let edid = EdidFacts {
        make: String::new(),
        model: String::new(),
        serial: EdidSerial::Unknown,
    };
    OutputKey::of(&edid, &OutputName::parse(name).expect("valid name"))
}

#[test]
fn a_timer_is_due_exactly_at_its_instant_and_fires_once() {
    let clock = VirtualClock::new(ms(0)).schedule(Due {
        at: ms(100),
        key: repeat(1),
    });
    assert_eq!(clock.next_due(), Some(ms(100)));
    let (clock, fired) = clock.advance_to(ms(99)).fire_due();
    assert_eq!(fired, Vec::<TimerKey>::new());
    let (clock, fired) = clock.advance_to(ms(100)).fire_due();
    assert_eq!(fired, vec![repeat(1)]);
    assert_eq!(clock.timers().next(), None);
    assert_eq!(clock.fire_due().1, Vec::<TimerKey>::new());
}

#[test]
fn a_cancelled_timer_leaves_the_book_and_time_never_runs_backwards() {
    let clock = VirtualClock::new(ms(0))
        .advance_to(ms(50))
        .schedule(Due {
            at: ms(60),
            key: repeat(1),
        })
        .schedule(Due {
            at: ms(80),
            key: repeat(2),
        });
    assert_eq!(clock.next_due(), Some(ms(60)));
    let clock = clock
        .clone()
        .with_timers(clock.timers().clone().cancel(&repeat(1)));
    assert_eq!(clock.next_due(), Some(ms(80)));
    let clock = clock
        .clone()
        .with_timers(clock.timers().clone().cancel(&repeat(2)));
    assert_eq!(clock.next_due(), None);
    assert_eq!(
        clock.advance_to(ms(20)).elapsed(),
        Duration::from_millis(50)
    );
}

#[test]
fn same_instant_timers_fire_in_arming_order_not_key_order() {
    let book = TimerBook::empty()
        .arm(Due {
            at: ms(250),
            key: repeat(2),
        })
        .arm(Due {
            at: ms(250),
            key: repeat(1),
        })
        .arm(Due {
            at: ms(100),
            key: TimerKey::SwitcherDelay,
        });
    let (book, fired) = book.fire_until(ms(250));
    assert_eq!(fired, vec![TimerKey::SwitcherDelay, repeat(2), repeat(1)]);
    assert_eq!(book.next(), None);
}

#[test]
fn arming_a_key_again_moves_it() {
    let book = TimerBook::empty()
        .arm(Due {
            at: ms(10),
            key: repeat(1),
        })
        .arm(Due {
            at: ms(30),
            key: repeat(1),
        });
    assert_eq!(
        book.next(),
        Some(Due {
            at: ms(30),
            key: repeat(1)
        })
    );
    assert_eq!(book.fire_until(ms(10)).1, Vec::<TimerKey>::new());
}

/// Every stop from now to `limit`, arriving at each as a driver would, with each advance
/// limited to `chunk`.
fn stops(clock: VirtualClock, limit: Mono, chunk: Duration) -> Vec<(Mono, Stop)> {
    let mut clock = clock;
    let mut seen = Vec::new();
    while clock.now() < limit {
        let step_limit = clock.now().after(chunk).min(limit);
        let (at, _) = clock.next_stop(step_limit);
        let (next, arrival) = clock.arrive(at);
        clock = next;
        seen.extend(arrival.timers.into_iter().map(|key| (at, Stop::Timer(key))));
        seen.extend(
            arrival
                .vblanks
                .into_iter()
                .map(|(key, seq)| (at, Stop::Vblank(key, seq))),
        );
    }
    seen
}

fn two_outputs_and_a_timer() -> VirtualClock {
    VirtualClock::new(ms(0))
        .with_output(
            output("HEADLESS-1"),
            VblankClock::new(Millihertz(60_000), ms(0)),
        )
        .with_output(
            output("HEADLESS-2"),
            VblankClock::new(Millihertz(30_000), ms(5)),
        )
        .schedule(Due {
            at: ms(20),
            key: TimerKey::SwitcherDelay,
        })
}

#[test]
fn stops_come_in_instant_order_with_timers_first() {
    let got = stops(two_outputs_and_a_timer(), ms(40), Duration::from_secs(1));
    let at = |ns: u64| Mono(Duration::from_nanos(ns));
    let want = vec![
        (
            at(5_000_000),
            Stop::Vblank(output("HEADLESS-2"), VblankSeq(0)),
        ),
        (
            at(16_666_666),
            Stop::Vblank(output("HEADLESS-1"), VblankSeq(1)),
        ),
        (ms(20), Stop::Timer(TimerKey::SwitcherDelay)),
        (
            at(33_333_333),
            Stop::Vblank(output("HEADLESS-1"), VblankSeq(2)),
        ),
        (
            at(38_333_333),
            Stop::Vblank(output("HEADLESS-2"), VblankSeq(1)),
        ),
    ];
    assert_eq!(got, want);
}

#[test]
fn how_an_advance_is_chopped_changes_no_stop() {
    let whole = stops(two_outputs_and_a_timer(), ms(1000), Duration::from_secs(10));
    let fine = stops(
        two_outputs_and_a_timer(),
        ms(1000),
        Duration::from_millis(1),
    );
    let odd = stops(
        two_outputs_and_a_timer(),
        ms(1000),
        Duration::from_micros(7_919),
    );
    assert_eq!(whole.len(), 1 + 60 + 30);
    assert_eq!(whole, fine);
    assert_eq!(whole, odd);
}

#[test]
fn removing_an_output_stops_its_vblanks() {
    let clock = two_outputs_and_a_timer()
        .without_output(&output("HEADLESS-2"))
        .without_output(&output("HEADLESS-1"));
    assert_eq!(
        clock.next_stop(ms(1000)),
        (ms(20), Stop::Timer(TimerKey::SwitcherDelay))
    );
    assert_eq!(clock.advance_to(ms(30)).next_stop(ms(25)).0, ms(30));
}

#[test]
fn vblanks_do_not_drift_over_ten_minutes() {
    let clock = VblankClock::new(Millihertz(60_000), ms(0));
    let (at, seq) = clock.next_after(Mono(Duration::from_nanos(599_983_333_334)));
    assert_eq!((at, seq), (ms(600_000), VblankSeq(36_000)));
    assert_eq!(clock.next_after(ms(600_000)).1, VblankSeq(36_001));
    let late = VblankClock::new(Millihertz(60_000), ms(10));
    assert_eq!(late.next_after(ms(3)), (ms(10), VblankSeq(0)));
    assert_eq!(late.next_after(ms(10)).1, VblankSeq(1));
}

#[test]
fn periods_and_stamps() {
    assert_eq!(
        Millihertz(60_000).period(),
        Duration::from_nanos(16_666_666)
    );
    assert_eq!(
        Millihertz(144_000).period(),
        Duration::from_nanos(6_944_444)
    );
    assert_eq!(Millihertz(0).period(), Duration::from_secs(1000));
    assert_eq!(Mono(Duration::from_micros(1_999_999)).stamp(), Stamp(1999));
    assert_eq!(ms(5).since(ms(9)), Duration::ZERO);
    assert_eq!(ms(5).after(Duration::from_millis(4)), ms(9));
}

#[test]
fn mono_round_trips_through_json() {
    let mono = Mono(Duration::new(3, 250));
    let text = serde_json::to_string(&mono).expect("serialise");
    assert_eq!(text, r#"{"secs":3,"nanos":250}"#);
    assert_eq!(serde_json::from_str::<Mono>(&text).expect("parse"), mono);
    let rate = serde_json::to_string(&Millihertz(59_940)).expect("serialise");
    assert_eq!(
        serde_json::from_str::<Millihertz>(&rate).expect("parse"),
        Millihertz(59_940)
    );
}
