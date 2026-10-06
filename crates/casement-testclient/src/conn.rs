//! A test client's connection.
//!
//! A [`TestClient`] is a real `wayland_client::Connection` whose socket is relayed through a
//! [`Tap`](tap::Tap): the tap records every event into the client's [`Transcript`] and keeps the
//! object table, so no protocol helper has to remember to log. Nothing here blocks except
//! [`TestClient::connect_env`], which is bounded.

mod catalogue;
mod encode;
mod errors;
mod objects;
mod tap;
#[cfg(test)]
mod testing;
mod wire;

use crate::globals::{GlobalList, GlobalListContents};
use crate::pump::Pumped;
use crate::transcript::Transcript;
use casement_core::time::Mono;
use objects::Life;
use std::ops::RangeInclusive;
use std::os::fd::OwnedFd;
use std::os::unix::net::UnixStream;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tap::Tap;
use wayland_client::protocol::wl_callback::{self, WlCallback};
use wayland_client::protocol::wl_compositor::{self, WlCompositor};
use wayland_client::protocol::wl_surface::{self, WlSurface};
use wayland_client::{Connection, Dispatch, EventQueue, Proxy, QueueHandle};

/// What a dispatched event can reach: protocol helpers add their `Dispatch` impls for this
/// type in their own modules and keep their state in each object's user data.
#[derive(Debug, Default)]
pub struct ClientState;

/// A `wl_display.sync` that may have come back; the user data of its callback.
#[derive(Debug, Clone, Default)]
pub struct SyncDone(Arc<AtomicBool>);

impl SyncDone {
    /// Whether the compositor has answered the sync.
    pub fn is_done(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}

impl Dispatch<WlCallback, SyncDone> for ClientState {
    fn event(
        _: &mut Self,
        _: &WlCallback,
        event: wl_callback::Event,
        done: &SyncDone,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let wl_callback::Event::Done { .. } = event {
            done.0.store(true, Ordering::SeqCst);
        }
    }
}

/// The user data of the compositor and surface objects [`TestClient::create_surface`] makes.
/// Protocol helpers that bind their own `wl_compositor` use their own user data, so the two
/// never collide.
#[derive(Debug, Clone, Copy, Default)]
pub struct BaseData;

impl Dispatch<WlCompositor, BaseData> for ClientState {
    fn event(
        _: &mut Self,
        _: &WlCompositor,
        _: wl_compositor::Event,
        _: &BaseData,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<WlSurface, BaseData> for ClientState {
    fn event(
        _: &mut Self,
        _: &WlSurface,
        _: wl_surface::Event,
        _: &BaseData,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

/// A protocol error the compositor reported on this connection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientProtocolError {
    /// The error code, in the enum of the interface the object has.
    pub code: u32,
    /// The object's interface.
    pub interface: String,
    /// The object's id.
    pub object: u32,
    /// The compositor's text.
    pub message: String,
}

/// One test client: its connection, its event queue, the globals it saw and its transcript.
#[derive(Debug)]
pub struct TestClient {
    connection: Connection,
    queue: EventQueue<ClientState>,
    state: ClientState,
    globals: GlobalList,
    tap: Tap,
    fault: Option<String>,
}

/// Why a test client could not connect or talk.
#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    /// The connection failed.
    #[error("connect: {0}")]
    Connect(String),
    /// A roundtrip or dispatch failed (the compositor sent a protocol error or went away).
    #[error("dispatch: {0}")]
    Dispatch(String),
    /// A global the helper needs is not advertised.
    #[error("missing global {0}")]
    MissingGlobal(String),
}

/// Poll rounds `connect_env` waits for its first roundtrip, 100 ms each.
const CONNECT_ROUNDS: u32 = 100;

impl TestClient {
    /// A client over the client end of a fixture's socket pair. Non-blocking, so it cannot wait
    /// for the registry (the fixture serves the compositor on this same thread): it sends
    /// `get_registry` and returns, and [`TestClient::globals`] fills as the fixture pumps.
    pub fn over(fd: OwnedFd) -> Result<TestClient, ClientError> {
        TestClient::tapped(UnixStream::from(fd))
    }

    /// A client of `$WAYLAND_DISPLAY` (the jailed binary), with a bounded roundtrip.
    pub fn connect_env() -> Result<TestClient, ClientError> {
        let display = std::env::var_os("WAYLAND_DISPLAY")
            .ok_or_else(|| ClientError::Connect("WAYLAND_DISPLAY is not set".to_owned()))?;
        let path = std::path::Path::new(&display);
        let path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            let runtime = std::env::var_os("XDG_RUNTIME_DIR")
                .ok_or_else(|| ClientError::Connect("XDG_RUNTIME_DIR is not set".to_owned()))?;
            std::path::Path::new(&runtime).join(path)
        };
        let stream = UnixStream::connect(&path)
            .map_err(|error| ClientError::Connect(format!("{}: {error}", path.display())))?;
        let mut client = TestClient::tapped(stream)?;
        let sync = client.sync()?;
        for _ in 0..CONNECT_ROUNDS {
            client.pump();
            if sync.is_done() {
                return Ok(client);
            }
            if let Some(fault) = client.fault.take() {
                return Err(ClientError::Dispatch(fault));
            }
            // Wait for the compositor to say something; the bound is the round count.
            let mut fds = [rustix::event::PollFd::new(
                client.tap.server(),
                rustix::event::PollFlags::IN,
            )];
            let waited = rustix::event::poll(
                &mut fds,
                Some(&rustix::event::Timespec {
                    tv_sec: 0,
                    tv_nsec: 100_000_000,
                }),
            );
            if let Err(error) = waited {
                return Err(ClientError::Dispatch(format!("poll: {error}")));
            }
        }
        Err(ClientError::Dispatch(
            "the compositor did not answer the first roundtrip".to_owned(),
        ))
    }

    fn tapped(server: UnixStream) -> Result<TestClient, ClientError> {
        let (tap, theirs) =
            Tap::new(server).map_err(|error| ClientError::Connect(error.to_string()))?;
        let connection = Connection::from_socket(theirs)
            .map_err(|error| ClientError::Connect(error.to_string()))?;
        let queue = connection.new_event_queue();
        let contents = GlobalListContents::default();
        let registry = connection
            .display()
            .get_registry(&queue.handle(), contents.clone());
        let mut client = TestClient {
            connection,
            queue,
            state: ClientState,
            globals: GlobalList::new(registry, contents),
            tap,
            fault: None,
        };
        client.pump();
        Ok(client)
    }

    /// Reads what is there without blocking and dispatches it; says whether anything moved.
    pub fn pump(&mut self) -> Pumped {
        let mut moved = self.dispatch();
        moved += self.relay();
        if self.fault.is_none()
            && let Some(guard) = self.connection.prepare_read()
        {
            match guard.read() {
                Ok(read) => moved += read,
                Err(wayland_client::backend::WaylandError::Io(error))
                    if error.kind() == std::io::ErrorKind::WouldBlock => {}
                Err(error) => self.fault = Some(error.to_string()),
            }
        }
        moved += self.dispatch();
        moved += self.relay();
        if moved == 0 {
            Pumped::Quiet
        } else {
            Pumped::Moved
        }
    }

    fn dispatch(&mut self) -> usize {
        match self.queue.dispatch_pending(&mut self.state) {
            Ok(dispatched) => dispatched,
            Err(error) => {
                self.fault = Some(error.to_string());
                0
            }
        }
    }

    /// Flushes the connection's requests into the tap and moves the tap's bytes both ways.
    fn relay(&mut self) -> usize {
        // A full socket buffer leaves the rest queued in the backend; the next pump sends it.
        let _ = self.connection.flush();
        self.tap.relay()
    }

    /// The descriptor that becomes readable when the compositor sends something, for a binary
    /// that polls it together with other inputs (the probe's session reads stdin too).
    pub fn wait_fd(&self) -> std::os::fd::BorrowedFd<'_> {
        std::os::fd::AsFd::as_fd(self.tap.server())
    }

