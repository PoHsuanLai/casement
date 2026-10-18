//! The real tty seams, assembled (feature `tty`): libseat, udev, the KMS opener, the render
//! nodes under the host's `/sys` and `/dev`, the formats each render node draws into, and (with
//! feature `libinput`) the seat's input devices opened through the same session.

use super::SysDeviceKinds;
use super::TtySeams;
use super::direct::NoGpus;
use super::kms_real::RealKmsOpener;
use super::render_hub_real::RenderHub;
use super::render_mode::RenderMode;
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
/// the `gpu.render` setting, `mode` as how outputs are drawn, and `handle`'s loop reading the
/// seat, the udev monitor, the display devices and the input devices.
///
/// In [`RenderMode::Software`] (`--safe`) the GPUs are not enumerated, no render plan is made,
/// no format source is given (the formats would be read through a GBM device and an EGL
/// display), and the display devices open no GBM device: nothing in the session loads a GPU
/// driver's userspace.
pub fn real_seams<D: 'static>(
    root: SysRoot,
    choice: GpuChoice,
    mode: RenderMode,
    handle: &LoopHandle<'static, D>,
) -> Result<TtySeams, BackendError> {
    let session = RealSession::open(handle)?;
    let seat = session.seat_name();
    let input_session = session.seat_handle();
    let udev = RealUdev::watch(&seat, handle)
        .map_err(|error| SessionFault::Unavailable(format!("udev: {error}")))?;
    let fds = session.fds();
    let kinds = SysDeviceKinds::new(root.clone());
    let opener = RealKmsOpener::new(Box::new(fds), handle.clone())
        .refusing_virtual()
        .drawing(mode);
    let seams = match mode {
        RenderMode::Gpu => on_gpus(Box::new(session), Box::new(udev), opener, root, choice),
        RenderMode::Software => on_cpu(Box::new(session), Box::new(udev), opener, choice),
    };
    with_input(seams.with_kinds(kinds), &seat, input_session, handle)
}

/// The seams of a session drawn by the GPUs: the render plan (when the GPUs were found) routes
/// each output, and the formats each GPU draws into are offered to clients.
fn on_gpus<D: 'static>(
    session: Box<RealSession>,
    udev: Box<RealUdev>,
    opener: RealKmsOpener<D>,
    root: SysRoot,
    choice: GpuChoice,
) -> TtySeams {
    let formats = RealFormats::new(root.clone());
    let nodes = SysRenderNodes::new(root.clone());
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
    TtySeams::new(session, udev, Box::new(opener), Box::new(nodes), choice)
        .with_formats(Box::new(formats))
}

/// The seams of a session drawn by the CPU: no GPU is enumerated, so there is no plan and clients
/// are offered shared memory only.
fn on_cpu<D: 'static>(
    session: Box<RealSession>,
    udev: Box<RealUdev>,
    opener: RealKmsOpener<D>,
    choice: GpuChoice,
) -> TtySeams {
    tracing::warn!("safe mode: every output is drawn by the CPU; no GPU driver is loaded");
    TtySeams::new(session, udev, Box::new(opener), Box::new(NoGpus), choice)
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
