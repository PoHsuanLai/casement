//! `zwlr_screencopy_manager_v1` (version 3): the older capture protocol `grim`, `wf-recorder`
//! and the screenshot portals' fallbacks speak. A client captures one output, or a region of it
//! in the output's logical pixels, into a shared-memory buffer.
//!
//! The frame is the one scanout shows, the same pixels `ext_image_copy_capture` serves
//! ([`crate::image_capture`]); this module keeps the wire and the pending requests, and the
//! composition root answers them ([`ScreencopyHandler::screencopy_wake`]) as frames are
//! presented. A copy that finds no frame yet (the first on a real seat, before the capture
//! shadow has drawn) waits for the next one. `copy_with_damage` waits until something changed
//! on the output since the frame the same client last got of it, and reports what changed
//! ([`crate::image_capture::DamageLogs`]; the whole area for a client's first frame). The
//! cursor is in the frame when the request's `overlay_cursor` is not 0 and left out when it is
//! ([`CursorPaint`]); dma-buf buffers are not offered. Visible to the same connections as `ext_image_copy_capture` (the I7 table's
//! `ImageCapture`): the public socket and the shell, not a sandbox.

mod region;
mod wire;

pub use region::physical_region;

use crate::image_capture::CursorPaint;
use crate::states::GlobalFilter;
use casement_core::OutputKey;
use casement_core::geometry::{PhysicalRect, PhysicalSize, Transform};
use casement_core::policy::GlobalKind;
use casement_core::time::Mono;
use ds_core::geometry::scale::Scale;
use smithay::reexports::wayland_protocols_wlr::screencopy::v1::server::{
    zwlr_screencopy_frame_v1::{self, ZwlrScreencopyFrameV1},
    zwlr_screencopy_manager_v1::{self, ZwlrScreencopyManagerV1},
};
use smithay::reexports::wayland_server::backend::{ClientId, GlobalId};
use smithay::reexports::wayland_server::protocol::wl_buffer::WlBuffer;
use smithay::reexports::wayland_server::protocol::wl_output::WlOutput;
use smithay::reexports::wayland_server::protocol::wl_shm::Format;
use smithay::reexports::wayland_server::{
    Client, DataInit, Dispatch, DisplayHandle, GlobalDispatch, New, Resource,
};
use smithay::wayland::shm::with_buffer_contents;
use smithay::wayland::{Dispatch2, GlobalDispatch2};
use std::sync::{Mutex, PoisonError};

/// The version casement serves.
pub const VERSION: u32 = 3;

/// Whether a copy reports damage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Damage {
    /// `copy`: the frame as it is now.
    Whole,
    /// `copy_with_damage`: waits for a frame newer than the client's last of this output.
    Fresh,
}

/// What the composition root knows of an output a client named. Plain data the root fills, so
/// not `non_exhaustive`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScreencopyOutput {
    /// The model's key.
    pub key: OutputKey,
    /// Its mode: the size of a full frame.
    pub mode: PhysicalSize,
    /// Its scale (a region is in logical pixels).
    pub scale: Scale,
    /// Its transform.
    pub transform: Transform,
}

/// How much of an output a request names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Region {
    /// All of it.
    Whole,
    /// This output-local logical rectangle: `(x, y, width, height)`.
    Part((i32, i32, i32, i32)),
}

impl ScreencopyOutput {
    /// What a request for `region` copies, in physical pixels of the frame; none when nothing
    /// of it is on the output. A region of a turned output is refused: its logical pixels do
    /// not line up with the frame's rows.
    pub fn target(&self, region: Region) -> Option<Target> {
        let area = match region {
            Region::Whole => PhysicalRect {
                x: 0,
                y: 0,
                w: self.mode.w,
                h: self.mode.h,
            },
            Region::Part(_) if self.transform != Transform::Normal => return None,
            Region::Part(rect) => physical_region(rect, self.scale, self.mode)?,
        };
        Some(Target {
            output: self.key.clone(),
            area,
        })
    }
}

/// The output and the part of its frame one frame object copies.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Target {
    /// The output.
    pub output: OutputKey,
    /// The part of its frame, in physical pixels.
    pub area: PhysicalRect,
}

/// A `copy` waiting for its frame.
#[derive(Debug)]
#[non_exhaustive]
pub struct PendingCopy {
    /// The frame object to answer.
    pub frame: ZwlrScreencopyFrameV1,
    /// The client's buffer (checked at the request).
    pub buffer: WlBuffer,
    /// What to copy.
    pub target: Target,
    /// Whether it reports damage.
    pub damage: Damage,
    /// Whether the frame has the cursor in it.
    pub cursor: CursorPaint,
}

/// The last frame a client was given of an output.
#[derive(Debug)]
struct Delivery {
    client: ClientId,
    output: OutputKey,
    at: Mono,
}

