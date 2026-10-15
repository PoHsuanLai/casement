//! The events of one `zwlr_screencopy_frame_v1`, as our types go out.

use casement_core::geometry::PhysicalRect;
use casement_core::time::Mono;
use smithay::reexports::wayland_protocols_wlr::screencopy::v1::server::zwlr_screencopy_frame_v1::{
    Flags, ZwlrScreencopyFrameV1,
};
use smithay::reexports::wayland_server::Resource;
use smithay::reexports::wayland_server::protocol::wl_shm::Format;

/// The version that added the `damage` event.
const DAMAGE_SINCE: u32 = 2;
/// The version that added `buffer_done` (and made several `buffer` events legal).
const BUFFER_DONE_SINCE: u32 = 3;

/// The buffers the client may copy into: `argb8888` always (the frame's own bytes) and, from
/// version 3 where several are allowed, `xrgb8888`.
pub(super) fn announce(frame: &ZwlrScreencopyFrameV1, area: PhysicalRect) {
    let stride = area.w.saturating_mul(4);
    frame.buffer(Format::Argb8888, area.w, area.h, stride);
    if frame.version() >= BUFFER_DONE_SINCE {
        frame.buffer(Format::Xrgb8888, area.w, area.h, stride);
        frame.buffer_done();
    }
}

/// The frame is in the buffer: no flags (the rows are top first), the damage when the client
/// asked for it (the whole area: casement keeps no per-client history), and `ready` at `at`.
pub(super) fn send_ready(frame: &ZwlrScreencopyFrameV1, damage: Option<PhysicalRect>, at: Mono) {
    frame.flags(Flags::empty());
    if let Some(area) = damage
        && frame.version() >= DAMAGE_SINCE
    {
        frame.damage(0, 0, area.w, area.h);
    }
    let seconds = at.0.as_secs();
    let high = u32::try_from(seconds >> 32).unwrap_or(u32::MAX);
    let low = u32::try_from(seconds & 0xFFFF_FFFF).unwrap_or(u32::MAX);
    frame.ready(high, low, at.0.subsec_nanos());
}
