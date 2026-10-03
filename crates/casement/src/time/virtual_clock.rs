//! The virtual clock as a calloop event source (A1, SP3). The clock (`casement_core::time::
//! VirtualClock`: now, timers, vblank clocks) is shared between the composition root and the
//! source; the source's `before_sleep` asks calloop to wake at once when anything is due at
//! the current instant, and `process_events` hands every due stop out in order (timers before
//! vblanks at one instant, timers by arming order, vblanks by output key). It never reads the
//! wall clock and is not thread-installed: two fixtures on one thread keep two clocks.

use casement_core::time::{Mono, Stop, VirtualClock};
use smithay::reexports::calloop::{EventSource, Poll, PostAction, Readiness, Token, TokenFactory};
use std::cell::RefCell;
use std::rc::Rc;

/// A virtual clock shared by the composition root and its event source.
#[derive(Debug, Clone)]
pub struct SharedClock(Rc<RefCell<VirtualClock>>);

impl SharedClock {
    /// A clock standing at `epoch`.
    pub fn new(epoch: Mono) -> SharedClock {
        SharedClock(Rc::new(RefCell::new(VirtualClock::new(epoch))))
    }

    /// Now.
    pub fn now(&self) -> Mono {
        self.0.borrow().now()
    }

    /// Applies `change` to the clock (arming, cancelling, adding an output's vblanks).
    pub fn update(&self, change: impl FnOnce(VirtualClock) -> VirtualClock) {
        let _ = change;
        todo!("P0-B-loop: SharedClock::update")
    }

    /// Where the next advance stops, at most `limit`.
    pub fn next_stop(&self, limit: Mono) -> (Mono, Stop) {
        self.0.borrow().next_stop(limit)
    }

    /// Moves the clock to `t` (never backwards).
    pub fn advance_to(&self, t: Mono) {
        let _ = t;
        todo!("P0-B-loop: SharedClock::advance_to")
    }
}

/// The calloop source handing out the virtual clock's due stops.
#[derive(Debug)]
pub struct VirtualTimerSource {
    #[expect(dead_code, reason = "P0-B-loop: the clock the source reads")]
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
        callback: F,
    ) -> Result<PostAction, Self::Error>
    where
        F: FnMut(Self::Event, &mut Self::Metadata) -> Self::Ret,
    {
        let _ = (readiness, token, callback);
        todo!("P0-B-loop: VirtualTimerSource::process_events")
    }

    fn before_sleep(&mut self) -> smithay::reexports::calloop::Result<Option<(Readiness, Token)>> {
        todo!("P0-B-loop: VirtualTimerSource::before_sleep")
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
