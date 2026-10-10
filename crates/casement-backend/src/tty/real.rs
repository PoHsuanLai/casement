//! The real tty seams, assembled (feature `tty`): libseat, udev, the KMS opener and the render
//! nodes under the host's `/sys` and `/dev`.

use super::TtySeams;
use super::kms_real::RealKmsOpener;
use super::session_real::RealSession;
use super::udev_real::RealUdev;
use crate::backend::BackendError;
use crate::gpu::{SysRenderNodes, SysRoot};
use crate::session::SessionFault;
use casement_core::gpu::GpuChoice;
use smithay::reexports::calloop::LoopHandle;

/// The seams of the machine casement runs on, with `root` as its `/sys` and `/dev`, `choice` as
/// the `gpu.render` setting, and `handle`'s loop reading the seat and the udev monitor.
pub fn real_seams<D: 'static>(
    root: SysRoot,
    choice: GpuChoice,
    handle: &LoopHandle<'static, D>,
) -> Result<TtySeams, BackendError> {
    let session = RealSession::open(handle)?;
    let udev = RealUdev::watch(&session.seat_name(), handle)
        .map_err(|error| SessionFault::Unavailable(format!("udev: {error}")))?;
    Ok(TtySeams::new(
        Box::new(session),
        Box::new(udev),
        Box::new(RealKmsOpener),
        Box::new(SysRenderNodes::new(root)),
        choice,
    ))
}
