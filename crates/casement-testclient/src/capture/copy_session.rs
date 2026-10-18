//! `ext_image_copy_capture_v1` as a client: a session on a source, one frame into a shared
//! memory buffer, the events folded into a [`Frame`]. Filled by P1-C1.

mod cursor;

pub use cursor::CursorEvent;

/// A dma-buf format the session offered: the fourcc code and the modifiers allowed with it.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct DmabufFormat {
    /// The fourcc code.
    pub format: u32,
    /// The modifiers.
    pub modifiers: Vec<u64>,
}

use super::output_source::NamedOutput;
use crate::misc::idle::{pump_queue, side_queue};
use crate::{ClientError, GlobalList, Pumped};
use desktop_client::capture::{
    CaptureEvent, CaptureFailure, CaptureSession, CursorMode, ShmFormat,
};
use desktop_client::output::Transform;
use desktop_client::{CaptureId, DesktopEvent, DesktopState, Frame, Rect, Size, reduce};
use rustix::fs::{MemfdFlags, ftruncate, memfd_create};
use std::fs::File;
use std::os::fd::AsFd;
use std::os::unix::fs::FileExt;
use std::time::Duration;
use wayland_client::protocol::wl_buffer::{self, WlBuffer};
use wayland_client::protocol::wl_seat::WlSeat;
use wayland_client::protocol::wl_shm::{self, Format, WlShm};
use wayland_client::protocol::wl_shm_pool::{self, WlShmPool};
use wayland_client::{Connection, Dispatch, EventQueue, QueueHandle, WEnum};
use wayland_protocols::ext::image_capture_source::v1::client::{
    ext_image_capture_source_v1::ExtImageCaptureSourceV1,
    ext_output_image_capture_source_manager_v1::ExtOutputImageCaptureSourceManagerV1,
};
use wayland_protocols::ext::image_copy_capture::v1::client::{
    ext_image_copy_capture_cursor_session_v1::ExtImageCopyCaptureCursorSessionV1,
    ext_image_copy_capture_frame_v1::{self, ExtImageCopyCaptureFrameV1},
    ext_image_copy_capture_manager_v1::{self, ExtImageCopyCaptureManagerV1, Options},
    ext_image_copy_capture_session_v1::{self, ExtImageCopyCaptureSessionV1},
};

/// The id the one capture of a client has in the reducer.
const ID: CaptureId = CaptureId::new(1);

/// What the queue dispatches into: the outputs, the reduced capture and the buffer in flight.
#[derive(Debug, Default)]
pub(super) struct Capturing {
    pub(super) outputs: Vec<NamedOutput>,
    desktop: DesktopState,
    buffer: Option<Pending>,
    stopped: bool,
    cursor_events: Vec<CursorEvent>,
    announced: Announced,
}

/// The session's last complete constraints. The reducer folds one frame per capture and
/// forgets the session once its frame is taken; a later frame on the same session starts
/// from these.
#[derive(Debug, Default)]
struct Announced {
    size: Option<Size>,
    formats: Vec<ShmFormat>,
    dmabuf: Vec<DmabufFormat>,
    device: Option<u64>,
    done: Done,
}

/// Whether the constraints round is complete.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum Done {
    #[default]
    Open,
    Complete,
}

impl Announced {
    /// A constraint arrived: one after a `done` starts a new round.
    fn round(&mut self) {
        if self.done == Done::Complete {
            self.formats.clear();
            self.dmabuf.clear();
            self.device = None;
            self.done = Done::Open;
        }
    }
}

/// The buffer a frame was asked into.
#[derive(Debug)]
struct Pending {
    file: File,
    stride: u32,
    rows: u32,
    // Held so the compositor's copy of the buffer lives until the frame is answered.
    _buffer: WlBuffer,
}

impl Capturing {
    fn fold(&mut self, event: CaptureEvent) {
        let state = std::mem::take(&mut self.desktop);
        self.desktop = reduce(state, DesktopEvent::Capture { id: ID, event });
    }

