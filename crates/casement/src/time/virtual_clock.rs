//! The virtual clock as a calloop event source (A1, SP3). The clock (`casement_core::time::
//! VirtualClock`: now, timers, vblank clocks) is shared between the composition root and the
//! source. Time moves only when told: [`SharedClock::advance_to`] jumps and queues what
//! happened on the way, [`SharedClock::run_to`] sets a goal the source walks toward one stop
//! at a time. Either way the source's `before_sleep` asks calloop to wake at once when
//! anything is ready, and `process_events` hands one arrival's stops out in order (timers
//! before vblanks at one instant, timers by arming order, vblanks by output key). It never
//! reads the wall clock and is not thread-installed: two fixtures on one thread keep two
//! clocks.
//!
//! A32: every move of the clock is `next_stop` then `arrive`, never the bare
//! `VirtualClock::advance_to`, so a timer and a vblank (or two outputs' vblanks) that share an
//! instant arrive together and in the same order however an advance is chopped.

use casement_core::OutputKey;
use casement_core::time::{Arrival, Mono, Stop, TimerKey, VblankClock, VirtualClock};
use smithay::reexports::calloop::{EventSource, Poll, PostAction, Readiness, Token, TokenFactory};
use std::cell::RefCell;
use std::collections::{BTreeSet, VecDeque};
use std::rc::Rc;

/// A virtual clock shared by the composition root and its event source.
#[derive(Debug, Clone)]
pub struct SharedClock(Rc<RefCell<Inner>>);

/// What the handles share.
#[derive(Debug)]
struct Inner {
    clock: VirtualClock,
    /// Where [`SharedClock::run_to`] is walking to (never before now).
    goal: Mono,
    /// Arrivals the clock made that the source has not handed out yet.
    queue: VecDeque<Arrival>,
    /// The timers handed out so far, until [`SharedClock::take_fired`] reads them.
    fired: Vec<TimerKey>,
    /// How many stops (timers and vblanks) the source has handed out.
    delivered: u64,
    /// The outputs whose vblanks the clock follows.
    outputs: BTreeSet<OutputKey>,
}

impl SharedClock {
    /// A clock standing at `epoch`.
    pub fn new(epoch: Mono) -> SharedClock {
        SharedClock(Rc::new(RefCell::new(Inner {
            clock: VirtualClock::new(epoch),
            goal: epoch,
            queue: VecDeque::new(),
            fired: Vec::new(),
            delivered: 0,
            outputs: BTreeSet::new(),
        })))
    }

    /// Now.
    pub fn now(&self) -> Mono {
        self.0.borrow().clock.now()
    }

    /// Applies `change` to the clock (arming, cancelling).
    pub fn update(&self, change: impl FnOnce(VirtualClock) -> VirtualClock) {
        self.with_clock(change);
    }

    /// Where the next advance stops, at most `limit`.
    pub fn next_stop(&self, limit: Mono) -> (Mono, Stop) {
        self.0.borrow().clock.next_stop(limit)
    }

    /// Moves the clock to `t` (never backwards) and queues what happened there: the timers
    /// due by `t` in firing order, then the vblanks that fall exactly at `t`. The event source
    /// hands them out on its next turn. Call it with the instant `next_stop` named.
    pub fn advance_to(&self, t: Mono) {
        let mut inner = self.0.borrow_mut();
        let (clock, arrival) = replace(&mut inner.clock, |clock| clock.arrive(t));
        inner.clock = clock;
        if !(arrival.timers.is_empty() && arrival.vblanks.is_empty()) {
            inner.queue.push_back(arrival);
        }
    }

    /// Asks the source to walk the clock to `t`, one stop at a time, handing each stop out as
    /// it arrives (so what a stop arms is seen before the next one). Never backwards.
    pub fn run_to(&self, t: Mono) {
        let mut inner = self.0.borrow_mut();
        inner.goal = inner.goal.max(t);
    }

    /// The vblanks of `output` follow `clock` from now on.
    pub fn plug_output(&self, output: OutputKey, clock: VblankClock) {
        let mut inner = self.0.borrow_mut();
        inner.outputs.insert(output.clone());
        let (moved, ()) = replace(&mut inner.clock, |c| (c.with_output(output, clock), ()));
        inner.clock = moved;
    }

    /// The vblanks of `output` stop.
    pub fn unplug_output(&self, output: &OutputKey) {
        let mut inner = self.0.borrow_mut();
        inner.outputs.remove(output);
        let (moved, ()) = replace(&mut inner.clock, |c| (c.without_output(output), ()));
        inner.clock = moved;
    }

    /// How many outputs' vblanks the clock follows.
    pub fn output_count(&self) -> usize {
        self.0.borrow().outputs.len()
    }

