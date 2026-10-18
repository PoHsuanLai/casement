//! The tap: a relay between the compositor's end of a connection and the end the real
//! `wayland_client::Connection` reads. Every byte and every descriptor passes through
//! unchanged, in order; on the way the tap decodes each message by its object's interface, so
//! the client's [`Transcript`] holds every event whatever helper (or none) handled it, and the
//! object table knows which ids are live, destroyed or never seen.
//!
//! The tap never blocks and never reads a clock: it moves what is there, and the caller says
//! what time it is.

use super::ClientProtocolError;
use super::objects::Objects;
use super::wire::{self, Arg, Frame};
use crate::transcript::{Transcript, TranscriptLine};
use casement_core::time::Mono;
use rustix::net::{
    RecvAncillaryBuffer, RecvAncillaryMessage, RecvFlags, SendAncillaryBuffer,
    SendAncillaryMessage, SendFlags, recvmsg, sendmsg,
};
use std::collections::VecDeque;
use std::io::{self, IoSlice, IoSliceMut};
use std::mem::MaybeUninit;
use std::os::fd::{AsFd, OwnedFd};
use std::os::unix::net::UnixStream;

/// How much one read takes.
const CHUNK: usize = 64 * 1024;
/// Descriptors one read can carry (the protocol's own limit is 28 per message).
const MAX_FDS: usize = 32;
/// Reads per direction per relay, so a flooding peer cannot starve the pump.
const READS: usize = 16;

/// Which way a message travels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Way {
    /// Client to compositor: requests.
    Up,
    /// Compositor to client: events.
    Down,
}

/// Bytes (and the descriptors that arrived with them) waiting to be written on.
#[derive(Debug)]
struct Chunk {
    bytes: Vec<u8>,
    fds: Vec<OwnedFd>,
    sent: usize,
}

/// One direction's state.
#[derive(Debug, Default)]
struct Lane {
    /// The tail of the stream not yet a whole message.
    carry: Vec<u8>,
    /// Waiting to be written to the far end.
    out: VecDeque<Chunk>,
    /// The near end has nothing more to give.
    source_closed: bool,
    /// The far end was told so.
    shut: bool,
    /// The far end is gone.
    dead: bool,
}

/// The relay.
#[derive(Debug)]
pub(super) struct Tap {
    server: UnixStream,
    inner: UnixStream,
    up: Lane,
    down: Lane,
    pub(super) objects: Objects,
    pub(super) transcript: Transcript,
    pub(super) now: Mono,
    /// The first `wl_display.error` the compositor sent: libwayland reports the ones posted on
    /// `wl_display` itself as bare errno values, so the tap reads the event.
    pub(super) error: Option<ClientProtocolError>,
}

impl Tap {
    /// A tap over `server` (the compositor's end), and the socket the connection should use.
    pub(super) fn new(server: UnixStream) -> io::Result<(Tap, UnixStream)> {
        server.set_nonblocking(true)?;
        let (inner, theirs) = UnixStream::pair()?;
        inner.set_nonblocking(true)?;
        theirs.set_nonblocking(true)?;
        let tap = Tap {
            server,
            inner,
            up: Lane::default(),
            down: Lane::default(),
            objects: Objects::default(),
            transcript: Transcript::default(),
            now: Mono(std::time::Duration::ZERO),
            error: None,
        };
        Ok((tap, theirs))
    }

    /// The compositor's end, for a caller that wants to poll it.
    pub(super) fn server(&self) -> &UnixStream {
        &self.server
    }

    /// Whether the compositor is still there.
    pub(super) fn is_open(&self) -> bool {
        !self.down.source_closed && !self.up.dead
    }

    /// Moves what is waiting in both directions; the number of bytes that moved.
    pub(super) fn relay(&mut self) -> usize {
        let pulled = self.pull(Way::Up) + self.pull(Way::Down);
        pulled + self.push(Way::Up) + self.push(Way::Down)
    }