    /// Starts the next frame's capture from the remembered constraints, if the last one was
    /// taken and the session announced complete constraints.
    fn reopen(&mut self) {
        if self.desktop.captures.get(ID).is_some() || self.announced.done == Done::Open {
            return;
        }
        if let Some(size) = self.announced.size {
            self.fold(CaptureEvent::BufferSize(size));
        }
        for format in self.announced.formats.clone() {
            self.fold(CaptureEvent::Format(format));
        }
        self.fold(CaptureEvent::ConstraintsDone);
    }
}

/// What buffer to attach to the next frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum BufferAsk {
    /// The size and format the session announced.
    Announced,
    /// An `argb8888` buffer of this size, whatever the session announced (to test the
    /// compositor's refusal).
    Sized(Size),
    /// The buffer the last request attached, attached again to a new frame (what a screen
    /// share does: one buffer, a frame after frame). Refused before any request.
    Reuse,
}

/// A buffer a frame was asked into, kept so that a later request can attach it again.
#[derive(Debug)]
struct Kept {
    file: File,
    buffer: WlBuffer,
    size: Size,
}

/// One capture session of one output, on a side queue of a test client.
#[derive(Debug)]
pub struct OutputCapture {
    connection: Connection,
    queue: EventQueue<Capturing>,
    state: Capturing,
    shm: WlShm,
    copies: ExtImageCopyCaptureManagerV1,
    sources: ExtOutputImageCaptureSourceManagerV1,
    seat: Option<WlSeat>,
    live: Option<Live>,
    kept: Option<Kept>,
}

/// The protocol objects of the open session.
#[derive(Debug)]
struct Live {
    source: ExtImageCaptureSourceV1,
    cursor: Option<ExtImageCopyCaptureCursorSessionV1>,
    session: ExtImageCopyCaptureSessionV1,
    frame: Option<ExtImageCopyCaptureFrameV1>,
}

impl OutputCapture {
    /// Binds the two managers, `wl_shm` and every `wl_output`. The outputs' names arrive at the
    /// fixture's next pump; the requests are flushed.
    pub fn start(globals: &GlobalList) -> Result<OutputCapture, ClientError> {
        let (connection, queue) = side_queue::<Capturing>(globals)?;
        let qh = queue.handle();
        let missing = |name: &str| ClientError::MissingGlobal(name.to_owned());
        let sources = globals
            .bind::<ExtOutputImageCaptureSourceManagerV1, _, _>(&qh, 1..=1, ())
            .map_err(|_| missing("ext_output_image_capture_source_manager_v1"))?;
        let copies = globals
            .bind::<ExtImageCopyCaptureManagerV1, _, _>(&qh, 1..=1, ())
            .map_err(|_| missing("ext_image_copy_capture_manager_v1"))?;
        let shm = globals
            .bind::<WlShm, _, _>(&qh, 1..=1, ())
            .map_err(|_| missing("wl_shm"))?;
        let seat = globals.bind::<WlSeat, _, _>(&qh, 1..=1, ()).ok();
        let mut state = Capturing::default();
        for global in globals.contents().clone_list() {
            if global.interface == "wl_output" {
                let output = globals
                    .registry()
                    .bind::<wayland_client::protocol::wl_output::WlOutput, _, _>(
                        global.name,
                        global.version.min(4),
                        &qh,
                        global.name,
                    );
                state.outputs.push(NamedOutput {
                    global: global.name,
                    output,
                    name: None,
                });
            }
        }
        flush(&connection)?;
        Ok(OutputCapture {
            connection,
            queue,
            state,
            shm,
            copies,
            sources,
            seat,
            live: None,
            kept: None,
        })
    }

    /// Reads what arrived.
    pub fn pump(&mut self) -> Result<Pumped, ClientError> {
        pump_queue(&self.connection, &mut self.queue, &mut self.state)
    }

