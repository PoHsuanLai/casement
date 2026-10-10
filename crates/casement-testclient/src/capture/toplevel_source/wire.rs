//! What a window capture session's queue dispatches into: the announced constraints, the buffer
//! in flight and the frame's outcome.

use super::super::CaptureFailure;
use super::{FORMATS, WindowFrame};
use std::fs::File;
use std::os::unix::fs::FileExt;
use wayland_client::protocol::wl_buffer::{self, WlBuffer};
use wayland_client::protocol::wl_shm::{self, Format, WlShm};
use wayland_client::protocol::wl_shm_pool::{self, WlShmPool};
use wayland_client::{Connection, Dispatch, QueueHandle, WEnum};
use wayland_protocols::ext::image_capture_source::v1::client::{
    ext_foreign_toplevel_image_capture_source_manager_v1::{
        self, ExtForeignToplevelImageCaptureSourceManagerV1,
    },
    ext_image_capture_source_v1::{self, ExtImageCaptureSourceV1},
};
use wayland_protocols::ext::image_copy_capture::v1::client::{
    ext_image_copy_capture_frame_v1::{self, ExtImageCopyCaptureFrameV1},
    ext_image_copy_capture_manager_v1::{self, ExtImageCopyCaptureManagerV1},
    ext_image_copy_capture_session_v1::{self, ExtImageCopyCaptureSessionV1},
};

/// The buffer a frame was asked into.
#[derive(Debug)]
pub(super) struct Pending {
    pub(super) file: File,
    pub(super) width: u32,
    pub(super) height: u32,
    pub(super) stride: u32,
    pub(super) format: Format,
    // Held so the compositor's copy of the buffer lives until the frame is answered.
    pub(super) _buffer: WlBuffer,
}

/// The session as the wire told it.
#[derive(Debug, Default)]
pub(super) struct Wire {
    /// The last `buffer_size`.
    pub(super) size: Option<(u32, u32)>,
    /// The `shm_format`s that came with it.
    formats: Vec<u32>,
    /// Whether the session was `stopped`.
    pub(super) stopped: bool,
    /// The buffer of the frame in flight.
    pub(super) buffer: Option<Pending>,
    /// How the last frame ended, until it is taken.
    pub(super) finished: Option<Result<WindowFrame, CaptureFailure>>,
}

impl Wire {
    /// The format a buffer should have: argb8888 when announced, else xrgb8888.
    pub(super) fn format(&self) -> Option<Format> {
        FORMATS
            .into_iter()
            .find(|format| self.formats.contains(&u32::from(*format)))
    }
}

/// A wire enum's number, known or not.
fn raw<T: Into<u32>>(value: WEnum<T>) -> u32 {
    match value {
        WEnum::Value(known) => known.into(),
        WEnum::Unknown(code) => code,
    }
}

/// The rows of R, G, B, A of `pending`'s buffer as the file holds it now.
fn read(pending: Pending) -> Option<WindowFrame> {
    let length = usize::try_from(u64::from(pending.stride) * u64::from(pending.height)).ok()?;
    let mut bytes = vec![0_u8; length];
    pending.file.read_exact_at(&mut bytes, 0).ok()?;
    let alpha = |a: u8| match pending.format {
        Format::Xrgb8888 => 255,
        _ => a,
    };
    let row = usize::try_from(pending.stride).ok()?;
    let wide = usize::try_from(pending.width).ok()?;
    let rgba = bytes
        .chunks(row.max(1))
        .flat_map(|line| line[..wide * 4].chunks(4))
        .flat_map(|p| [p[2], p[1], p[0], alpha(p[3])])
        .collect();
    Some(WindowFrame {
        width: pending.width,
        height: pending.height,
        rgba,
    })
}

impl Dispatch<ExtImageCopyCaptureSessionV1, ()> for Wire {
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
                state.size = Some((width, height));
                state.formats.clear();
            }
            Event::ShmFormat { format } => state.formats.push(raw(format)),
            Event::Stopped => state.stopped = true,
            _ => {}
        }
    }
}

impl Dispatch<ExtImageCopyCaptureFrameV1, ()> for Wire {
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
            Event::Ready => {
                state.finished = Some(
                    state
                        .buffer
                        .take()
                        .and_then(read)
                        .ok_or(CaptureFailure::Unknown),
                );
            }
            Event::Failed { reason } => {
                state.buffer = None;
                state.finished = Some(Err(match raw(reason) {
                    1 => CaptureFailure::BufferConstraints,
                    2 => CaptureFailure::Stopped,
                    _ => CaptureFailure::Unknown,
                }));
            }
            _ => {}
        }
    }
}

macro_rules! quiet {
    ($($proxy:ty => $event:ty),* $(,)?) => {$(
        impl Dispatch<$proxy, ()> for Wire {
            fn event(
                _: &mut Self,
                _: &$proxy,
                _: $event,
                _: &(),
                _: &Connection,
                _: &QueueHandle<Self>,
            ) {
            }
        }
    )*};
}

quiet! {
    ExtForeignToplevelImageCaptureSourceManagerV1 => ext_foreign_toplevel_image_capture_source_manager_v1::Event,
    ExtImageCaptureSourceV1 => ext_image_capture_source_v1::Event,
    ExtImageCopyCaptureManagerV1 => ext_image_copy_capture_manager_v1::Event,
    WlShm => wl_shm::Event,
    WlShmPool => wl_shm_pool::Event,
    WlBuffer => wl_buffer::Event,
}
