//! The real writeback connector (feature `tty`): [`Writeback`] over the DRM device's atomic API,
//! a thin adapter (B7). Used only by `--drm-node` mode on the vkms device.
//!
//! A job is one atomic commit on the writeback connector: `CRTC_ID` names the CRTC to read and
//! `WRITEBACK_FB_ID` a dumb buffer (`XRGB8888`, the CRTC's mode size) the device composes the
//! next frame into. The commit is made on the same open file as the display device's (a
//! duplicate of its descriptor), because an atomic commit needs the DRM master. No out-fence is
//! used (it needs a pointer the kernel writes through), so the caller waits some flips instead
//! ([`FLIPS_TO_WAIT`](super::FLIPS_TO_WAIT)). `finish` reads the buffer through the dumb
//! buffer's mapping, detaches the connector and frees the buffer.

use super::kms::CrtcId;
use super::writeback::{RawCapture, Writeback, WritebackError};
use casement_core::geometry::PhysicalSize;
use casement_core::gpu::Fourcc;
use smithay::backend::drm::DrmDeviceFd;
use smithay::reexports::drm::buffer::{Buffer, DrmFourcc};
use smithay::reexports::drm::control::atomic::AtomicModeReq;
use smithay::reexports::drm::control::dumbbuffer::DumbBuffer;
use smithay::reexports::drm::control::{
    AtomicCommitFlags, Device as ControlDevice, ResourceHandle, connector, crtc, framebuffer,
    property,
};
use smithay::reexports::drm::{ClientCapability, Device as BasicDevice};
use std::io;

/// The pixel format of the buffer a job writes into: the one every writeback connector offers.
const FORMAT: DrmFourcc = DrmFourcc::Xrgb8888;

/// A job that has been committed and not yet read.
#[derive(Debug)]
struct Job {
    connector: connector::Handle,
    buffer: DumbBuffer,
    framebuffer: framebuffer::Handle,
}

/// The writeback connector of the device behind `fd`.
#[derive(Debug)]
pub struct RealWriteback {
    fd: DrmDeviceFd,
    job: Option<Job>,
}

impl RealWriteback {
    /// Wraps the device behind `fd` (the same open file the display device uses) and asks the
    /// kernel to list writeback connectors, which it hides from clients that do not.
    pub fn open(fd: DrmDeviceFd) -> Result<RealWriteback, WritebackError> {
        fd.set_client_capability(ClientCapability::UniversalPlanes, true)?;
        fd.set_client_capability(ClientCapability::Atomic, true)?;
        fd.set_client_capability(ClientCapability::WritebackConnectors, true)?;
        Ok(RealWriteback { fd, job: None })
    }

    /// The device's writeback connector.
    fn connector(&self) -> Result<connector::Handle, WritebackError> {
        let handles = self.fd.resource_handles()?;
        handles
            .connectors()
            .iter()
            .copied()
            .find(|handle| {
                self.fd
                    .get_connector(*handle, false)
                    .is_ok_and(|info| info.interface() == connector::Interface::Writeback)
            })
            .ok_or(WritebackError::NoConnector)
    }

    /// The CRTC with kernel id `id`.
    fn crtc(&self, id: CrtcId) -> Result<crtc::Handle, WritebackError> {
        self.fd
            .resource_handles()?
            .crtcs()
            .iter()
            .copied()
            .find(|handle| u32::from(*handle) == id.0)
            .ok_or(WritebackError::NoCrtc(id))
    }

    /// The handle of the property `name` on `object`.
    fn property(
        &self,
        object: impl ResourceHandle,
        name: &str,
    ) -> Result<property::Handle, WritebackError> {
        let set = self.fd.get_properties(object)?;
        let (ids, _) = set.as_props_and_values();
        ids.iter()
            .copied()
            .find(|id| {
                self.fd
                    .get_property(*id)
                    .is_ok_and(|info| info.name().to_bytes() == name.as_bytes())
            })
            .ok_or_else(|| io::Error::other(format!("the connector has no {name} property")).into())
    }

    /// Points the writeback connector at `crtc` and `framebuffer` (or at none).
    fn commit(
        &self,
        connector: connector::Handle,
        crtc: Option<crtc::Handle>,
        framebuffer: Option<framebuffer::Handle>,
    ) -> Result<(), WritebackError> {
        let mut request = AtomicModeReq::new();
        request.add_property(
            connector,
            self.property(connector, "CRTC_ID")?,
            property::Value::CRTC(crtc),
        );
        request.add_property(
            connector,
            self.property(connector, "WRITEBACK_FB_ID")?,
            property::Value::Framebuffer(framebuffer),
        );
        self.fd
            .atomic_commit(AtomicCommitFlags::ALLOW_MODESET, request)?;
        Ok(())
    }

    /// Frees the job's buffer.
    fn release(&self, job: Job) {
        if let Err(error) = self.fd.destroy_framebuffer(job.framebuffer) {
            tracing::warn!(%error, "a writeback framebuffer was not freed");
        }
        if let Err(error) = self.fd.destroy_dumb_buffer(job.buffer) {
            tracing::warn!(%error, "a writeback buffer was not freed");
        }
    }
}

impl Writeback for RealWriteback {
    fn begin(&mut self, crtc: CrtcId, size: PhysicalSize) -> Result<(), WritebackError> {
        if self.job.is_some() {
            return Err(WritebackError::Busy);
        }
        let connector = self.connector()?;
        let crtc = self.crtc(crtc)?;
        let buffer = self.fd.create_dumb_buffer((size.w, size.h), FORMAT, 32)?;
        let framebuffer = match self.fd.add_framebuffer(&buffer, 24, 32) {
            Ok(framebuffer) => framebuffer,
            Err(error) => {
                let _ = self.fd.destroy_dumb_buffer(buffer);
                return Err(error.into());
            }
        };
        let job = Job {
            connector,
            buffer,
            framebuffer,
        };
        match self.commit(connector, Some(crtc), Some(framebuffer)) {
            Ok(()) => {
                self.job = Some(job);
                Ok(())
            }
            Err(error) => {
                self.release(job);
                Err(error)
            }
        }
    }

    fn finish(&mut self) -> Result<RawCapture, WritebackError> {
        let mut job = self.job.take().ok_or(WritebackError::NotBegun)?;
        let (width, height) = job.buffer.size();
        let pitch = job.buffer.pitch();
        let read = self
            .fd
            .map_dumb_buffer(&mut job.buffer)
            .map(|mapping| mapping.to_vec());
        // The connector goes back to idle whatever the read said.
        if let Err(error) = self.commit(job.connector, None, None) {
            tracing::warn!(%error, "the writeback connector was not detached");
        }
        self.release(job);
        let bytes = read?;
        Ok(RawCapture::new(
            PhysicalSize {
                w: width,
                h: height,
            },
            pitch,
            Fourcc(FORMAT as u32),
            bytes,
        ))
    }
}