    /// The connector names of the outputs, once they have arrived.
    pub fn output_names(&self) -> Vec<String> {
        self.state
            .outputs
            .iter()
            .filter_map(|output| output.name.clone())
            .collect()
    }

    /// Makes a source of the output named `connector` and a session on it.
    pub fn open(&mut self, connector: &str, cursor: CursorMode) -> Result<(), ClientError> {
        let output = self
            .state
            .outputs
            .iter()
            .find(|output| output.name.as_deref() == Some(connector))
            .map(|output| output.output.clone())
            .ok_or_else(|| ClientError::MissingGlobal(format!("wl_output {connector}")))?;
        let qh = self.queue.handle();
        let source = self.sources.create_source(&output, &qh, ());
        let options = match cursor {
            CursorMode::Painted => Options::PaintCursors,
            _ => Options::empty(),
        };
        let session = self.copies.create_session(&source, options, &qh, ());
        self.live = Some(Live {
            source,
            cursor: None,
            session,
            frame: None,
        });
        flush(&self.connection)
    }

    /// Makes a source of the Space `space` of the output named `connector`, from the shell's
    /// `get_space_capture_source` (`width` by `height` pixels), and a session on it.
    pub fn open_space(
        &mut self,
        shell: &casement_protocols::client::quire_shell_v1::quire_shell_v1::QuireShellV1,
        connector: &str,
        space: &str,
        size: Size,
    ) -> Result<(), ClientError> {
        let output = self
            .state
            .outputs
            .iter()
            .find(|output| output.name.as_deref() == Some(connector))
            .map(|output| output.output.clone())
            .ok_or_else(|| ClientError::MissingGlobal(format!("wl_output {connector}")))?;
        let qh = self.queue.handle();
        let source =
            shell.get_space_capture_source(&output, space.to_owned(), size.w, size.h, &qh, ());
        let session = self
            .copies
            .create_session(&source, Options::empty(), &qh, ());
        self.live = Some(Live {
            source,
            cursor: None,
            session,
            frame: None,
        });
        flush(&self.connection)
    }

    /// The dma-buf formats the session offered in its last complete round of constraints.
    pub fn dmabuf_formats(&self) -> &[DmabufFormat] {
        &self.state.announced.dmabuf
    }

    /// The device the session said dma-bufs are allocated on (`dev_t`), if it offered any.
    pub fn dmabuf_device(&self) -> Option<u64> {
        self.state.announced.device
    }

    /// The session, as the reducer has it: its size, formats and phase.
    pub fn session(&self) -> Option<&CaptureSession> {
        self.state.desktop.captures.get(ID)
    }

    /// Asks for a frame into a buffer of `ask`. The session's constraints must have arrived.
    pub fn request(&mut self, ask: BufferAsk) -> Result<(), ClientError> {
        self.state.reopen();
        let kept = match ask {
            BufferAsk::Reuse => self.kept.take().ok_or_else(|| {
                ClientError::Dispatch("no buffer has been attached yet".to_owned())
            })?,
            other => self.allocate(other)?,
        };
        let qh = self.queue.handle();
        let (width, height) = (fits(kept.size.w)?, fits(kept.size.h)?);
        let file = kept
            .file
            .try_clone()
            .map_err(|error| ClientError::Dispatch(error.to_string()))?;
        let live = self
            .live
            .as_mut()
            .ok_or_else(|| ClientError::Dispatch("no session is open".to_owned()))?;
        // A session has one frame at a time: the last one goes before the next is made.
        if let Some(last) = live.frame.take() {
            last.destroy();
        }
        let frame = live.session.create_frame(&qh, ());
        frame.attach_buffer(&kept.buffer);
        frame.damage_buffer(0, 0, width, height);
        frame.capture();
        live.frame = Some(frame);
        self.state.buffer = Some(Pending {
            file,
            stride: kept.size.w.saturating_mul(4),
            rows: kept.size.h,
            _buffer: kept.buffer.clone(),
        });
        self.kept = Some(kept);
        flush(&self.connection)
    }