    /// Stamps the fixture's virtual `now` onto events relayed from here on.
    pub fn set_now(&mut self, now: Mono) {
        self.tap.now = now;
    }

    /// The globals the registry advertised.
    pub fn globals(&self) -> &GlobalList {
        &self.globals
    }

    /// Every event this client received, in order.
    pub fn transcript(&self) -> &Transcript {
        &self.tap.transcript
    }

    /// The connection, for helpers that send requests.
    pub fn connection(&self) -> &Connection {
        &self.connection
    }

    /// The queue's handle, to create objects with.
    pub fn qh(&self) -> QueueHandle<ClientState> {
        self.queue.handle()
    }

    /// Binds a global at a version in `versions`.
    pub fn bind<I, U>(&self, versions: RangeInclusive<u32>, udata: U) -> Result<I, ClientError>
    where
        I: Proxy + 'static,
        ClientState: Dispatch<I, U>,
        U: Send + Sync + 'static,
    {
        self.globals
            .bind(&self.qh(), versions, udata)
            .map_err(|_| ClientError::MissingGlobal(I::interface().name.to_owned()))
    }

    /// A new `wl_surface` (binding `wl_compositor` for it): the base the shell helpers build
    /// toplevels, popups and layer surfaces on. Its events are ignored here; the transcript
    /// has them, and a helper that wants them binds with its own user data.
    pub fn create_surface(&mut self) -> Result<WlSurface, ClientError> {
        let compositor: WlCompositor = self.bind(1..=6, BaseData)?;
        let surface = compositor.create_surface(&self.qh(), BaseData);
        let _ = self.connection.flush();
        Ok(surface)
    }

