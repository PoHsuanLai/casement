//! Looking at the program behind a new public connection, off the loop thread (NEW-CASB-4,
//! NEW-CAS-3). Holding the process and reading its facts touches `/proc` and stats the program's
//! image; an image on a hung network or FUSE mount (an AppImage whose helper is stuck) blocks that
//! stat in the kernel, and on the loop that is the whole session. The look runs on a thread of
//! its own, on a copy of the connection's descriptor, and is given [`PROBE_DEADLINE`]: the
//! connection is taken in with what the look found, or, if it has not finished by then, as a
//! program nothing is known of (asked about every power). At most [`MAX_PROBES`] looks run at
//! once; a connection past that is taken in unknown at once, so stuck looks cannot grow the
//! thread count either.

use super::{Arrival, Judge, Room, insert_client, room_for_peer};
use crate::state::State;
use crate::time;
use casement_core::policy::ConnectionRole;
use smithay::reexports::calloop::channel::{Channel, Event as ChannelEvent, Sender, channel};
use smithay::reexports::calloop::{Error as LoopError, LoopHandle};
use std::collections::BTreeMap;
use std::os::unix::net::UnixStream;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

/// Looks running at once: a person's programs connect a few at a time.
pub const MAX_PROBES: usize = 8;

/// How long a look may take before the connection is taken in unknown.
pub const PROBE_DEADLINE: Duration = Duration::from_secs(2);

/// Which look.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct ProbeId(u64);

/// A finished look.
#[derive(Debug)]
struct Done {
    id: ProbeId,
    arrival: Arrival,
}

/// Counts a running look down when it ends, however it ends.
struct Running(Arc<AtomicUsize>);

impl Drop for Running {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}

/// The looks in progress, or nothing when connections are taken in without one (a test).
#[derive(Debug, Default)]
pub enum Probes {
    /// Connections are taken in as unknown.
    #[default]
    Off,
    /// Looks run on threads.
    On(Looking),
}

/// The state of the looks.
#[derive(Debug)]
pub struct Looking {
    judge: Arc<Judge>,
    /// The connections waiting for their look, by look.
    pending: BTreeMap<ProbeId, UnixStream>,
    next: u64,
    running: Arc<AtomicUsize>,
    done: Sender<Done>,
}

impl Probes {
    /// Looks run by `judge`; their results arrive on `handle`'s loop.
    pub fn start(judge: Judge, handle: &LoopHandle<'static, State>) -> Result<Probes, LoopError> {
        let (done, results): (Sender<Done>, Channel<Done>) = channel();
        handle
            .insert_source(results, |event, (), state: &mut State| {
                if let ChannelEvent::Msg(done) = event {
                    finish(state, done.id, done.arrival);
                }
            })
            .map_err(LoopError::from)?;
        Ok(Probes::On(Looking {
            judge: Arc::new(judge),
            pending: BTreeMap::new(),
            next: 0,
            running: Arc::new(AtomicUsize::new(0)),
            done,
        }))
    }
}

/// A connection arrived on the public socket: look at its program, then take it in.
pub fn begin(state: &mut State, stream: UnixStream) {
    let floor = state.core.model.privilege().floor().clone();
    let Probes::On(looking) = &mut state.probes else {
        return admit(state, stream, Arrival::unknown());
    };
    if looking.running.load(Ordering::SeqCst) >= MAX_PROBES {
        return admit(state, stream, Arrival::unknown());
    }
    let Ok(copy) = stream.try_clone() else {
        return admit(state, stream, Arrival::unknown());
    };
    let id = ProbeId(looking.next);
    looking.next = looking.next.saturating_add(1);
    looking.running.fetch_add(1, Ordering::SeqCst);
    let (judge, done, running) = (
        Arc::clone(&looking.judge),
        looking.done.clone(),
        Running(Arc::clone(&looking.running)),
    );
    let started = std::thread::Builder::new()
        .name("casement-probe".to_owned())
        .spawn(move || {
            let _running = running;
            let arrival = judge.arrival(&copy, &floor);
            let _ = done.send(Done { id, arrival });
        });
    match started {
        Ok(_) => {
            looking.pending.insert(id, stream);
            let handle = state.event_loop.clone();
            time::after(&handle, PROBE_DEADLINE, move |state: &mut State| {
                finish(state, id, Arrival::unknown());
            });
        }
        Err(error) => {
            tracing::warn!(%error, "a program could not be looked at: no thread");
            // The closure with the counter guard was dropped with the failed spawn.
            admit(state, stream, Arrival::unknown());
        }
    }
}

/// The look `id` ended, or its time ran out: the connection is taken in with `arrival`. The
/// second of the two finds nothing waiting and does nothing.
fn finish(state: &mut State, id: ProbeId, arrival: Arrival) {
    let Probes::On(looking) = &mut state.probes else {
        return;
    };
    if let Some(stream) = looking.pending.remove(&id) {
        admit(state, stream, arrival);
    }
}

/// Takes the connection in as a public client, unless its program already holds as many
/// connections as one may.
fn admit(state: &mut State, stream: UnixStream, arrival: Arrival) {
    if room_for_peer(state, &arrival.peer, &stream) != Room::Free {
        return;
    }
    let role = ConnectionRole::Public;
    if let Err(error) = insert_client(state, stream, role, arrival.peer, arrival.standing) {
        tracing::warn!(%error, "a client could not be inserted");
    }
}