    /// How many stops the source has handed out (a fixture reads it to know a round moved).
    pub fn delivered(&self) -> u64 {
        self.0.borrow().delivered
    }

    /// The timers handed out since the last call, in firing order.
    pub fn take_fired(&self) -> Vec<TimerKey> {
        std::mem::take(&mut self.0.borrow_mut().fired)
    }

    /// Whether the source has something to hand out now.
    fn ready(&self) -> bool {
        let inner = self.0.borrow();
        let now = inner.clock.now();
        !inner.queue.is_empty()
            || inner.clock.timers().next().is_some_and(|due| due.at <= now)
            || now < inner.goal
    }

    /// The next arrival to hand out: one the clock queued; else the timers that came due at
    /// the current instant (armed after the clock got here); else one step toward the goal.
    fn step(&self) -> Option<Arrival> {
        let mut inner = self.0.borrow_mut();
        if let Some(queued) = inner.queue.pop_front() {
            return Some(queued);
        }
        let now = inner.clock.now();
        if inner.clock.timers().next().is_some_and(|due| due.at <= now) {
            let (clock, timers) = replace(&mut inner.clock, VirtualClock::fire_due);
            inner.clock = clock;
            return Some(Arrival {
                at: now,
                timers,
                vblanks: Vec::new(),
            });
        }
        if now < inner.goal {
            let (at, _) = inner.clock.next_stop(inner.goal);
            let (clock, arrival) = replace(&mut inner.clock, |clock| clock.arrive(at));
            inner.clock = clock;
            return Some(arrival);
        }
        None
    }

    /// Notes that the source handed out `stop`.
    fn handed_out(&self, stop: &Stop) {
        let mut inner = self.0.borrow_mut();
        inner.delivered = inner.delivered.saturating_add(1);
        if let Stop::Timer(key) = stop {
            inner.fired.push(key.clone());
        }
    }

    /// The clock after `change` (the clock is a value: it is taken, changed and put back).
    fn with_clock(&self, change: impl FnOnce(VirtualClock) -> VirtualClock) {
        let mut inner = self.0.borrow_mut();
        let (clock, ()) = replace(&mut inner.clock, |clock| (change(clock), ()));
        inner.clock = clock;
    }
}

/// Takes `clock` by value out of its place (leaving a stand-in), applies `change` and returns
/// what it made.
fn replace<T>(clock: &mut VirtualClock, change: impl FnOnce(VirtualClock) -> T) -> T {
    let epoch = clock.now();
    change(std::mem::replace(clock, VirtualClock::new(epoch)))
}

/// The calloop source handing out the virtual clock's stops.
#[derive(Debug)]
pub struct VirtualTimerSource {
    clock: SharedClock,
    token: Option<Token>,
}

impl VirtualTimerSource {
    /// A source over `clock`.
    pub fn new(clock: SharedClock) -> VirtualTimerSource {
        VirtualTimerSource { clock, token: None }
    }
}

impl EventSource for VirtualTimerSource {
    type Event = Stop;
    type Metadata = ();
    type Ret = ();
    type Error = std::io::Error;

    const NEEDS_EXTRA_LIFECYCLE_EVENTS: bool = true;

    fn process_events<F>(
        &mut self,
        readiness: Readiness,
        token: Token,
        mut callback: F,
    ) -> Result<PostAction, Self::Error>
    where
        F: FnMut(Self::Event, &mut Self::Metadata) -> Self::Ret,
    {
        let _ = (readiness, token);
        if let Some(arrival) = self.clock.step() {
            let timers = arrival.timers.into_iter().map(Stop::Timer);
            let vblanks = arrival
                .vblanks
                .into_iter()
                .map(|(output, seq)| Stop::Vblank(output, seq));
            for stop in timers.chain(vblanks) {
                self.clock.handed_out(&stop);
                callback(stop, &mut ());
            }
        }
        Ok(PostAction::Continue)
    }

    fn before_sleep(&mut self) -> smithay::reexports::calloop::Result<Option<(Readiness, Token)>> {
        Ok(match (self.clock.ready(), self.token) {
            (true, Some(token)) => Some((Readiness::EMPTY, token)),
            _ => None,
        })
    }

    fn register(
        &mut self,
        _poll: &mut Poll,
        factory: &mut TokenFactory,
    ) -> smithay::reexports::calloop::Result<()> {
        self.token = Some(factory.token());
        Ok(())
    }

    fn reregister(
        &mut self,
        poll: &mut Poll,
        factory: &mut TokenFactory,
    ) -> smithay::reexports::calloop::Result<()> {
        self.register(poll, factory)
    }

    fn unregister(&mut self, _poll: &mut Poll) -> smithay::reexports::calloop::Result<()> {
        self.token = None;
        Ok(())
    }
}