/// The manager's state: the global, the copies waiting and what each client last got.
#[derive(Debug, Default)]
pub struct ScreencopyPart {
    /// Our global (none until [`ScreencopyPart::new`]).
    pub global: Option<GlobalId>,
    pending: Vec<PendingCopy>,
    delivered: Vec<Delivery>,
}

impl ScreencopyPart {
    /// Creates the global (version 3), filtered by the I7 table.
    pub fn new<D>(display: &DisplayHandle, role_filter: GlobalFilter) -> ScreencopyPart
    where
        D: GlobalDispatch<ZwlrScreencopyManagerV1, ScreencopyGlobal> + 'static,
    {
        ScreencopyPart {
            global: Some(display.create_global::<D, ZwlrScreencopyManagerV1, _>(
                VERSION,
                ScreencopyGlobal {
                    filter: role_filter,
                },
            )),
            ..ScreencopyPart::default()
        }
    }

    /// Takes the waiting copies, to answer them (those still waiting go back with [`keep`]).
    ///
    /// [`keep`]: ScreencopyPart::keep
    pub fn take_pending(&mut self) -> Vec<PendingCopy> {
        std::mem::take(&mut self.pending)
    }

    /// Keeps `copy` waiting.
    pub fn keep(&mut self, copy: PendingCopy) {
        self.pending.push(copy);
    }

    /// The instant of the frame `copy`'s client was last given of its output.
    pub fn last_delivery(&self, copy: &PendingCopy) -> Option<Mono> {
        let client = copy.frame.client()?.id();
        self.delivered
            .iter()
            .find(|d| d.client == client && d.output == copy.target.output)
            .map(|d| d.at)
    }

    /// Whether a copy waits for a frame of `output`: a real seat draws the frame only while
    /// someone asks for it.
    pub fn wants(&self, output: &OutputKey) -> bool {
        self.pending
            .iter()
            .any(|copy| copy.target.output == *output)
    }

    /// Notes that `copy`'s client was given the frame presented at `at`.
    fn note_delivery(&mut self, copy: &PendingCopy, at: Mono) {
        let Some(client) = copy.frame.client().map(|client| client.id()) else {
            return;
        };
        self.delivered
            .retain(|d| !(d.client == client && d.output == copy.target.output));
        self.delivered.push(Delivery {
            client,
            output: copy.target.output.clone(),
            at,
        });
    }
}

/// What the composition root provides to this protocol's dispatch.
pub trait ScreencopyHandler: Sized + 'static {
    /// The part.
    fn screencopy_part(&mut self) -> &mut ScreencopyPart;
    /// The output `output` is, if the model has it.
    fn screencopy_output(&self, output: &WlOutput) -> Option<ScreencopyOutput>;
    /// A copy was queued: answer what can be answered now.
    fn screencopy_wake(&mut self);
}

/// The frame was copied into the client's buffer, presented at `at`: `ready`, with `damage`
/// (rectangles in the copied area's pixels) when the client asked for it.
pub fn apply_ready(
    part: &mut ScreencopyPart,
    copy: &PendingCopy,
    at: Mono,
    damage: &[PhysicalRect],
) {
    let damage = match copy.damage {
        Damage::Whole => None,
        Damage::Fresh => Some(damage),
    };
    wire::send_ready(&copy.frame, damage, at);
    part.note_delivery(copy, at);
}

/// The frame could not be made: `failed`.
pub fn apply_failed(copy: &PendingCopy) {
    copy.frame.failed();
}

/// `output` was unplugged: its waiting copies fail and its deliveries are forgotten.
pub fn apply_output_gone(part: &mut ScreencopyPart, output: &OutputKey) {
    let (gone, kept): (Vec<_>, Vec<_>) = std::mem::take(&mut part.pending)
        .into_iter()
        .partition(|copy| copy.target.output == *output);
    part.pending = kept;
    gone.iter().for_each(apply_failed);
    part.delivered.retain(|d| d.output != *output);
}

/// The global's data: the filter its `can_view` asks.
#[derive(Debug, Clone, Copy)]
pub struct ScreencopyGlobal {
    filter: GlobalFilter,
}

/// A bound manager's data.
#[derive(Debug, Clone, Copy, Default)]
pub struct ManagerData;

/// Whether a frame object has had its one copy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Used {
    No,
    Yes,
}

/// Whether a client's buffer suits a frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Fit {
    Yes,
    No,
}

/// One frame object's data: what it copies (none when the request named nothing capturable,
/// the frame was `failed` at once) and whether it is used up.
#[derive(Debug)]
pub struct FrameData {
    target: Option<Target>,
    cursor: CursorPaint,
    used: Mutex<Used>,
}

impl FrameData {
    fn new(target: Option<Target>, cursor: CursorPaint) -> FrameData {
        FrameData {
            target,
            cursor,
            used: Mutex::new(Used::No),
        }
    }

    /// Marks the frame used; what it was.
    fn take_use(&self) -> Used {
        let mut used = self.used.lock().unwrap_or_else(PoisonError::into_inner);
        std::mem::replace(&mut *used, Used::Yes)
    }
}

