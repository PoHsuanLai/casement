//! `zwlr_screencopy_manager_v1` as a client (what `grim` speaks): one frame of an output, or of
//! a region of it, into a shared-memory buffer. One [`ScreenCopy`] is one frame at a time, on
//! its own queue next to the client's. Filled by the capture lane of wave 2026-10-15.

use crate::misc::idle::{pump_queue, side_queue};
use crate::{ClientError, GlobalList, Pumped};
use desktop_client::Size;
use rustix::fs::{MemfdFlags, ftruncate, memfd_create};
use std::fs::File;
use std::os::fd::AsFd;
use std::os::unix::fs::FileExt;
use wayland_client::protocol::wl_buffer::{self, WlBuffer};
use wayland_client::protocol::wl_output::{self, WlOutput};
use wayland_client::protocol::wl_shm::{self, Format, WlShm};
use wayland_client::protocol::wl_shm_pool::{self, WlShmPool};
use wayland_client::{Connection, Dispatch, EventQueue, QueueHandle, WEnum};
use wayland_protocols_wlr::screencopy::v1::client::{
    zwlr_screencopy_frame_v1::{self, ZwlrScreencopyFrameV1},
    zwlr_screencopy_manager_v1::{self, ZwlrScreencopyManagerV1},
};

/// How much of the output to ask for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum CopyArea {
    /// All of it.
    Output,
    /// This output-local logical rectangle: `(x, y, width, height)` (`grim -g`).
    Region((i32, i32, i32, i32)),
}

/// Whether the copy asks for damage (`copy_with_damage`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum CopyKind {
    /// `copy`.
    Plain,
    /// `copy_with_damage`.
    WithDamage,
}

/// A buffer the frame offered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Offer {
    /// The `wl_shm` format code.
    pub format: u32,
    /// Its size in pixels.
    pub size: Size,
    /// Bytes per row.
    pub stride: u32,
}

/// Why a copy failed.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum CopyFailure {
    /// The compositor sent `failed`.
    Failed,
    /// The compositor closed the connection on a protocol error.
    Protocol,
}

/// A finished copy.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Copied {
    /// Its size in pixels.
    pub size: Size,
    /// Bytes per row of `bytes`.
    pub stride: u32,
    /// The pixels as the buffer held them.
    pub bytes: Vec<u8>,
    /// The damage the frame reported, `(x, y, width, height)`, if any.
    pub damage: Option<(u32, u32, u32, u32)>,
    /// The flags the frame carried.
    pub flags: u32,
}

impl Copied {
    /// The pixels as tightly packed R, G, B, A bytes (the buffer was `argb8888` or
    /// `xrgb8888`: little-endian B, G, R, A or X).
    pub fn to_rgba8(&self) -> Vec<u8> {
        let row = usize::try_from(self.size.w).unwrap_or(0).saturating_mul(4);
        let stride = usize::try_from(self.stride).unwrap_or(row);
        let rows = usize::try_from(self.size.h).unwrap_or(0);
        (0..rows)
            .flat_map(|y| {
                let from = y.saturating_mul(stride);
                self.bytes
                    .get(from..from.saturating_add(row))
                    .unwrap_or(&[])
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .flat_map(|&[b, g, r, a]| [r, g, b, a])
                    .collect::<Vec<u8>>()
            })
            .collect()
    }
}

/// What the queue dispatches into.
#[derive(Debug, Default)]
struct Copying {
    outputs: Vec<(u32, WlOutput, Option<String>)>,
    offers: Vec<Offer>,
    file: Option<File>,
    flags: u32,
    damage: Option<(u32, u32, u32, u32)>,
    done: Option<Result<(), CopyFailure>>,
    buffer: Option<(Offer, WlBuffer)>,
}