    /// A new shared-memory buffer for `ask` (not [`BufferAsk::Reuse`]).
    fn allocate(&self, ask: BufferAsk) -> Result<Kept, ClientError> {
        let session = self
            .session()
            .ok_or_else(|| ClientError::Dispatch("no session is open".to_owned()))?;
        let (size, format) = match ask {
            BufferAsk::Announced => (
                session.size,
                ShmFormat::pick(&session.formats).map(ShmFormat::code),
            ),
            BufferAsk::Sized(size) => (Some(size), Some(ShmFormat::ARGB8888.code())),
            BufferAsk::Reuse => (None, None),
        };
        let (Some(size), Some(code)) = (size, format) else {
            return Err(ClientError::Dispatch(
                "the session announced no usable buffer".to_owned(),
            ));
        };
        let format = Format::try_from(code)
            .map_err(|_| ClientError::Dispatch(format!("shm format {code}")))?;
        let stride = size.w.saturating_mul(4);
        let length = u64::from(stride) * u64::from(size.h);
        let file = File::from(
            memfd_create("casement-capture", MemfdFlags::CLOEXEC)
                .map_err(|error| ClientError::Dispatch(error.to_string()))?,
        );
        ftruncate(&file, length).map_err(|error| ClientError::Dispatch(error.to_string()))?;
        let qh = self.queue.handle();
        let pool = self.shm.create_pool(
            file.as_fd(),
            fits(u32::try_from(length).unwrap_or(u32::MAX))?,
            &qh,
            (),
        );
        let buffer = pool.create_buffer(
            0,
            fits(size.w)?,
            fits(size.h)?,
            fits(stride)?,
            format,
            &qh,
            (),
        );
        pool.destroy();
        Ok(Kept { file, buffer, size })
    }

    /// The finished frame, or why it failed; `None` while it is still going. Takes it.
    pub fn take(&mut self) -> Option<Result<Frame, CaptureFailure>> {
        let captures = std::mem::take(&mut self.state.desktop.captures);
        let (captures, ready) = captures.take_ready(ID);
        self.state.desktop.captures = captures;
        ready
    }

    /// Whether the compositor ended the session (`stopped`: its output went away).
    pub fn is_stopped(&self) -> bool {
        self.state.stopped
    }

    /// Destroys the session and its source.
    pub fn close(&mut self) -> Result<(), ClientError> {
        if let Some(live) = self.live.take() {
            if let Some(frame) = live.frame {
                frame.destroy();
            }
            live.session.destroy();
            if let Some(cursor) = live.cursor {
                cursor.destroy();
            }
            live.source.destroy();
        }
        flush(&self.connection)
    }
}

fn flush(connection: &Connection) -> Result<(), ClientError> {
    connection
        .flush()
        .map_err(|error| ClientError::Dispatch(error.to_string()))
}

/// `n` as a wire integer.
fn fits(n: u32) -> Result<i32, ClientError> {
    i32::try_from(n).map_err(|_| ClientError::Dispatch("buffer too large".to_owned()))
}

/// A wire enum's number, known or not.
fn raw<T: Into<u32>>(value: WEnum<T>) -> u32 {
    match value {
        WEnum::Value(known) => known.into(),
        WEnum::Unknown(code) => code,
    }
}