    /// Asks the compositor for a `wl_display.sync`; pump (the fixture's, or this client's)
    /// until [`SyncDone::is_done`].
    pub fn sync(&mut self) -> Result<SyncDone, ClientError> {
        let done = SyncDone::default();
        self.connection.display().sync(&self.qh(), done.clone());
        self.connection
            .flush()
            .map_err(|error| ClientError::Dispatch(error.to_string()))?;
        Ok(done)
    }

    /// The protocol error the compositor reported, if it did (read off the wire, so it
    /// carries the compositor's text and holds for errors posted on `wl_display` too).
    pub fn protocol_error(&self) -> Option<ClientProtocolError> {
        self.tap.error.clone()
    }

    /// Whether the protocol error the compositor reported is one its interface documents.
    /// `None` when there is no error.
    pub fn error_is_documented(&self) -> Option<bool> {
        let error = self.protocol_error()?;
        Some(errors::is_documented(&error.interface, error.code).unwrap_or(false))
    }

    /// Whether the compositor still holds the connection and nothing has failed here.
    pub fn is_connected(&self) -> bool {
        self.tap.is_open() && self.fault.is_none() && self.protocol_error().is_none()
    }

    /// What went wrong on this connection, if anything did (a dispatch or read failure).
    pub fn fault(&self) -> Option<&str> {
        self.fault.as_deref()
    }

    /// The objects created and not yet destroyed, as (id, interface), in id order.
    pub fn live_objects(&self) -> Vec<(u32, String)> {
        self.tap.objects.listed(Life::Live)
    }

    /// The objects destroyed, as (id, interface), in id order (their ids are not reused until
    /// the compositor's `delete_id`).
    pub fn destroyed_objects(&self) -> Vec<(u32, String)> {
        self.tap.objects.listed(Life::Dead)
    }

    /// The interfaces the clients can decode and build requests for, as (name, request count),
    /// in name order.
    pub fn known_interfaces() -> Vec<(&'static str, usize)> {
        catalogue::all()
            .values()
            .map(|interface| (interface.name, interface.requests.len()))
            .collect()
    }

    /// The opcode of request `request` of `interface`, when both are known.
    pub fn request_opcode(interface: &str, request: &str) -> Option<u16> {
        let known = catalogue::lookup(interface)?;
        let at = known
            .requests
            .iter()
            .position(|desc| desc.name == request)?;
        u16::try_from(at).ok()
    }

    /// Sends request `opcode` of `interface` on object `target` with arguments made from `ints`
    /// (see the fuzzer): boundary values of each argument's type, objects of every liveness,
    /// descriptors of the wrong kind. The target need not exist and the opcode need not be the
    /// interface's. New objects take ids this client has not used; do not mix with typed
    /// helpers that allocate their own.
    pub fn send_wire(&mut self, target: u32, interface: &str, opcode: u16, ints: &[i32]) {
        // Requests a typed helper queued are on the wire and in the object table first, so a
        // fresh id is really fresh.
        let _ = self.connection.flush();
        self.tap.relay();
        let desc =
            catalogue::lookup(interface).and_then(|known| known.requests.get(usize::from(opcode)));
        let globals = self.globals.contents().clone_list();
        let mut next_id = self.tap.objects.next_client_id();
        let built = encode::build(
            desc,
            ints,
            &mut encode::Context {
                objects: &self.tap.objects,
                globals: &globals,
                next_id: &mut next_id,
            },
        );
        self.send_raw(target, opcode, &built.payload, built.fds);
    }

    /// Writes one raw message, with descriptors, as if the client had sent it: for fuzzing
    /// requests no typed helper would produce. The connection's own pending requests go first.
    pub fn send_raw(&mut self, object: u32, opcode: u16, payload: &[u8], fds: Vec<OwnedFd>) {
        let _ = self.connection.flush();
        self.tap.relay();
        let size = u32::try_from(wire_len(payload)).unwrap_or(u32::MAX);
        let mut bytes = Vec::with_capacity(wire_len(payload));
        bytes.extend(object.to_ne_bytes());
        bytes.extend(((size << 16) | u32::from(opcode)).to_ne_bytes());
        bytes.extend(payload);
        self.tap.inject(bytes, fds);
        self.tap.relay();
    }
}

/// The message length for a payload: header plus payload, which the wire counts in bytes.
fn wire_len(payload: &[u8]) -> usize {
    wire::HEADER + payload.len()
}

#[cfg(test)]
mod tests {
    use super::testing::{global, heads, message, read_all};
    use super::*;
    use std::io::Write;

    /// A client over a pair whose other end is a compositor played by the test.
    fn client() -> (TestClient, UnixStream) {
        let (ours, compositor) = UnixStream::pair().expect("pair");
        compositor.set_nonblocking(true).expect("nonblocking");
        (
            TestClient::over(OwnedFd::from(ours)).expect("client"),
            compositor,
        )
    }