/// Whether `buffer` is a shared-memory buffer of an offered format that holds `area`.
fn fits(buffer: &WlBuffer, area: PhysicalRect) -> Fit {
    let checked = with_buffer_contents(buffer, |_, _, data| {
        let format = matches!(data.format, Format::Argb8888 | Format::Xrgb8888);
        let size = i64::from(data.width) == i64::from(area.w)
            && i64::from(data.height) == i64::from(area.h);
        let stride = i64::from(data.stride) >= i64::from(area.w).saturating_mul(4);
        (format, size, stride)
    });
    match checked {
        Ok((true, true, true)) => Fit::Yes,
        _ => Fit::No,
    }
}

impl<D> GlobalDispatch2<ZwlrScreencopyManagerV1, D> for ScreencopyGlobal
where
    D: Dispatch<ZwlrScreencopyManagerV1, ManagerData> + 'static,
{
    fn bind(
        &self,
        _state: &mut D,
        _handle: &DisplayHandle,
        _client: &Client,
        resource: New<ZwlrScreencopyManagerV1>,
        data_init: &mut DataInit<'_, D>,
    ) {
        data_init.init(resource, ManagerData);
    }

    fn can_view(&self, client: &Client) -> bool {
        self.filter.sees(GlobalKind::ImageCapture, client)
    }
}

impl<D> Dispatch2<ZwlrScreencopyManagerV1, D> for ManagerData
where
    D: ScreencopyHandler + Dispatch<ZwlrScreencopyFrameV1, FrameData>,
{
    fn request(
        &self,
        state: &mut D,
        _client: &Client,
        _resource: &ZwlrScreencopyManagerV1,
        request: zwlr_screencopy_manager_v1::Request,
        _handle: &DisplayHandle,
        data_init: &mut DataInit<'_, D>,
    ) {
        let (frame, output, region, overlay_cursor) = match request {
            zwlr_screencopy_manager_v1::Request::CaptureOutput {
                frame,
                overlay_cursor,
                output,
            } => (frame, output, Region::Whole, overlay_cursor),
            zwlr_screencopy_manager_v1::Request::CaptureOutputRegion {
                frame,
                overlay_cursor,
                output,
                x,
                y,
                width,
                height,
            } => (
                frame,
                output,
                Region::Part((x, y, width, height)),
                overlay_cursor,
            ),
            _ => return,
        };
        let target = state
            .screencopy_output(&output)
            .and_then(|known| known.target(region));
        let announced = target.as_ref().map(|target| target.area);
        let frame = data_init.init(
            frame,
            FrameData::new(target, CursorPaint::from_overlay(overlay_cursor)),
        );
        match announced {
            Some(area) => wire::announce(&frame, area),
            None => frame.failed(),
        }
    }
}

impl FrameData {
    /// The client's one `copy` or `copy_with_damage`.
    fn copy<D: ScreencopyHandler>(
        &self,
        state: &mut D,
        frame: &ZwlrScreencopyFrameV1,
        buffer: WlBuffer,
        damage: Damage,
    ) {
        if self.take_use() == Used::Yes {
            frame.post_error(
                zwlr_screencopy_frame_v1::Error::AlreadyUsed,
                "the frame was already copied",
            );
            return;
        }
        // A frame the request refused was `failed` when it was made.
        let Some(target) = self.target.clone() else {
            return;
        };
        if fits(&buffer, target.area) == Fit::No {
            frame.post_error(
                zwlr_screencopy_frame_v1::Error::InvalidBuffer,
                "the buffer is not a shared-memory buffer of the size and format offered",
            );
            return;
        }
        state.screencopy_part().keep(PendingCopy {
            frame: frame.clone(),
            buffer,
            target,
            damage,
            cursor: self.cursor,
        });
        state.screencopy_wake();
    }
}

impl<D> Dispatch2<ZwlrScreencopyFrameV1, D> for FrameData
where
    D: ScreencopyHandler,
{
    fn request(
        &self,
        state: &mut D,
        _client: &Client,
        resource: &ZwlrScreencopyFrameV1,
        request: zwlr_screencopy_frame_v1::Request,
        _handle: &DisplayHandle,
        _data_init: &mut DataInit<'_, D>,
    ) {
        match request {
            zwlr_screencopy_frame_v1::Request::Copy { buffer } => {
                self.copy(state, resource, buffer, Damage::Whole);
            }
            zwlr_screencopy_frame_v1::Request::CopyWithDamage { buffer } => {
                self.copy(state, resource, buffer, Damage::Fresh);
            }
            _ => {}
        }
    }

    fn destroyed(&self, state: &mut D, _client: ClientId, resource: &ZwlrScreencopyFrameV1) {
        state
            .screencopy_part()
            .pending
            .retain(|copy| copy.frame != *resource);
    }
}
