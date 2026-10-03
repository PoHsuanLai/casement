//! The stop-order table: what a driver sees, instant by instant, from a clock with timers and
//! outputs. Each row names its setup and the exact sequence of `(nanoseconds, what)`.

use super::*;
use crate::ids::{DeviceId, OutputKey, OutputName};
use crate::output::{EdidFacts, EdidSerial};
use std::time::Duration;

/// An output's name, its refresh in millihertz and its phase in nanoseconds.
type OutputRow = (&'static str, u32, u64);

/// A timer's instant in nanoseconds and the keyboard it repeats for.
type TimerRow = (u64, u32);

/// One row: a name, the timers armed in order, the outputs, the limit in nanoseconds and the
/// expected `(instant, label)` sequence. Labels are `T<device>` and `V<output>#<seq>`.
struct Row {
    name: &'static str,
    timers: &'static [TimerRow],
    outputs: &'static [OutputRow],
    limit: u64,
    want: &'static [(u64, &'static str)],
}

fn at(ns: u64) -> Mono {
    Mono(Duration::from_nanos(ns))
}

fn key(name: &str) -> OutputKey {
    let edid = EdidFacts {
        make: String::new(),
        model: String::new(),
        serial: EdidSerial::Unknown,
    };
    OutputKey::of(&edid, &OutputName::parse(name).expect("valid name"))
}

fn clock_of(row: &Row) -> VirtualClock {
    let with_outputs =
        row.outputs
            .iter()
            .fold(VirtualClock::new(at(0)), |clock, (name, rate, phase)| {
                clock.with_output(key(name), VblankClock::new(Millihertz(*rate), at(*phase)))
            });
    row.timers
        .iter()
        .fold(with_outputs, |clock, (when, device)| {
            clock.schedule(Due {
                at: at(*when),
                key: TimerKey::KeyRepeat(DeviceId(*device)),
            })
        })
}

fn label_timer(key: &TimerKey) -> String {
    match key {
        TimerKey::KeyRepeat(DeviceId(device)) => format!("T{device}"),
        other => format!("{other:?}"),
    }
}

/// Every instant from the clock's start to `limit`, as a driver sees it, checking that
/// `next_stop` always names the first thing `arrive` then reports.
fn drive(row: &Row) -> Vec<(u64, String)> {
    let limit = at(row.limit);
    let mut clock = clock_of(row);
    let mut seen = Vec::new();
    while clock.now() < limit {
        let (when, stop) = clock.next_stop(limit);
        let (next, arrival) = clock.arrive(when);
        clock = next;
        let ns = u64::try_from(arrival.at.0.as_nanos()).expect("fits");
        let labels: Vec<String> = arrival
            .timers
            .iter()
            .map(label_timer)
            .chain(arrival.vblanks.iter().map(|(key, seq)| {
                format!(
                    "V{}#{}",
                    key.as_str().trim_start_matches("connector:"),
                    seq.0
                )
            }))
            .collect();
        match &stop {
            Stop::End => assert!(
                labels.is_empty(),
                "{}: an End stop arrives at nothing",
                row.name
            ),
            Stop::Timer(first) => {
                assert_eq!(labels.first(), Some(&label_timer(first)), "{}", row.name)
            }
            Stop::Vblank(first, seq) => assert_eq!(
                labels.first(),
                Some(&format!(
                    "V{}#{}",
                    first.as_str().trim_start_matches("connector:"),
                    seq.0
                )),
                "{}",
                row.name
            ),
        }
        seen.extend(labels.into_iter().map(|label| (ns, label)));
    }
    seen
}