    /// Queues raw bytes (and descriptors) for the compositor, as if the client had sent them.
    pub(super) fn inject(&mut self, bytes: Vec<u8>, fds: Vec<OwnedFd>) {
        self.scan(Way::Up, &bytes);
        self.up.out.push_back(Chunk {
            bytes,
            fds,
            sent: 0,
        });
    }

    fn lane(&mut self, way: Way) -> &mut Lane {
        match way {
            Way::Up => &mut self.up,
            Way::Down => &mut self.down,
        }
    }

    fn pull(&mut self, way: Way) -> usize {
        let mut moved = 0;
        for _ in 0..READS {
            if self.lane(way).source_closed {
                break;
            }
            let source = match way {
                Way::Up => &self.inner,
                Way::Down => &self.server,
            };
            match receive(source) {
                Ok(Some((bytes, fds))) if bytes.is_empty() && fds.is_empty() => {
                    self.lane(way).source_closed = true;
                }
                Ok(Some((bytes, fds))) => {
                    moved += bytes.len();
                    self.observe_all(way, &bytes);
                    self.lane(way).out.push_back(Chunk {
                        bytes,
                        fds,
                        sent: 0,
                    });
                }
                Ok(None) => break,
                Err(_) => self.lane(way).source_closed = true,
            }
        }
        moved
    }

    fn push(&mut self, way: Way) -> usize {
        let mut moved = 0;
        let sink = match way {
            Way::Up => &self.server,
            Way::Down => &self.inner,
        };
        let lane = match way {
            Way::Up => &mut self.up,
            Way::Down => &mut self.down,
        };
        while let Some(chunk) = lane.out.front_mut() {
            match send(sink, chunk) {
                Ok(0) => break,
                Ok(n) => {
                    moved += n;
                    chunk.sent += n;
                    chunk.fds.clear();
                    if chunk.sent == chunk.bytes.len() {
                        lane.out.pop_front();
                    }
                }
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => break,
                Err(_) => {
                    lane.dead = true;
                    lane.out.clear();
                    break;
                }
            }
        }
        if lane.out.is_empty() && lane.source_closed && !lane.shut {
            lane.shut = true;
            // The far end learns of the close; if it is gone already there is no one to tell.
            let _ = sink.shutdown(std::net::Shutdown::Write);
        }
        moved
    }

    /// Reads every whole message in `bytes` (after what the lane carried over).
    fn observe_all(&mut self, way: Way, bytes: &[u8]) {
        let mut stream = std::mem::take(&mut self.lane(way).carry);
        stream.extend_from_slice(bytes);
        let took = self.scan(way, &stream);
        stream.drain(..took);
        self.lane(way).carry = stream;
    }

    /// Observes each whole message at the start of `stream`; how many bytes they took.
    fn scan(&mut self, way: Way, stream: &[u8]) -> usize {
        let mut at = 0;
        while let Some((frame, took)) = wire::next_frame(&stream[at..]) {
            self.observe(way, &frame);
            at += took;
        }
        at
    }

    fn observe(&mut self, way: Way, frame: &Frame<'_>) {
        let Some(slot) = self.objects.get(frame.object).cloned() else {
            if way == Way::Down {
                self.record(frame, "?", &Undecoded::Bytes(frame.payload.len()));
            }
            return;
        };
        let described = slot.interface.and_then(|interface| {
            let table = match way {
                Way::Up => interface.requests,
                Way::Down => interface.events,
            };
            table.get(usize::from(frame.opcode))
        });
        let Some(desc) = described else {
            if way == Way::Down {
                self.record(frame, &slot.name, &Undecoded::Bytes(frame.payload.len()));
            }
            return;
        };
        let Ok(args) = wire::decode(desc, frame.payload) else {
            if way == Way::Down {
                self.record(frame, &slot.name, &Undecoded::Malformed);
            }
            return;
        };
        // `wl_registry.bind` declares no interface; the string before its new id names it.
        let declared: Option<String> =
            desc.child_interface.map(|i| i.name.to_owned()).or_else(|| {
                args.iter().find_map(|arg| match arg {
                    Arg::Str(Some(name)) => Some(name.clone()),
                    _ => None,
                })
            });
        if way == Way::Down {
            let objects = &self.objects;
            let shown: Vec<String> = args
                .iter()
                .map(|arg| wire::show(arg, declared.as_deref(), &|id| objects.name_of(id)))
                .collect();
            self.transcript.0.push(TranscriptLine {
                at: self.now,
                object: format!("{}#{}", slot.name, frame.object),
                event: desc.name.to_owned(),
                args: shown,
            });
        }
        if way == Way::Down && frame.object == 1 && desc.name == "error" {
            self.note_error(&args);
        }
        for arg in &args {
            if let Arg::NewId(id) = arg {
                self.objects.create(*id, declared.as_deref().unwrap_or("?"));
            }
        }
        match way {
            Way::Up if desc.is_destructor => self.objects.kill(frame.object),
            Way::Down if frame.object == 1 && desc.name == "delete_id" => {
                if let Some(Arg::Uint(id)) = args.first() {
                    self.objects.kill(*id);
                }
            }
            _ => {}
        }
    }

