//! `ext_idle_notifier_v1` as a client (P0-H8): ask to be told after a timeout, record `idled`
//! and `resumed` with the virtual instant they were read at.
//!
//! The helper is self-contained until P0-T-client's core says how a protocol helper joins a
//! [`TestClient`]: it reaches the client's connection through the registry the client's
//! [`GlobalList`] holds ([`side_queue`]) and reads its own events on a second queue. Replace
//! [`side_queue`] and [`pump_queue`] with the core's when it lands.

use crate::{ClientError, GlobalList, Pumped};
use casement_core::time::Mono;
use std::time::Duration;
use wayland_client::protocol::wl_seat::{self, WlSeat};
use wayland_client::{Connection, Dispatch, EventQueue, Proxy, QueueHandle};
use wayland_protocols::ext::idle_notify::v1::client::{
    ext_idle_notification_v1::{self, ExtIdleNotificationV1},
    ext_idle_notifier_v1::{self, ExtIdleNotifierV1},
};

/// A second event queue on the connection `globals` came from, so a helper can bind and read
/// its own objects without the client's core. Its events are read when [`pump_queue`] runs or
/// when the client itself reads the socket.
pub fn side_queue<S>(globals: &GlobalList) -> Result<(Connection, EventQueue<S>), ClientError> {
    let backend = globals
        .registry()
        .backend()
        .upgrade()
        .ok_or_else(|| ClientError::Connect("the connection is closed".to_owned()))?;
    let connection = Connection::from_backend(backend);
    let queue = connection.new_event_queue();
    Ok((connection, queue))
}

/// Flushes `connection`, reads what is there without blocking and dispatches what `queue`
/// holds into `state`; says whether anything was dispatched.
pub fn pump_queue<S>(
    connection: &Connection,
    queue: &mut EventQueue<S>,
    state: &mut S,
) -> Result<Pumped, ClientError> {
    connection
        .flush()
        .map_err(|error| ClientError::Dispatch(error.to_string()))?;
    if let Some(guard) = connection.prepare_read() {
        // A non-blocking socket with nothing to read reports `WouldBlock`; that is quiet.
        let _ = guard.read();
    }
    let dispatched = queue
        .dispatch_pending(state)
        .map_err(|error| ClientError::Dispatch(error.to_string()))?;
    Ok(if dispatched > 0 {
        Pumped::Moved
    } else {
        Pumped::Quiet
    })
}

/// Which notification to ask for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IdleKind {
    /// `get_idle_notification`: idle inhibitors hold it off.
    Inhibitable,
    /// `get_input_idle_notification` (version 2): input only.
    InputOnly,
}

/// What the compositor told the watcher.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IdleSignal {
    /// `idled`.
    Idled,
    /// `resumed`.
    Resumed,
}

/// One signal and the instant it was read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IdleLine {
    /// The virtual instant the client read it (the pump's `now`).
    pub at: Mono,
    /// What it was.
    pub signal: IdleSignal,
}

/// What a pump has dispatched into.
#[derive(Debug)]
struct Seen {
    now: Mono,
    lines: Vec<IdleLine>,
}

/// One idle notification a test client holds.
#[derive(Debug)]
pub struct IdleWatch {
    connection: Connection,
    queue: EventQueue<Seen>,
    seen: Seen,
    notification: ExtIdleNotificationV1,
}

impl IdleWatch {
    /// Asks for a notification of `kind` after `timeout`. The request is flushed; the
    /// compositor sees it at the fixture's next pump.
    pub fn start(
        globals: &GlobalList,
        kind: IdleKind,
        timeout: Duration,
    ) -> Result<IdleWatch, ClientError> {
        let (connection, queue) = side_queue::<Seen>(globals)?;
        let qh = queue.handle();
        let missing = |name: &str| ClientError::MissingGlobal(name.to_owned());
        let version = match kind {
            IdleKind::Inhibitable => 1,
            IdleKind::InputOnly => 2,
        };
        let notifier = globals
            .bind::<ExtIdleNotifierV1, _, _>(&qh, version..=2, ())
            .map_err(|_| missing("ext_idle_notifier_v1"))?;
        let seat = globals
            .bind::<WlSeat, _, _>(&qh, 1..=1, ())
            .map_err(|_| missing("wl_seat"))?;
        let millis = u32::try_from(timeout.as_millis()).unwrap_or(u32::MAX);
        let notification = match kind {
            IdleKind::Inhibitable => notifier.get_idle_notification(millis, &seat, &qh, ()),
            IdleKind::InputOnly => notifier.get_input_idle_notification(millis, &seat, &qh, ()),
        };
        connection
            .flush()
            .map_err(|error| ClientError::Dispatch(error.to_string()))?;
        Ok(IdleWatch {
            connection,
            queue,
            seen: Seen {
                now: Mono(Duration::ZERO),
                lines: Vec::new(),
            },
            notification,
        })
    }

    /// Reads what arrived, stamping it with `now` (the fixture's virtual instant).
    pub fn pump(&mut self, now: Mono) -> Result<Pumped, ClientError> {
        self.seen.now = now;
        pump_queue(&self.connection, &mut self.queue, &mut self.seen)
    }

    /// Every signal read so far, in order.
    pub fn lines(&self) -> &[IdleLine] {
        &self.seen.lines
    }

    /// The signals, without their instants.
    pub fn signals(&self) -> Vec<IdleSignal> {
        self.seen.lines.iter().map(|line| line.signal).collect()
    }

    /// Destroys the notification (it stops counting).
    pub fn destroy(self) -> Result<(), ClientError> {
        self.notification.destroy();
        self.connection
            .flush()
            .map_err(|error| ClientError::Dispatch(error.to_string()))
    }
}

impl Dispatch<ExtIdleNotificationV1, ()> for Seen {
    fn event(
        state: &mut Seen,
        _proxy: &ExtIdleNotificationV1,
        event: ext_idle_notification_v1::Event,
        _data: &(),
        _connection: &Connection,
        _qh: &QueueHandle<Seen>,
    ) {
        let signal = match event {
            ext_idle_notification_v1::Event::Idled => IdleSignal::Idled,
            ext_idle_notification_v1::Event::Resumed => IdleSignal::Resumed,
            _ => return,
        };
        state.lines.push(IdleLine {
            at: state.now,
            signal,
        });
    }
}

impl Dispatch<ExtIdleNotifierV1, ()> for Seen {
    fn event(
        _state: &mut Seen,
        _proxy: &ExtIdleNotifierV1,
        _event: ext_idle_notifier_v1::Event,
        _data: &(),
        _connection: &Connection,
        _qh: &QueueHandle<Seen>,
    ) {
    }
}

impl Dispatch<WlSeat, ()> for Seen {
    fn event(
        _state: &mut Seen,
        _proxy: &WlSeat,
        _event: wl_seat::Event,
        _data: &(),
        _connection: &Connection,
        _qh: &QueueHandle<Seen>,
    ) {
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_line_names_the_instant_it_was_read() {
        let line = IdleLine {
            at: Mono(Duration::from_secs(30)),
            signal: IdleSignal::Idled,
        };
        assert_eq!(line.at.0, Duration::from_secs(30));
    }
}
