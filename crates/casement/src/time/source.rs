//! The session clock.

use super::calloop;
use super::virtual_clock::{SharedClock, VirtualTimerSource};
use crate::cli::ClockChoice;
use crate::core_loop;
use crate::state::{StartError, State};
use casement_core::time::{Due, Millihertz, Mono, Stop, TimerKey, VblankClock};
use casement_core::{Event, OutputKey};
use smithay::reexports::calloop::LoopHandle;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;
use std::time::{Duration, Instant};

/// What `Mono` is in this run. (The sketch held `VirtualClock` by value; the calloop source
/// shares it, so it is a shared handle: FINDINGS.)
#[derive(Debug, Clone)]
pub enum ClockSource {
    /// The monotonic wall clock, from this epoch, with its armed timers.
    Monotonic(WallClock),
    /// A virtual clock the test kit advances.
    Virtual(SharedClock),
    /// A virtual clock `casementctl debug clock` steps.
    Stepped(SharedClock),
}

/// The wall clock: the epoch `Mono` counts from, and what its calloop timers are for (a timer
/// that was cancelled or re-armed finds its entry changed when it fires and does nothing).
#[derive(Debug, Clone)]
pub struct WallClock {
    epoch: Instant,
    timers: Rc<RefCell<BTreeMap<TimerKey, Mono>>>,
    vblanks: Rc<RefCell<BTreeMap<OutputKey, u64>>>,
}

impl WallClock {
    /// A wall clock whose epoch is now.
    pub(super) fn start() -> WallClock {
        WallClock {
            epoch: Instant::now(),
            timers: Rc::new(RefCell::new(BTreeMap::new())),
            vblanks: Rc::new(RefCell::new(BTreeMap::new())),
        }
    }

    /// Now: the time since the epoch.
    pub(super) fn now(&self) -> Mono {
        Mono(self.epoch.elapsed())
    }

    /// The wall instant `at` is.
    pub(super) fn instant_of(&self, at: Mono) -> Instant {
        self.epoch + at.0
    }

    /// Records that `due` is armed.
    pub(super) fn arm(&self, due: &Due) {
        self.timers.borrow_mut().insert(due.key.clone(), due.at);
    }

    /// Whether `key` is still armed for `at`, taking it out if so (it is firing).
    pub(super) fn take_if_armed(&self, key: &TimerKey, at: Mono) -> bool {
        let mut timers = self.timers.borrow_mut();
        match timers.get(key) {
            Some(armed) if *armed == at => timers.remove(key).is_some(),
            _ => false,
        }
    }

    /// `output`'s vblank timer starts a new generation (any earlier one stops at its next
    /// tick); returns it.
    pub(super) fn start_vblanks(&self, output: &OutputKey) -> u64 {
        let mut vblanks = self.vblanks.borrow_mut();
        let next = vblanks.get(output).map_or(1, |generation| generation + 1);
        vblanks.insert(output.clone(), next);
        next
    }

    /// Whether `generation` is `output`'s current vblank timer.
    pub(super) fn vblanks_run(&self, output: &OutputKey, generation: u64) -> bool {
        self.vblanks.borrow().get(output) == Some(&generation)
    }
}

impl ClockSource {
    /// The clock `choice` asks for, its timer source inserted into `event_loop`.
    pub fn start(
        choice: ClockChoice,
        event_loop: &LoopHandle<'static, State>,
    ) -> Result<ClockSource, StartError> {
        match choice {
            ClockChoice::Wall => Ok(ClockSource::Monotonic(WallClock::start())),
            ClockChoice::Stepped => virtual_clock(event_loop).map(ClockSource::Stepped),
            ClockChoice::Virtual => virtual_clock(event_loop).map(ClockSource::Virtual),
        }
    }

    /// Now, on this clock.
    pub fn now(&self) -> Mono {
        match self {
            ClockSource::Monotonic(wall) => wall.now(),
            ClockSource::Virtual(clock) | ClockSource::Stepped(clock) => clock.now(),
        }
    }

    /// The virtual clock, when this is one.
    pub fn shared(&self) -> Option<&SharedClock> {
        match self {
            ClockSource::Monotonic(_) => None,
            ClockSource::Virtual(clock) | ClockSource::Stepped(clock) => Some(clock),
        }
    }

    /// Arms `due` (`Effect::ArmTimer`): on the virtual clock's book, or as a calloop timer.
    pub fn arm(&self, handle: &LoopHandle<'static, State>, due: Due) {
        match self {
            ClockSource::Monotonic(wall) => calloop::arm(handle, wall, due),
            ClockSource::Virtual(clock) | ClockSource::Stepped(clock) => {
                clock.update(|clock| clock.schedule(due));
            }
        }
    }

    /// Cancels the timer for `key` (`Effect::CancelTimer`).
    pub fn cancel(&self, key: &TimerKey) {
        match self {
            ClockSource::Monotonic(wall) => {
                wall.timers.borrow_mut().remove(key);
            }
            ClockSource::Virtual(clock) | ClockSource::Stepped(clock) => {
                clock.update(|clock| {
                    let timers = clock.timers().clone().cancel(key);
                    clock.with_timers(timers)
                });
            }
        }
    }

    /// `output`'s vblanks begin (or restart at this refresh rate): each calls
    /// `core_loop::frame` for it.
    pub fn plug_output(
        &self,
        handle: &LoopHandle<'static, State>,
        output: OutputKey,
        refresh: Millihertz,
    ) {
        match self {
            ClockSource::Monotonic(wall) => calloop::run_vblanks(handle, wall, output, refresh),
            ClockSource::Virtual(clock) | ClockSource::Stepped(clock) => {
                clock.plug_output(output, VblankClock::new(refresh, clock.now()));
            }
        }
    }

    /// `output`'s vblanks stop.
    pub fn unplug_output(&self, output: &OutputKey) {
        match self {
            ClockSource::Monotonic(wall) => {
                wall.vblanks.borrow_mut().remove(output);
            }
            ClockSource::Virtual(clock) | ClockSource::Stepped(clock) => {
                clock.unplug_output(output);
            }
        }
    }

    /// The real instant now, for a wait that depends on processes outside the session (a
    /// stepped clock stands still while a client starts).
    pub fn wall_instant(&self) -> Instant {
        Instant::now()
    }

    /// `Mono` after `by` (a timer's due instant).
    pub fn after(&self, by: Duration) -> Mono {
        self.now().after(by)
    }
}

/// A virtual clock at the epoch with its source inserted into the loop.
fn virtual_clock(event_loop: &LoopHandle<'static, State>) -> Result<SharedClock, StartError> {
    let clock = SharedClock::new(Mono(Duration::ZERO));
    event_loop
        .insert_source(
            VirtualTimerSource::new(clock.clone()),
            |stop, (), state: &mut State| route(state, stop),
        )
        .map_err(|error| StartError::Loop(error.to_string()))?;
    Ok(clock)
}

/// What each stop the virtual clock reaches does: a timer is an event, a vblank is a frame.
fn route(state: &mut State, stop: Stop) {
    match stop {
        Stop::Timer(key) => core_loop::feed(state, vec![Event::TimerFired(key)]),
        Stop::Vblank(output, _seq) => core_loop::frame(state, &output),
        Stop::End => {}
    }
}
