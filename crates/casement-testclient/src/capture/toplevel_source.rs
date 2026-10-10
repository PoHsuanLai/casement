//! `ext_foreign_toplevel_image_capture_source_manager_v1` as a client (P2-Q3): a source made from
//! a foreign toplevel handle, then a copy session on it and frames into shared-memory buffers.
//!
//! [`WindowCapture`] is the client's `ext_foreign_toplevel_list_v1` (for the handles) beside the
//! copy session. It does not fold events with `desktop_client`'s reducer as [`super::OutputCapture`]
//! does: that reducer ends a capture at its first frame, and window capture is tested over many
//! frames and across constraint changes, so the session here is kept as the wire says it.

mod wire;

use crate::manage::foreign_toplevel_list::{Listed, ToplevelList};
use crate::misc::idle::{pump_queue, side_queue};
use crate::{ClientError, GlobalList, Pumped};
use rustix::fs::{MemfdFlags, ftruncate, memfd_create};
use std::fs::File;
use std::os::fd::AsFd;
use wayland_client::protocol::wl_shm::{Format, WlShm};
use wayland_client::{Connection, EventQueue};
use wayland_protocols::ext::image_capture_source::v1::client::{
    ext_foreign_toplevel_image_capture_source_manager_v1::ExtForeignToplevelImageCaptureSourceManagerV1,
    ext_image_capture_source_v1::ExtImageCaptureSourceV1,
};
use wayland_protocols::ext::image_copy_capture::v1::client::{
    ext_image_copy_capture_frame_v1::ExtImageCopyCaptureFrameV1,
    ext_image_copy_capture_manager_v1::{ExtImageCopyCaptureManagerV1, Options},
    ext_image_copy_capture_session_v1::ExtImageCopyCaptureSessionV1,
};
use wire::{Pending, Wire};

pub use super::{CaptureFailure, CursorMode};

/// A finished frame, as rows of R, G, B, A bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowFrame {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// `width * height * 4` bytes.
    pub rgba: Vec<u8>,
}

/// The protocol objects of the open session.
#[derive(Debug)]
struct Live {
    source: ExtImageCaptureSourceV1,
    session: ExtImageCopyCaptureSessionV1,
    frame: Option<ExtImageCopyCaptureFrameV1>,
}

/// One capture session of one window, on a side queue of a test client.
#[derive(Debug)]
pub struct WindowCapture {
    connection: Connection,
    queue: EventQueue<Wire>,
    wire: Wire,
    list: ToplevelList,
    shm: WlShm,
    copies: ExtImageCopyCaptureManagerV1,
    sources: ExtForeignToplevelImageCaptureSourceManagerV1,
    live: Option<Live>,
}

impl WindowCapture {
    /// Binds the window list, the two managers and `wl_shm`. The windows arrive at the
    /// fixture's next pump.
    pub fn start(globals: &GlobalList) -> Result<WindowCapture, ClientError> {
        let (connection, queue) = side_queue::<Wire>(globals)?;
        let qh = queue.handle();
        let missing = |name: &str| ClientError::MissingGlobal(name.to_owned());
        let sources = globals
            .bind::<ExtForeignToplevelImageCaptureSourceManagerV1, _, _>(&qh, 1..=1, ())
            .map_err(|_| missing("ext_foreign_toplevel_image_capture_source_manager_v1"))?;
        let copies = globals
            .bind::<ExtImageCopyCaptureManagerV1, _, _>(&qh, 1..=1, ())
            .map_err(|_| missing("ext_image_copy_capture_manager_v1"))?;
        let shm = globals
            .bind::<WlShm, _, _>(&qh, 1..=1, ())
            .map_err(|_| missing("wl_shm"))?;
        let list = ToplevelList::start(globals)?;
        flush(&connection)?;
        Ok(WindowCapture {
            connection,
            queue,
            wire: Wire::default(),
            list,
            shm,
            copies,
            sources,
            live: None,
        })
    }