/// One frame of an output, through `zwlr_screencopy_manager_v1`.
#[derive(Debug)]
pub struct ScreenCopy {
    connection: Connection,
    queue: EventQueue<Copying>,
    state: Copying,
    shm: WlShm,
    manager: ZwlrScreencopyManagerV1,
    frame: Option<ZwlrScreencopyFrameV1>,
}

impl ScreenCopy {
    /// Binds the manager (up to version 3), `wl_shm` and every `wl_output`. The outputs' names
    /// arrive at the fixture's next pump.
    ///
    /// ```no_run
    /// use casement_testclient::GlobalList;
    /// use casement_testclient::capture::ScreenCopy;
    ///
    /// # fn demo(globals: &GlobalList) {
    /// let copy = ScreenCopy::start(globals);
    /// # let _ = copy;
    /// # }
    /// ```
    pub fn start(globals: &GlobalList) -> Result<ScreenCopy, ClientError> {
        let (connection, queue) = side_queue::<Copying>(globals)?;
        let qh = queue.handle();
        let missing = |name: &str| ClientError::MissingGlobal(name.to_owned());
        let manager = globals
            .bind::<ZwlrScreencopyManagerV1, _, _>(&qh, 1..=3, ())
            .map_err(|_| missing("zwlr_screencopy_manager_v1"))?;
        let shm = globals
            .bind::<WlShm, _, _>(&qh, 1..=1, ())
            .map_err(|_| missing("wl_shm"))?;
        let mut state = Copying::default();
        for global in globals.contents().clone_list() {
            if global.interface == "wl_output" {
                let output = globals.registry().bind::<WlOutput, _, _>(
                    global.name,
                    global.version.min(4),
                    &qh,
                    global.name,
                );
                state.outputs.push((global.name, output, None));
            }
        }
        flush(&connection)?;
        Ok(ScreenCopy {
            connection,
            queue,
            state,
            shm,
            manager,
            frame: None,
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
            .filter_map(|(_, _, name)| name.clone())
            .collect()
    }

    /// Asks for a frame of the output named `connector`. The offered buffers arrive at the
    /// next pumps ([`ScreenCopy::offers`]).
    pub fn capture(&mut self, connector: &str, area: CopyArea) -> Result<(), ClientError> {
        let output = self
            .state
            .outputs
            .iter()
            .find(|(_, _, name)| name.as_deref() == Some(connector))
            .map(|(_, output, _)| output.clone())
            .ok_or_else(|| ClientError::MissingGlobal(format!("wl_output {connector}")))?;
        let qh = self.queue.handle();
        self.state.offers.clear();
        self.state.done = None;
        self.state.damage = None;
        let frame = match area {
            CopyArea::Output => self.manager.capture_output(0, &output, &qh, ()),
            CopyArea::Region((x, y, w, h)) => {
                self.manager
                    .capture_output_region(0, &output, x, y, w, h, &qh, ())
            }
        };
        self.frame = Some(frame);
        flush(&self.connection)
    }

    /// The buffers the frame offered so far.
    pub fn offers(&self) -> &[Offer] {
        &self.state.offers
    }

    /// Copies the frame into a buffer made to `offer`, with `kind`.
    pub fn copy(&mut self, offer: Offer, kind: CopyKind) -> Result<(), ClientError> {
        let format = Format::try_from(offer.format)
            .map_err(|_| ClientError::Dispatch(format!("shm format {}", offer.format)))?;
        let length = u64::from(offer.stride) * u64::from(offer.size.h);
        let file = File::from(
            memfd_create("casement-screencopy", MemfdFlags::CLOEXEC)
                .map_err(|error| ClientError::Dispatch(error.to_string()))?,
        );
        ftruncate(&file, length).map_err(|error| ClientError::Dispatch(error.to_string()))?;
        let qh = self.queue.handle();
        let fits = |n: u32| {
            i32::try_from(n).map_err(|_| ClientError::Dispatch("buffer too large".to_owned()))
        };
        let pool = self.shm.create_pool(
            file.as_fd(),
            fits(u32::try_from(length).unwrap_or(u32::MAX))?,
            &qh,
            (),
        );
        let buffer = pool.create_buffer(
            0,
            fits(offer.size.w)?,
            fits(offer.size.h)?,
            fits(offer.stride)?,
            format,
            &qh,
            (),
        );
        pool.destroy();
        let frame = self
            .frame
            .as_ref()
            .ok_or_else(|| ClientError::Dispatch("no frame was asked for".to_owned()))?;
        match kind {
            CopyKind::Plain => frame.copy(&buffer),
            CopyKind::WithDamage => frame.copy_with_damage(&buffer),
        }
        self.state.file = Some(file);
        self.state.buffer = Some((offer, buffer));
        flush(&self.connection)
    }

    /// The finished copy, or why it failed; `None` while it is still going. Takes it.
    pub fn take(&mut self) -> Option<Result<Copied, CopyFailure>> {
        let done = self.state.done.take()?;
        if let Err(why) = done {
            self.state.buffer = None;
            self.state.file = None;
            return Some(Err(why));
        }
        let (offer, _buffer) = self.state.buffer.take()?;
        let file = self.state.file.take()?;
        let length = usize::try_from(u64::from(offer.stride) * u64::from(offer.size.h)).ok()?;
        let mut bytes = vec![0_u8; length];
        let read = file.read_exact_at(&mut bytes, 0);
        Some(read.map_err(|_| CopyFailure::Failed).map(|()| Copied {
            size: offer.size,
            stride: offer.stride,
            bytes,
            damage: self.state.damage,
            flags: self.state.flags,
        }))
    }

    /// Destroys the frame.
    pub fn close(&mut self) -> Result<(), ClientError> {
        if let Some(frame) = self.frame.take() {
            frame.destroy();
        }
        flush(&self.connection)
    }
}

fn flush(connection: &Connection) -> Result<(), ClientError> {
    connection
        .flush()
        .map_err(|error| ClientError::Dispatch(error.to_string()))
}

/// A wire enum's number, known or not.
fn raw<T: Into<u32>>(value: WEnum<T>) -> u32 {
    match value {
        WEnum::Value(known) => known.into(),
        WEnum::Unknown(code) => code,
    }
}

impl Dispatch<ZwlrScreencopyManagerV1, ()> for Copying {
    fn event(
        _: &mut Self,
        _: &ZwlrScreencopyManagerV1,
        _: zwlr_screencopy_manager_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ZwlrScreencopyFrameV1, ()> for Copying {
    fn event(
        state: &mut Self,
        _: &ZwlrScreencopyFrameV1,
        event: zwlr_screencopy_frame_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        use zwlr_screencopy_frame_v1::Event;
        match event {
            Event::Buffer {
                format,
                width,
                height,
                stride,
            } => state.offers.push(Offer {
                format: raw(format),
                size: Size::new(width, height),
                stride,
            }),
            Event::Flags { flags } => {
                state.flags = flags.into_result().map_or(0, |known| known.bits());
            }
            Event::Damage {
                x,
                y,
                width,
                height,
            } => state.damage = Some((x, y, width, height)),
            Event::Ready { .. } => state.done = Some(Ok(())),
            Event::Failed => state.done = Some(Err(CopyFailure::Failed)),
            _ => {}
        }
    }
}

impl Dispatch<WlOutput, u32> for Copying {
    fn event(
        state: &mut Self,
        _: &WlOutput,
        event: wl_output::Event,
        global: &u32,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let wl_output::Event::Name { name } = event
            && let Some(entry) = state.outputs.iter_mut().find(|(g, _, _)| g == global)
        {
            entry.2 = Some(name);
        }
    }
}

impl Dispatch<WlShm, ()> for Copying {
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

impl Dispatch<WlShmPool, ()> for Copying {
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

impl Dispatch<WlBuffer, ()> for Copying {
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
