//! The virtual clock as a calloop source (P0-B-loop, A1, SP3): stops in order, nothing before
//! the clock moves, never backwards. Driven by a bare event loop, without a `State`.

use casement::time::{SharedClock, VirtualTimerSource};
use casement_core::output::{EdidFacts, EdidSerial};
use casement_core::time::{Due, Millihertz, Mono, Stop, TimerKey, VblankClock, VblankSeq};
use casement_core::{ClientKey, DeviceId, OutputKey, OutputName};
use smithay::reexports::calloop::EventLoop;
use std::cell::RefCell;
use std::rc::Rc;
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

/// A 100 Hz output's clock (a vblank every 10 ms, so instants are whole milliseconds).
fn hundred_hertz(name: &str) -> (OutputKey, VblankClock) {
    (output(name), VblankClock::new(Millihertz(100_000), ms(0)))
}

#[test]
fn a_timer_and_vblanks_that_share_an_instant_arrive_together_timers_first() {
    // A32: a driver steps with next_stop then arrive; a timer, then the vblanks by output key.
    let clock = SharedClock::new(ms(0));
    clock.update(|c| {
        c.schedule(Due {
            at: ms(10),
            key: TimerKey::SwitcherDelay,
        })
    });
    for name in ["HEADLESS-2", "HEADLESS-1"] {
        let (key, vblanks) = hundred_hertz(name);
        clock.plug_output(key, vblanks);
    }
    let mut event_loop = looped(&clock);
    let mut seen = Vec::new();
    let (at, _) = clock.next_stop(ms(1000));
    assert_eq!(at, ms(10));
    clock.advance_to(at);
    event_loop
        .dispatch(Some(Duration::ZERO), &mut seen)
        .expect("dispatch");
    assert_eq!(
        seen,
        vec![
            Stop::Timer(TimerKey::SwitcherDelay),
            Stop::Vblank(output("HEADLESS-1"), VblankSeq(1)),
            Stop::Vblank(output("HEADLESS-2"), VblankSeq(1)),
        ]
    );
    assert_eq!(clock.take_fired(), vec![TimerKey::SwitcherDelay]);
    assert_eq!(clock.delivered(), 3);
}

#[test]
fn running_to_a_goal_hands_stops_out_one_instant_at_a_time_so_arming_shows() {
    let clock = SharedClock::new(ms(0));
    let (key, vblanks) = hundred_hertz("HEADLESS-1");
    clock.plug_output(key, vblanks);
    let seen: Rc<RefCell<Vec<(Mono, Stop)>>> = Rc::default();
    let mut event_loop: EventLoop<'static, ()> = EventLoop::try_new().expect("event loop");
    let (log, view, arm) = (seen.clone(), clock.clone(), clock.clone());
    event_loop
        .handle()
        .insert_source(
            VirtualTimerSource::new(clock.clone()),
            move |stop, _, ()| {
                // The first vblank arms a timer 5 ms later: it must come before the next vblank.
                if matches!(stop, Stop::Vblank(..)) && log.borrow().is_empty() {
                    arm.update(|c| {
                        c.schedule(Due {
                            at: ms(15),
                            key: TimerKey::SwitcherDelay,
                        })
                    });
                }
                log.borrow_mut().push((view.now(), stop));
            },
        )
        .expect("source");
    clock.run_to(ms(35));
    for _ in 0..20 {
        event_loop
            .dispatch(Some(Duration::ZERO), &mut ())
            .expect("dispatch");
    }
    let seen = seen.borrow();
    let at: Vec<Mono> = seen.iter().map(|(at, _)| *at).collect();
    assert_eq!(at, [ms(10), ms(15), ms(20), ms(30)]);
    assert!(matches!(seen[1].1, Stop::Timer(TimerKey::SwitcherDelay)));
    assert_eq!(
        clock.now(),
        ms(35),
        "the clock reaches its goal after the last stop"
    );
}

#[test]
fn a_timer_armed_for_the_present_instant_fires_without_the_clock_moving() {
    let clock = SharedClock::new(ms(50));
    let mut event_loop = looped(&clock);
    let mut seen = Vec::new();
    clock.update(|c| {
        c.schedule(Due {
            at: ms(40),
            key: TimerKey::SwitcherDelay,
        })
    });
    event_loop
        .dispatch(Some(Duration::ZERO), &mut seen)
        .expect("dispatch");
    assert_eq!(seen, vec![Stop::Timer(TimerKey::SwitcherDelay)]);
    assert_eq!(clock.now(), ms(50));
}

#[test]
fn an_output_that_is_unplugged_stops_having_vblanks() {
    let clock = SharedClock::new(ms(0));
    let (key, vblanks) = hundred_hertz("HEADLESS-1");
    clock.plug_output(key.clone(), vblanks);
    assert_eq!(clock.output_count(), 1);
    clock.unplug_output(&key);
    assert_eq!(clock.output_count(), 0);
    assert_eq!(clock.next_stop(ms(1000)), (ms(1000), Stop::End));
}

#[test]
fn the_clock_never_moves_backwards() {
    let clock = SharedClock::new(ms(500));
    clock.advance_to(ms(100));
    assert_eq!(clock.now(), ms(500));
}