    /// Reads what arrived on both queues.
    pub fn pump(&mut self) -> Result<Pumped, ClientError> {
        let list = self.list.pump()?;
        let capture = pump_queue(&self.connection, &mut self.queue, &mut self.wire)?;
        Ok(match (list, capture) {
            (Pumped::Quiet, Pumped::Quiet) => Pumped::Quiet,
            _ => Pumped::Moved,
        })
    }

    /// The windows the list announced, in order.
    pub fn windows(&self) -> Vec<Listed> {
        self.list.listed()
    }

    /// Makes a source of the window announced as `identifier` and a session on it.
    pub fn open(&mut self, identifier: &str, cursor: CursorMode) -> Result<(), ClientError> {
        let handle = self
            .list
            .handle_of(identifier)
            .ok_or_else(|| ClientError::MissingGlobal(format!("a window {identifier}")))?;
        let qh = self.queue.handle();
        let source = self.sources.create_source(&handle, &qh, ());
        let options = match cursor {
            CursorMode::Painted => Options::PaintCursors,
            _ => Options::empty(),
        };
        let session = self.copies.create_session(&source, options, &qh, ());
        self.live = Some(Live {
            source,
            session,
            frame: None,
        });
        flush(&self.connection)
    }

    /// The buffer size the session announced last, in pixels.
    pub fn size(&self) -> Option<(u32, u32)> {
        self.wire.size
    }

    /// Asks for a frame into a buffer of the size and format the session announced. The
    /// previous frame object is destroyed first (the protocol allows one at a time).
    pub fn request(&mut self) -> Result<(), ClientError> {
        let dispatch = |text: &str| ClientError::Dispatch(text.to_owned());
        let (width, height) = self
            .wire
            .size
            .ok_or_else(|| dispatch("no size announced"))?;
        let format = self
            .wire
            .format()
            .ok_or_else(|| dispatch("no usable format"))?;
        let stride = width.saturating_mul(4);
        let length = u64::from(stride) * u64::from(height);
        let file = File::from(
            memfd_create("casement-window-capture", MemfdFlags::CLOEXEC)
                .map_err(|error| ClientError::Dispatch(error.to_string()))?,
        );
        ftruncate(&file, length).map_err(|error| ClientError::Dispatch(error.to_string()))?;
        let qh = self.queue.handle();
        let fits = |n: u64| i32::try_from(n).map_err(|_| dispatch("buffer too large"));
        let pool = self.shm.create_pool(file.as_fd(), fits(length)?, &qh, ());
        let buffer = pool.create_buffer(
            0,
            fits(u64::from(width))?,
            fits(u64::from(height))?,
            fits(u64::from(stride))?,
            format,
            &qh,
            (),
        );
        pool.destroy();
        let live = self
            .live
            .as_mut()
            .ok_or_else(|| dispatch("no session is open"))?;
        if let Some(old) = live.frame.take() {
            old.destroy();
        }
        let frame = live.session.create_frame(&qh, ());
        frame.attach_buffer(&buffer);
        frame.damage_buffer(0, 0, fits(u64::from(width))?, fits(u64::from(height))?);
        frame.capture();
        live.frame = Some(frame);
        self.wire.finished = None;
        self.wire.buffer = Some(Pending {
            file,
            width,
            height,
            stride,
            format,
            _buffer: buffer,
        });
        flush(&self.connection)
    }

    /// The finished frame, or why it failed; `None` while it is still going. Takes it.
    pub fn take(&mut self) -> Option<Result<WindowFrame, CaptureFailure>> {
        self.wire.finished.take()
    }

    /// Whether the compositor ended the session (`stopped`: its window closed).
    pub fn is_stopped(&self) -> bool {
        self.wire.stopped
    }

    /// Destroys the session and its source.
    pub fn close(&mut self) -> Result<(), ClientError> {
        if let Some(live) = self.live.take() {
            if let Some(frame) = live.frame {
                frame.destroy();
            }
            live.session.destroy();
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

/// The shared-memory formats a capture buffer can have.
pub(super) const FORMATS: [Format; 2] = [Format::Argb8888, Format::Xrgb8888];