#[test]
fn the_stop_order_table() {
    const ROWS: &[Row] = &[
        Row {
            name: "nothing armed",
            timers: &[],
            outputs: &[],
            limit: 50,
            want: &[],
        },
        Row {
            name: "timers by instant",
            timers: &[(30, 1), (10, 2), (20, 3)],
            outputs: &[],
            limit: 50,
            want: &[(10, "T2"), (20, "T3"), (30, "T1")],
        },
        Row {
            name: "same instant: arming order, not device order",
            timers: &[(10, 9), (10, 1), (10, 5)],
            outputs: &[],
            limit: 50,
            want: &[(10, "T9"), (10, "T1"), (10, "T5")],
        },
        Row {
            name: "a timer at the limit stops; one past it does not",
            timers: &[(50, 1), (51, 2)],
            outputs: &[],
            limit: 50,
            want: &[(50, "T1")],
        },
        Row {
            name: "vblanks by instant, phase first",
            timers: &[],
            outputs: &[("HEADLESS-1", 60_000, 5_000_000)],
            limit: 40_000_000,
            want: &[
                (5_000_000, "VHEADLESS-1#0"),
                (21_666_666, "VHEADLESS-1#1"),
                (38_333_333, "VHEADLESS-1#2"),
            ],
        },
        Row {
            name: "a timer before the vblank at its own instant",
            timers: &[(16_666_666, 1)],
            outputs: &[("HEADLESS-1", 60_000, 0)],
            limit: 20_000_000,
            want: &[(16_666_666, "T1"), (16_666_666, "VHEADLESS-1#1")],
        },
        Row {
            name: "two outputs at one instant: by output key, whatever the add order",
            timers: &[],
            outputs: &[("HEADLESS-2", 30_000, 0), ("HEADLESS-1", 60_000, 0)],
            limit: 34_000_000,
            want: &[
                (16_666_666, "VHEADLESS-1#1"),
                (33_333_333, "VHEADLESS-1#2"),
                (33_333_333, "VHEADLESS-2#1"),
            ],
        },
        Row {
            name: "timers in arming order, then vblanks by key, all at one instant",
            timers: &[(33_333_333, 2), (33_333_333, 1)],
            outputs: &[("HEADLESS-2", 30_000, 0), ("HEADLESS-1", 60_000, 0)],
            limit: 34_000_000,
            want: &[
                (16_666_666, "VHEADLESS-1#1"),
                (33_333_333, "T2"),
                (33_333_333, "T1"),
                (33_333_333, "VHEADLESS-1#2"),
                (33_333_333, "VHEADLESS-2#1"),
            ],
        },
    ];
    for row in ROWS {
        let got = drive(row);
        let want: Vec<(u64, String)> = row
            .want
            .iter()
            .map(|(ns, label)| (*ns, (*label).to_owned()))
            .collect();
        assert_eq!(got, want, "{}", row.name);
    }
}

#[test]
fn an_overdue_timer_stops_at_now_and_time_never_goes_back() {
    let clock = VirtualClock::new(at(100))
        .schedule(Due {
            at: at(40),
            key: TimerKey::SwitcherDelay,
        })
        .with_output(key("DP-1"), VblankClock::new(Millihertz(60_000), at(0)));
    let (when, stop) = clock.next_stop(at(1_000));
    assert_eq!(
        (when, stop),
        (at(100), Stop::Timer(TimerKey::SwitcherDelay))
    );
    let (clock, arrival) = clock.arrive(when);
    assert_eq!(arrival.timers, vec![TimerKey::SwitcherDelay]);
    assert_eq!(
        arrival.vblanks,
        Vec::new(),
        "nothing moved, so no vblank is reported"
    );
    assert_eq!(clock.now(), at(100));
    let (back, arrival) = clock.arrive(at(10));
    assert_eq!((back.now(), arrival.at), (at(100), at(100)));
}

#[test]
fn a_stop_past_the_limit_is_the_limit() {
    let clock = VirtualClock::new(at(0)).schedule(Due {
        at: at(500),
        key: TimerKey::SwitcherDelay,
    });
    assert_eq!(clock.next_stop(at(100)), (at(100), Stop::End));
}

#[test]
fn books_with_the_same_timers_are_equal_whatever_armed_them() {
    let due = |ns: u64, device: u32| Due {
        at: at(ns),
        key: TimerKey::KeyRepeat(DeviceId(device)),
    };
    let direct = TimerBook::empty().arm(due(10, 1)).arm(due(20, 2));
    let detour = TimerBook::empty()
        .arm(due(5, 1))
        .arm(due(20, 2))
        .arm(due(10, 1));
    assert_eq!(direct, detour, "history before the last arming is not data");
    let then_cancelled = TimerBook::empty()
        .arm(due(7, 3))
        .cancel(&TimerKey::KeyRepeat(DeviceId(3)));
    assert_eq!(then_cancelled, TimerBook::empty());
    let first_then_second = TimerBook::empty().arm(due(10, 1)).arm(due(10, 2));
    let second_then_first = TimerBook::empty().arm(due(10, 2)).arm(due(10, 1));
    assert_ne!(
        first_then_second, second_then_first,
        "arming order at one instant is part of the book"
    );
}