impl Dispatch<ExtImageCopyCaptureManagerV1, ()> for Capturing {
    fn event(
        _: &mut Self,
        _: &ExtImageCopyCaptureManagerV1,
        _: ext_image_copy_capture_manager_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ExtImageCopyCaptureSessionV1, ()> for Capturing {
    fn event(
        state: &mut Self,
        _: &ExtImageCopyCaptureSessionV1,
        event: ext_image_copy_capture_session_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        use ext_image_copy_capture_session_v1::Event;
        match event {
            Event::BufferSize { width, height } => {
                let size = Size::new(width, height);
                state.announced.round();
                state.announced.size = Some(size);
                state.fold(CaptureEvent::BufferSize(size));
            }
            Event::ShmFormat { format } => {
                let format = ShmFormat::from_code(raw(format));
                state.announced.round();
                if !state.announced.formats.contains(&format) {
                    state.announced.formats.push(format);
                }
                state.fold(CaptureEvent::Format(format));
            }
            Event::DmabufDevice { device } => {
                state.announced.round();
                let bytes = device.as_slice().first_chunk::<8>().copied();
                state.announced.device = bytes.map(u64::from_ne_bytes);
            }
            Event::DmabufFormat { format, modifiers } => {
                state.announced.round();
                let modifiers = modifiers
                    .as_chunks::<8>()
                    .0
                    .iter()
                    .map(|chunk| u64::from_ne_bytes(*chunk))
                    .collect();
                state
                    .announced
                    .dmabuf
                    .push(DmabufFormat { format, modifiers });
            }
            Event::Done => {
                state.announced.done = Done::Complete;
                state.fold(CaptureEvent::ConstraintsDone);
            }
            Event::Stopped => {
                state.stopped = true;
                state.fold(CaptureEvent::Failed(CaptureFailure::Stopped));
            }
            _ => {}
        }
    }
}

impl Dispatch<ExtImageCopyCaptureFrameV1, ()> for Capturing {
    fn event(
        state: &mut Self,
        _: &ExtImageCopyCaptureFrameV1,
        event: ext_image_copy_capture_frame_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        use ext_image_copy_capture_frame_v1::Event;
        match event {
            Event::Transform { transform } => {
                if let Some(transform) = Transform::from_code(raw(transform)) {
                    state.fold(CaptureEvent::Transform(transform));
                }
            }
            Event::Damage {
                x,
                y,
                width,
                height,
            } => {
                let extent = |n: i32| u32::try_from(n).unwrap_or(0);
                state.fold(CaptureEvent::Damage(Rect::new(
                    x,
                    y,
                    extent(width),
                    extent(height),
                )));
            }
            Event::PresentationTime {
                tv_sec_hi,
                tv_sec_lo,
                tv_nsec,
            } => {
                let seconds = (u64::from(tv_sec_hi) << 32) | u64::from(tv_sec_lo);
                state.fold(CaptureEvent::PresentationTime(Duration::new(
                    seconds, tv_nsec,
                )));
            }
            Event::Ready => {
                let read = state.buffer.take().and_then(|pending| {
                    let length =
                        usize::try_from(u64::from(pending.stride) * u64::from(pending.rows))
                            .ok()?;
                    let mut pixels = vec![0_u8; length];
                    pending.file.read_exact_at(&mut pixels, 0).ok()?;
                    Some((pending.stride, pixels))
                });
                state.fold(match read {
                    Some((stride, pixels)) => CaptureEvent::Ready { stride, pixels },
                    None => CaptureEvent::Failed(CaptureFailure::Unknown),
                });
            }
            Event::Failed { reason } => {
                let reason = match raw(reason) {
                    1 => CaptureFailure::BufferConstraints,
                    2 => CaptureFailure::Stopped,
                    _ => CaptureFailure::Unknown,
                };
                state.buffer = None;
                state.fold(CaptureEvent::Failed(reason));
            }
            _ => {}
        }
    }
}

impl Dispatch<WlShm, ()> for Capturing {
    fn event(
        _: &mut Self,
        _: &WlShm,
        _: wl_shm::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<WlShmPool, ()> for Capturing {
    fn event(
        _: &mut Self,
        _: &WlShmPool,
        _: wl_shm_pool::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<WlBuffer, ()> for Capturing {
    fn event(
        _: &mut Self,
        _: &WlBuffer,
        _: wl_buffer::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}
