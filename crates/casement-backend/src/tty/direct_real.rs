//! The real direct-open seams (feature `tty`): `--drm-node PATH` mode, a thin adapter (B7). The
//! node is opened once, as the user, with no libseat and no logind; the DRM version ioctl must
//! name the vkms driver or nothing is built; and no other device is opened (no libinput, no GPU
//! detection). The node's descriptor serves the display device and, when asked, the writeback
//! connector: duplicates of one open file, so they share one DRM master.

use super::TtySeams;
use super::direct::{DirectSession, FixedNode, NoGpus, require_vkms};
use super::kms::KmsError;
use super::kms_real::{FdSource, RealKmsOpener};
use super::render_mode::RenderMode;
use super::udev::DevicePath;
use super::writeback::WritebackMode;
use super::writeback_real::RealWriteback;
use crate::backend::BackendError;
use crate::session::SessionFault;
use casement_core::gpu::GpuChoice;
use smithay::backend::drm::DrmDeviceFd;
use smithay::reexports::calloop::LoopHandle;
use smithay::reexports::calloop::ping::make_ping;
use smithay::reexports::drm::Device as BasicDevice;
use smithay::utils::DeviceFd;
use std::fs::OpenOptions;
use std::io;
use std::os::fd::OwnedFd;
use std::path::Path;

/// Hands the node's descriptor to the KMS opener, once, and only for the node.
#[derive(Debug)]
struct DirectFds {
    node: DevicePath,
    fd: Option<OwnedFd>,
}

impl FdSource for DirectFds {
    fn fd(&mut self, path: &DevicePath) -> Result<OwnedFd, KmsError> {
        if *path != self.node {
            return Err(KmsError::Io(io::Error::other(format!(
                "{} is not the --drm-node",
                path.as_path().display()
            ))));
        }
        self.fd
            .take()
            .ok_or_else(|| KmsError::Io(io::Error::other("the --drm-node is already open")))
    }
}

/// A second descriptor on the same open file, as a DRM device.
fn device_on(fd: &OwnedFd) -> io::Result<DrmDeviceFd> {
    Ok(DrmDeviceFd::new(DeviceFd::from(fd.try_clone()?)))
}

/// The seams of `--drm-node node`: the node opened directly and checked to be vkms, a session
/// that moves only on the edge's word, the node as the one display device, no GPUs and no
/// input; with the writeback connector when `writeback` asks. `mode` says how the node's outputs
/// are drawn (`--safe`: by the CPU, with no GBM device opened). `handle`'s loop is woken whenever
/// the edge sends a note.
pub fn direct_seams<D: 'static>(
    node: &Path,
    writeback: WritebackMode,
    mode: RenderMode,
    handle: &LoopHandle<'static, D>,
) -> Result<TtySeams, BackendError> {
    let unavailable = |why: String| SessionFault::Unavailable(why);
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(node)
        .map_err(|source| BackendError::DeniedOpen {
            path: node.to_path_buf(),
            source,
        })?;
    let fd = OwnedFd::from(file);
    let named = |error: io::Error| unavailable(format!("{}: {error}", node.display()));
    let driver = device_on(&fd)
        .and_then(|device| device.get_driver())
        .map_err(named)?;
    require_vkms(driver.name()).map_err(|refusal| unavailable(refusal.to_string()))?;
    let writeback = match writeback {
        WritebackMode::Enabled => {
            let device = device_on(&fd).map_err(named)?;
            let seam = RealWriteback::open(device)
                .map_err(|error| unavailable(format!("{}: {error}", node.display())))?;
            Some(seam)
        }
        _ => None,
    };
    let (ping, wake) = make_ping().map_err(|error| unavailable(error.to_string()))?;
    handle
        .insert_source(wake, |(), _, _| {})
        .map_err(|error| unavailable(error.error.to_string()))?;
    let path = DevicePath(node.to_path_buf());
    let fds = DirectFds {
        node: path.clone(),
        fd: Some(fd),
    };
    let seams = TtySeams::new(
        Box::new(DirectSession::new(move || ping.ping())),
        Box::new(FixedNode::new(path)),
        Box::new(RealKmsOpener::new(Box::new(fds), handle.clone()).drawing(mode)),
        Box::new(NoGpus),
        GpuChoice::Auto,
    );
    Ok(match writeback {
        Some(seam) => seams.with_writeback(Box::new(seam)),
        None => seams,
    })
}
