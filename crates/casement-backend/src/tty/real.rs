//! The real tty seams, assembled (feature `tty`): libseat, udev, the KMS opener and the render
//! nodes under the host's `/sys` and `/dev`. P1-Z declares it; P1-D1 fills it.

use super::TtySeams;
use crate::backend::BackendError;
use crate::gpu::SysRoot;
use casement_core::gpu::GpuChoice;

/// The seams of the machine casement runs on, with `root` as its `/sys` and `/dev` and
/// `choice` as the `gpu.render` setting.
pub fn real_seams(root: SysRoot, choice: GpuChoice) -> Result<TtySeams, BackendError> {
    let _ = (root, choice);
    todo!("P1-D1: real_seams")
}
