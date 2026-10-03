//! The virtual clock as a calloop source (P0-B-loop, A1, SP3): stops in order, nothing before
//! the clock moves, never backwards. Driven by a bare event loop, without a `State`.

use casement::time::{SharedClock, VirtualTimerSource};
use casement_core::output::{EdidFacts, EdidSerial};
use casement_core::time::{Due, Millihertz, Mono, Stop, TimerKey, VblankClock};
use casement_core::{ClientKey, DeviceId, OutputKey, OutputName};
use smithay::reexports::calloop::EventLoop;
use std::time::Duration;

fn ms(n: u64) -> Mono {
    Mono(Duration::from_millis(n))
}

fn output(name: &str) -> OutputKey {
    let edid = EdidFacts {
        make: String::new(),
        model: String::new(),
        serial: EdidSerial::Unknown,
    };
    OutputKey::of(&edid, &OutputName::parse(name).expect("name"))
}

/// A loop whose data collects every stop the source hands out.
fn looped(clock: &SharedClock) -> EventLoop<'static, Vec<Stop>> {
    let event_loop: EventLoop<'static, Vec<Stop>> = EventLoop::try_new().expect("event loop");
    event_loop
        .handle()
        .insert_source(
            VirtualTimerSource::new(clock.clone()),
            |stop, _, seen: &mut Vec<Stop>| seen.push(stop),
        )
        .expect("source");
    event_loop
}

#[test]
#[ignore = "P0-B-loop"]
fn due_stops_come_out_in_order_and_only_when_the_clock_moves() {
    let clock = SharedClock::new(ms(0));
    clock.update(|c| {
        c.schedule(Due {
            at: ms(250),
            key: TimerKey::Ping(ClientKey(2)),
        })
        .schedule(Due {
            at: ms(250),
            key: TimerKey::Ping(ClientKey(1)),
        })
        .schedule(Due {
            at: ms(100),
            key: TimerKey::KeyRepeat(DeviceId(1)),
        })
    });
    let mut event_loop = looped(&clock);
    let mut seen = Vec::new();
    event_loop
        .dispatch(Some(Duration::ZERO), &mut seen)
        .expect("dispatch");
    assert!(seen.is_empty(), "nothing is due before the clock moves");
    clock.advance_to(ms(300));
    event_loop
        .dispatch(Some(Duration::ZERO), &mut seen)
        .expect("dispatch");
    assert_eq!(
        seen,
        vec![
            Stop::Timer(TimerKey::KeyRepeat(DeviceId(1))),
            Stop::Timer(TimerKey::Ping(ClientKey(2))),
            Stop::Timer(TimerKey::Ping(ClientKey(1))),
        ],
        "earliest first; same-instant timers in arming order"
    );
}

#[test]
#[ignore = "P0-B-loop"]
fn sixty_vblanks_in_a_virtual_second() {
    let clock = SharedClock::new(ms(0));
    clock.update(|c| {
        c.with_output(
            output("HEADLESS-1"),
            VblankClock::new(Millihertz(60_000), ms(0)),
        )
    });
    let mut vblanks = 0;
    let mut now = ms(0);
    while now < ms(1000) {
        let (at, stop) = clock.next_stop(ms(1000));
        clock.advance_to(at);
        now = at;
        if matches!(stop, Stop::Vblank(..)) {
            vblanks += 1;
        }
    }
    assert_eq!(vblanks, 60);
}

#[test]
#[ignore = "P0-B-loop"]
fn the_clock_never_moves_backwards() {
    let clock = SharedClock::new(ms(500));
    clock.advance_to(ms(100));
    assert_eq!(clock.now(), ms(500));
}

#[test]
#[ignore = "P0-B-loop"]
fn the_binary_answers_help() {
    let status = std::process::Command::new(env!("CARGO_BIN_EXE_casement"))
        .arg("--help")
        .env_clear()
        .status()
        .expect("the casement binary runs");
    assert!(status.success());
}