    #[test]
    fn a_new_client_asks_for_the_registry_and_nothing_else() {
        let (mut client, compositor) = client();
        client.pump();
        // wl_display.get_registry is opcode 1 on object 1.
        assert_eq!(heads(&read_all(&compositor)), vec![(1, 1)]);
        assert!(client.globals().contents().clone_list().is_empty());
        assert_eq!(client.pump(), Pumped::Quiet);
    }

    #[test]
    fn globals_fill_as_the_registry_is_dispatched_and_the_transcript_records_them() {
        let (mut client, mut compositor) = client();
        client.pump();
        read_all(&compositor);
        client.set_now(Mono(std::time::Duration::from_millis(3)));
        compositor
            .write_all(&global(2, 1, "wl_compositor", 5))
            .expect("global");
        compositor
            .write_all(&global(2, 2, "wl_shm", 2))
            .expect("global");
        assert_eq!(client.pump(), Pumped::Moved);
        let listed: Vec<_> = client
            .globals()
            .contents()
            .clone_list()
            .into_iter()
            .map(|g| (g.interface, g.version))
            .collect();
        assert_eq!(
            listed,
            vec![("wl_compositor".to_owned(), 5), ("wl_shm".to_owned(), 2)]
        );
        assert_eq!(
            client.transcript().text(),
            "3.000 wl_registry#2.global(1, \"wl_compositor\", 5)\n3.000 wl_registry#2.global(2, \"wl_shm\", 2)\n"
        );
        assert_eq!(client.pump(), Pumped::Quiet);
    }

    #[test]
    fn a_removed_global_leaves_the_list() {
        let (mut client, mut compositor) = client();
        client.pump();
        read_all(&compositor);
        compositor
            .write_all(&global(2, 1, "wl_compositor", 5))
            .expect("global");
        // wl_registry.global_remove is opcode 1.
        compositor
            .write_all(&message(2, 1, &1u32.to_ne_bytes()))
            .expect("remove");
        client.pump();
        assert!(client.globals().contents().clone_list().is_empty());
    }

    #[test]
    fn a_sync_completes_when_the_compositor_answers_it() {
        let (mut client, mut compositor) = client();
        client.pump();
        read_all(&compositor);
        let done = client.sync().expect("sync");
        client.pump();
        let sent = read_all(&compositor);
        // wl_display.sync is opcode 0; the callback is the next id, 3.
        assert_eq!(heads(&sent), vec![(1, 0)]);
        assert!(!done.is_done());
        // wl_callback.done(0), then wl_display.delete_id(3).
        compositor
            .write_all(&message(3, 0, &0u32.to_ne_bytes()))
            .expect("done");
        compositor
            .write_all(&message(1, 1, &3u32.to_ne_bytes()))
            .expect("delete_id");
        client.pump();
        assert!(done.is_done());
        assert!(client.is_connected());
        assert!(client.destroyed_objects().iter().any(|(id, _)| *id == 3));
    }

    #[test]
    fn a_protocol_error_is_reported_and_the_client_goes_quiet() {
        let (mut client, mut compositor) = client();
        client.pump();
        read_all(&compositor);
        // wl_display.error(object 1, code 0, "no such object")
        let mut payload = 1u32.to_ne_bytes().to_vec();
        payload.extend(0u32.to_ne_bytes());
        payload.extend(super::testing::string("no such object"));
        compositor
            .write_all(&message(1, 0, &payload))
            .expect("error");
        client.pump();
        let error = client.protocol_error().expect("an error");
        assert_eq!((error.code, error.interface.as_str()), (0, "wl_display"));
        assert!(!client.is_connected());
        assert!(
            client
                .transcript()
                .text()
                .contains("wl_display#1.error(wl_display#1, 0, \"no such object\")")
        );
        assert_eq!(client.pump(), Pumped::Quiet);
    }

    #[test]
    fn a_closed_compositor_is_a_disconnect_not_a_panic() {
        let (mut client, compositor) = client();
        client.pump();
        drop(compositor);
        client.pump();
        assert!(!client.is_connected());
        assert_eq!(client.pump(), Pumped::Quiet);
    }

    #[test]
    fn a_raw_request_reaches_the_compositor_and_its_new_id_is_tracked() {
        let (mut client, compositor) = client();
        client.pump();
        read_all(&compositor);
        // wl_display.sync with a chosen id.
        client.send_raw(1, 0, &40u32.to_ne_bytes(), Vec::new());
        assert_eq!(heads(&read_all(&compositor)), vec![(1, 0)]);
        assert!(
            client
                .live_objects()
                .iter()
                .any(|(id, name)| *id == 40 && name == "wl_callback")
        );
    }
}
