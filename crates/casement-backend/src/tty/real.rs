//! The real tty seams, assembled (feature `tty`): libseat, udev, the KMS opener, the render
//! nodes under the host's `/sys` and `/dev`, the formats each render node draws into, and (with
//! feature `libinput`) the seat's input devices opened through the same session.

use super::SysDeviceKinds;
use super::TtySeams;
use super::kms_real::RealKmsOpener;
use super::render_hub_real::RenderHub;
use super::session_real::RealSession;
use super::udev_real::RealUdev;
use crate::backend::BackendError;
use crate::gpu::{RealFormats, RenderNodes, SysRenderNodes, SysRoot};
#[cfg(feature = "libinput")]
use crate::input::RealInputFeed;
use crate::session::SessionFault;
use casement_core::gpu::{GpuChoice, plan_for};
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
    let nodes = SysRenderNodes::new(root.clone());
    let opener = RealKmsOpener::new(Box::new(fds), handle.clone()).refusing_virtual();
    let opener = match plan_for(&nodes.nodes(), &choice) {
        Ok(plan) => {
            tracing::info!(render = ?plan.render, routes = ?plan.routes(), "the render GPU and each output's route");
            opener.routed(RenderHub::new(root, plan))
        }
        Err(refusal) => {
            tracing::warn!(%refusal, "no render plan; every output is drawn by its own GPU");
            opener
        }
    };
    let seams = TtySeams::new(
        Box::new(session),
        Box::new(udev),
        Box::new(opener),
        Box::new(nodes),
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