    /// Keeps the first `wl_display.error(object, code, message)`.
    fn note_error(&mut self, args: &[Arg]) {
        if let [Arg::Object(object), Arg::Uint(code), Arg::Str(message)] = args {
            let interface = self
                .objects
                .get(*object)
                .map_or_else(String::new, |s| s.name.clone());
            self.error.get_or_insert_with(|| ClientProtocolError {
                code: *code,
                interface,
                object: *object,
                message: message.clone().unwrap_or_default(),
            });
        }
    }

    fn record(&mut self, frame: &Frame<'_>, name: &str, what: &Undecoded) {
        let shown = match what {
            Undecoded::Bytes(n) => format!("bytes({n})"),
            Undecoded::Malformed => "malformed".to_owned(),
        };
        self.transcript.0.push(TranscriptLine {
            at: self.now,
            object: format!("{name}#{}", frame.object),
            event: format!("#{}", frame.opcode),
            args: vec![shown],
        });
    }
}

/// Why an event has no decoded arguments.
enum Undecoded {
    /// The interface or opcode is not in the catalogue.
    Bytes(usize),
    /// The payload does not fit the signature.
    Malformed,
}

/// One non-blocking read: what arrived and its descriptors; `Ok(None)` when nothing is there.
fn receive(from: &UnixStream) -> io::Result<Option<(Vec<u8>, Vec<OwnedFd>)>> {
    let mut buffer = vec![0u8; CHUNK];
    let mut space = [MaybeUninit::uninit(); rustix::cmsg_space!(ScmRights(MAX_FDS))];
    let mut control = RecvAncillaryBuffer::new(&mut space);
    let got = match recvmsg(
        from,
        &mut [IoSliceMut::new(&mut buffer)],
        &mut control,
        RecvFlags::CMSG_CLOEXEC,
    ) {
        Ok(got) => got,
        Err(rustix::io::Errno::AGAIN) => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    buffer.truncate(got.bytes);
    let mut fds = Vec::new();
    for message in control.drain() {
        if let RecvAncillaryMessage::ScmRights(received) = message {
            fds.extend(received);
        }
    }
    Ok(Some((buffer, fds)))
}

/// One non-blocking write of what is left of `chunk`, its descriptors on the first.
fn send(to: &UnixStream, chunk: &Chunk) -> io::Result<usize> {
    let mut space = [MaybeUninit::uninit(); rustix::cmsg_space!(ScmRights(MAX_FDS))];
    let mut control = SendAncillaryBuffer::new(&mut space);
    let borrowed: Vec<_> = chunk.fds.iter().map(AsFd::as_fd).collect();
    if !borrowed.is_empty() {
        control.push(SendAncillaryMessage::ScmRights(&borrowed));
    }
    let wrote = sendmsg(
        to,
        &[IoSlice::new(&chunk.bytes[chunk.sent..])],
        &mut control,
        SendFlags::NOSIGNAL,
    );
    match wrote {
        Ok(n) => Ok(n),
        Err(rustix::io::Errno::AGAIN) => Ok(0),
        Err(error) => Err(error.into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conn::testing::{message, read_all};
    use std::io::{Read, Write};

    /// A tap over one end of a pair; the other end is the "compositor".
    fn tapped() -> (Tap, UnixStream, UnixStream) {
        let (server_side, compositor) = UnixStream::pair().expect("pair");
        compositor.set_nonblocking(true).expect("nonblocking");
        let (tap, theirs) = Tap::new(server_side).expect("tap");
        (tap, compositor, theirs)
    }

    #[test]
    fn requests_pass_up_unchanged_and_new_ids_are_tracked() {
        let (mut tap, compositor, mut theirs) = tapped();
        // wl_display.get_registry(new_id 2), then wl_display.sync(new_id 3).
        let mut wire_bytes = message(1, 1, &2u32.to_ne_bytes());
        wire_bytes.extend(message(1, 0, &3u32.to_ne_bytes()));
        theirs.write_all(&wire_bytes).expect("write");
        assert_eq!(tap.relay(), wire_bytes.len() * 2);
        assert_eq!(read_all(&compositor), wire_bytes);
        assert_eq!(tap.objects.name_of(2), "wl_registry#2");
        assert_eq!(tap.objects.name_of(3), "wl_callback#3");
    }

    #[test]
    fn an_event_is_transcribed_with_the_time_it_was_relayed_at() {
        let (mut tap, mut compositor, mut theirs) = tapped();
        theirs
            .write_all(&message(1, 1, &2u32.to_ne_bytes()))
            .expect("get_registry");
        tap.relay();
        // wl_registry.global(name 1, "wl_shm", 2), split in two writes.
        let mut payload = 1u32.to_ne_bytes().to_vec();
        payload.extend(7u32.to_ne_bytes());
        payload.extend(b"wl_shm\0\0");
        payload.extend(2u32.to_ne_bytes());
        let event = message(2, 0, &payload);
        tap.now = Mono(std::time::Duration::from_millis(5));
        compositor.write_all(&event[..10]).expect("first half");
        tap.relay();
        assert!(
            tap.transcript.0.is_empty(),
            "half a message is not an event"
        );
        compositor.write_all(&event[10..]).expect("second half");
        tap.now = Mono(std::time::Duration::from_millis(9));
        tap.relay();
        assert_eq!(
            tap.transcript.text(),
            "9.000 wl_registry#2.global(1, \"wl_shm\", 2)\n"
        );
        assert_eq!(read_all(&theirs), event);
    }

    #[test]
    fn a_descriptor_travels_with_its_bytes() {
        let (mut tap, compositor, theirs) = tapped();
        let (probe, keep) = UnixStream::pair().expect("probe pair");
        let to_send = OwnedFd::from(probe);
        let chunk = Chunk {
            bytes: vec![1, 2, 3, 4],
            fds: vec![to_send],
            sent: 0,
        };
        send(&compositor, &chunk).expect("send with a descriptor");
        assert_eq!(tap.relay(), 4 + 4);
        let (bytes, fds) = receive(&theirs).expect("read").expect("something");
        assert_eq!(bytes, vec![1, 2, 3, 4]);
        assert_eq!(fds.len(), 1);
        drop(keep);
    }

    #[test]
    fn the_compositor_closing_reaches_the_connection() {
        let (mut tap, compositor, theirs) = tapped();
        drop(compositor);
        tap.relay();
        assert!(!tap.is_open());
        let mut buffer = [0u8; 8];
        assert_eq!((&theirs).read(&mut buffer).expect("eof, not an error"), 0);
    }

    #[test]
    fn an_unknown_object_or_opcode_does_not_derail_the_stream() {
        let (mut tap, mut compositor, theirs) = tapped();
        compositor
            .write_all(&message(77, 3, &[1, 2, 3, 4]))
            .expect("write");
        compositor.write_all(&message(1, 99, &[])).expect("write");
        tap.relay();
        assert_eq!(
            tap.transcript.text(),
            "0.000 ?#77.#3(bytes(4))\n0.000 wl_display#1.#99(bytes(0))\n"
        );
        assert_eq!(read_all(&theirs).len(), 12 + 8);
    }
}
