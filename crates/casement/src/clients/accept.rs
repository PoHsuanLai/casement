//! Accepting connections on a listening socket without letting a failed accept end the session
//! (NEW-CAS-1, NEW-CASB-1). smithay's listening sources return the `accept` error from
//! `process_events`, and calloop ends `EventLoop::run` with it: one `EMFILE` (a program that
//! opens connections until the descriptors are gone), `ENFILE`, `ECONNABORTED` or `ENOMEM` from
//! an unprivileged peer would take the whole session down. [`insert`] wraps such a source so
//! the error is logged, the listener rests for a moment (a level-triggered listener whose
//! accept keeps failing would otherwise spin the loop) and then listens again; the clients
//! already connected never notice.

use smithay::reexports::calloop::{
    EventSource, LoopHandle, Poll, PostAction, Readiness, RegistrationToken, Token, TokenFactory,
};
use std::cell::Cell;
use std::fmt;
use std::io;
use std::rc::Rc;
use std::time::Duration;

/// How long a listener rests after an accept fails: long enough that a descriptor shortage
/// does not spin the loop, short enough that a person's next program does not wait.
pub const ACCEPT_REST: Duration = Duration::from_millis(250);

/// A listening source whose accept errors rest the listener instead of ending the loop.
pub struct Surviving<S, D: 'static> {
    inner: S,
    handle: LoopHandle<'static, D>,
    /// The registration the rest ends by enabling again; set once the source is inserted.
    token: Rc<Cell<Option<RegistrationToken>>>,
    rest: Duration,
}

impl<S, D: 'static> fmt::Debug for Surviving<S, D> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Surviving")
            .field("rest", &self.rest)
            .finish_non_exhaustive()
    }
}

/// Inserts `source` into the loop behind `handle` so that an accept error rests it for `rest`
/// and is logged, instead of ending the loop. `callback` gets each accepted stream as it
/// would from `source` itself.
pub fn insert<S, D, F>(
    handle: &LoopHandle<'static, D>,
    source: S,
    rest: Duration,
    callback: F,
) -> smithay::reexports::calloop::Result<()>
where
    S: EventSource<Error = io::Error> + 'static,
    D: 'static,
    F: FnMut(S::Event, &mut S::Metadata, &mut D) -> S::Ret + 'static,
{
    let token = Rc::new(Cell::new(None));
    let surviving = Surviving {
        inner: source,
        handle: handle.clone(),
        token: Rc::clone(&token),
        rest,
    };
    let registered = handle.insert_source(surviving, callback)?;
    token.set(Some(registered));
    Ok(())
}

impl<S, D> Surviving<S, D>
where
    D: 'static,
{
    /// Logs `error` and lets the listener rest: it is switched off now and on again after the
    /// rest. A source removed in the meantime makes the switch-on a no-op.
    fn rest_after(&self, error: &io::Error) {
        tracing::warn!(%error, "a connection could not be accepted; listening again shortly");
        let handle = self.handle.clone();
        let token = Rc::clone(&self.token);
        crate::time::after(&self.handle, self.rest, move |_: &mut D| {
            if let Some(token) = token.get()
                && let Err(error) = handle.enable(&token)
            {
                tracing::debug!(%error, "a resting listener is gone");
            }
        });
    }
}

impl<S, D> EventSource for Surviving<S, D>
where
    S: EventSource<Error = io::Error>,
    D: 'static,
{
    type Event = S::Event;
    type Metadata = S::Metadata;
    type Ret = S::Ret;
    type Error = io::Error;

    fn process_events<F>(
        &mut self,
        readiness: Readiness,
        token: Token,
        callback: F,
    ) -> io::Result<PostAction>
    where
        F: FnMut(Self::Event, &mut Self::Metadata) -> Self::Ret,
    {
        match self.inner.process_events(readiness, token, callback) {
            Ok(action) => Ok(action),
            Err(error) => {
                self.rest_after(&error);
                Ok(PostAction::Disable)
            }
        }
    }

    fn register(
        &mut self,
        poll: &mut Poll,
        token_factory: &mut TokenFactory,
    ) -> smithay::reexports::calloop::Result<()> {
        self.inner.register(poll, token_factory)
    }

    fn reregister(
        &mut self,
        poll: &mut Poll,
        token_factory: &mut TokenFactory,
    ) -> smithay::reexports::calloop::Result<()> {
        self.inner.reregister(poll, token_factory)
    }

    fn unregister(&mut self, poll: &mut Poll) -> smithay::reexports::calloop::Result<()> {
        self.inner.unregister(poll)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use smithay::reexports::calloop::generic::Generic;
    use smithay::reexports::calloop::{EventLoop, Interest, Mode};
    use std::io::{Read, Write};
    use std::os::unix::net::UnixStream;

    /// A listener whose `accept` fails `failures` times with `EMFILE`, then yields one "stream"
    /// per byte that is ready (the byte it read).
    struct Flaky {
        ready: Generic<UnixStream>,
        failures: u32,
    }

    impl EventSource for Flaky {
        type Event = u8;
        type Metadata = ();
        type Ret = ();
        type Error = io::Error;

        fn process_events<F>(
            &mut self,
            readiness: Readiness,
            token: Token,
            mut callback: F,
        ) -> io::Result<PostAction>
        where
            F: FnMut(u8, &mut ()),
        {
            let failures = &mut self.failures;
            self.ready.process_events(readiness, token, |_, stream| {
                if *failures > 0 {
                    *failures -= 1;
                    return Err(io::Error::from(rustix::io::Errno::MFILE));
                }
                let mut byte = [0u8; 1];
                (&**stream).read_exact(&mut byte)?;
                callback(byte[0], &mut ());
                Ok(PostAction::Continue)
            })
        }

        fn register(
            &mut self,
            poll: &mut Poll,
            tokens: &mut TokenFactory,
        ) -> smithay::reexports::calloop::Result<()> {
            self.ready.register(poll, tokens)
        }

        fn reregister(
            &mut self,
            poll: &mut Poll,
            tokens: &mut TokenFactory,
        ) -> smithay::reexports::calloop::Result<()> {
            self.ready.reregister(poll, tokens)
        }

        fn unregister(&mut self, poll: &mut Poll) -> smithay::reexports::calloop::Result<()> {
            self.ready.unregister(poll)
        }
    }

    /// The most turns a test waits for the listener to come back; each turn waits at most 5 ms.
    const TURNS: u32 = 400;

    #[test]
    fn an_accept_error_rests_the_listener_and_the_next_connection_is_still_taken() {
        let mut event_loop: EventLoop<'static, Vec<u8>> = EventLoop::try_new().expect("a loop");
        let (mut peer, ours) = UnixStream::pair().expect("a socket pair");
        peer.write_all(&[7]).expect("a connection waiting");
        let source = Flaky {
            ready: Generic::new(ours, Interest::READ, Mode::Level),
            failures: 3,
        };
        insert(
            &event_loop.handle(),
            source,
            Duration::from_millis(1),
            |byte, (), taken: &mut Vec<u8>| {
                taken.push(byte);
            },
        )
        .expect("the listener joins the loop");
        let mut taken = Vec::new();
        for _ in 0..TURNS {
            // Every dispatch returns Ok: an accept error never reaches the loop's caller.
            event_loop
                .dispatch(Some(Duration::from_millis(5)), &mut taken)
                .expect("an accept error does not end the loop");
            if !taken.is_empty() {
                break;
            }
        }
        assert_eq!(
            taken,
            vec![7],
            "the connection waiting behind the errors was taken"
        );
    }
}
