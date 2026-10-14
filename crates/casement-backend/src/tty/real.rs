//! The real tty seams, assembled (feature `tty`): libseat, udev, the KMS opener, the render
//! nodes under the host's `/sys` and `/dev`, the formats each render node draws into, and (with
//! feature `libinput`) the seat's input devices opened through the same session.

use super::SysDeviceKinds;
use super::TtySeams;
use super::kms_real::RealKmsOpener;
use super::session_real::RealSession;
use super::udev_real::RealUdev;
use crate::backend::BackendError;
use crate::gpu::{RealFormats, SysRenderNodes, SysRoot};
#[cfg(feature = "libinput")]
use crate::input::RealInputFeed;
use crate::session::SessionFault;
use casement_core::gpu::GpuChoice;
use smithay::backend::session::libseat::LibSeatSession;
use smithay::reexports::calloop::LoopHandle;

/// The seams of the machine casement runs on, with `root` as its `/sys` and `/dev`, `choice` as
/// the `gpu.render` setting, and `handle`'s loop reading the seat, the udev monitor, the display
/// devices and the input devices.
pub fn real_seams<D: 'static>(
    root: SysRoot,
    choice: GpuChoice,
    handle: &LoopHandle<'static, D>,
) -> Result<TtySeams, BackendError> {
    let session = RealSession::open(handle)?;
    let seat = session.seat_name();
    let input_session = session.seat_handle();
    let udev = RealUdev::watch(&seat, handle)
        .map_err(|error| SessionFault::Unavailable(format!("udev: {error}")))?;
    let fds = session.fds();
    let formats = RealFormats::new(root.clone());
    let kinds = SysDeviceKinds::new(root.clone());
    let seams = TtySeams::new(
        Box::new(session),
        Box::new(udev),
        Box::new(RealKmsOpener::new(Box::new(fds), handle.clone()).refusing_virtual()),
        Box::new(SysRenderNodes::new(root)),
        choice,
    )
    .with_formats(Box::new(formats))
    .with_kinds(Box::new(kinds));
    with_input(seams, &seat, input_session, handle)
}

/// The seat's input devices, opened through its session.
#[cfg(feature = "libinput")]
fn with_input<D: 'static>(
    seams: TtySeams,
    seat: &str,
    session: LibSeatSession,
    handle: &LoopHandle<'static, D>,
) -> Result<TtySeams, BackendError> {
    let feed = RealInputFeed::open(seat, session, handle)
        .map_err(|error| SessionFault::Unavailable(format!("libinput: {error}")))?;
    Ok(seams.with_input(Box::new(feed)))
}

/// A build without libinput has a seat with no input.
#[cfg(not(feature = "libinput"))]
fn with_input<D: 'static>(
    seams: TtySeams,
    seat: &str,
    session: LibSeatSession,
    handle: &LoopHandle<'static, D>,
) -> Result<TtySeams, BackendError> {
    let _ = (seat, session, handle);
    Ok(seams)
}
