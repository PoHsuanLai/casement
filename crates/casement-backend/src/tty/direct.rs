//! The direct-open seams (`casement --backend tty --drm-node PATH`, P1-V2): a test-only way to
//! run the tty backend on one named DRM node, the kernel's virtual `vkms` device, with no
//! libseat, no logind session and no VT.
//!
//! * [`DirectSession`] is the [`TtySession`] of that mode. It opens nothing itself (the node is
//!   opened once by `direct_real.rs`, after [`require_vkms`] checked its driver) and refuses a
//!   VT switch. The seat changes hands only when the edge says so through its [`SleepLink`]:
//!   the binary maps `SIGUSR1` to a pause and `SIGUSR2` to a return, the VT switch simulated.
//! * [`FixedNode`] is the [`DeviceSource`]: the one node is present at start, and stands in for
//!   the udev monitor by reporting the node changed now and then, so a hotplug is seen.
//! * [`NoGpus`] is the [`RenderNodes`]: this mode detects nothing and so never touches, or even
//!   enumerates, another GPU.
//!
//! There is no input feed in this mode (no libinput, so no other device is opened). Only
//! `--drm-node` mode builds these; a real seat never does.
//!
//! ```
//! use casement_backend::tty::{DirectSession, SessionNote, TtySession};
//!
//! let mut session = DirectSession::new(|| {});
//! let link = session.sleep_link().expect("the edge's end");
//! link.activated();
//! assert_eq!(session.notes(), [SessionNote::Activated]);
//! ```

use super::session::{SessionNote, TtySession};
use super::sleep::{SleepInbox, SleepLink, sleep_channel};
use super::udev::{DeviceEvent, DevicePath, DeviceSource};
use super::vt::VtNumber;
use crate::gpu::{GpuError, GpuHandle, RenderNodes};
use crate::session::SessionFault;
use casement_core::gpu::{DrmNodeId, GpuFacts};
use std::ffi::OsStr;
use std::os::fd::OwnedFd;

/// The one driver `--drm-node` accepts.
pub const VKMS_DRIVER: &str = "vkms";

/// The node is driven by something else than vkms.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error(
    "the DRM node is driven by `{driver}`, not `vkms`: --drm-node is a test mode for the \
     virtual display driver only and never opens a real GPU"
)]
#[non_exhaustive]
pub struct DriverRefusal {
    /// The driver the version ioctl named.
    pub driver: String,
}

/// Accepts the driver name `driver` (what the DRM version ioctl reports) only when it is vkms.
///
/// ```
/// use casement_backend::tty::require_vkms;
/// use std::ffi::OsStr;
///
/// assert!(require_vkms(OsStr::new("vkms")).is_ok());
/// assert!(require_vkms(OsStr::new("amdgpu")).is_err());
/// ```
pub fn require_vkms(driver: &OsStr) -> Result<(), DriverRefusal> {
    match driver == OsStr::new(VKMS_DRIVER) {
        true => Ok(()),
        false => Err(DriverRefusal {
            driver: driver.to_string_lossy().into_owned(),
        }),
    }
}

/// The session of `--drm-node` mode: no seat, only the edge's pause and return.
#[derive(Debug)]
pub struct DirectSession {
    inbox: SleepInbox,
    link: Option<SleepLink>,
}

impl DirectSession {
    /// A session whose edge calls `wake` after each note it sends (the real one pings the
    /// event loop; a test passes a no-op).
    pub fn new(wake: impl Fn() + Send + Sync + 'static) -> DirectSession {
        let (link, inbox) = sleep_channel(wake);
        DirectSession {
            inbox,
            link: Some(link),
        }
    }
}

impl TtySession for DirectSession {
    fn open_device(&mut self, path: &DevicePath) -> Result<OwnedFd, SessionFault> {
        Err(SessionFault::DeniedOpen(
            path.as_path().display().to_string(),
        ))
    }

    fn switch_vt(&mut self, vt: VtNumber) -> Result<(), SessionFault> {
        Err(SessionFault::Unavailable(format!(
            "no VT {} to switch to: --drm-node mode has no VTs",
            vt.0
        )))
    }

    fn notes(&mut self) -> Vec<SessionNote> {
        self.inbox.take()
    }

    fn settled(&mut self, note: SessionNote) {
        if note == SessionNote::Paused {
            self.inbox.settled();
        }
    }

    fn sleep_link(&mut self) -> Option<SleepLink> {
        self.link.take()
    }
}

/// How many of the backend's turns pass between two looks at the device.
pub const POLL_EVERY: u8 = 15;

/// Where the poll stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Poll {
    /// This many turns until the next look.
    Wait(u8),
    /// The look was reported; the turn that reported it ends with the next call.
    Reported,
}

/// The one display device of `--drm-node` mode: present at start, never added or removed. There
/// is no udev monitor in this mode, so a hotplug (a vkms connector connected through configfs)
/// would never be heard; instead the device is reported `Changed` once every
/// [`POLL_EVERY`] + 1 turns of the backend, and the backend reads its connectors again. A look
/// that finds nothing new reports nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixedNode {
    node: DevicePath,
    poll: Poll,
}

impl FixedNode {
    /// The node at `path` is the only display device.
    pub fn new(path: DevicePath) -> FixedNode {
        FixedNode {
            node: path,
            poll: Poll::Wait(POLL_EVERY),
        }
    }
}

impl DeviceSource for FixedNode {
    fn present(&self) -> Vec<DevicePath> {
        vec![self.node.clone()]
    }

    /// The backend asks until this answers none, once a turn: the call that reports the look is
    /// followed by one that ends the turn.
    fn next_event(&mut self) -> Option<DeviceEvent> {
        match self.poll {
            Poll::Wait(0) => {
                self.poll = Poll::Reported;
                Some(DeviceEvent::Changed(self.node.clone()))
            }
            Poll::Wait(turns) => {
                self.poll = Poll::Wait(turns - 1);
                None
            }
            Poll::Reported => {
                self.poll = Poll::Wait(POLL_EVERY);
                None
            }
        }
    }
}

/// A machine whose GPUs are not detected: no render plan, no dma-buf feedback.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct NoGpus;

impl RenderNodes for NoGpus {
    fn nodes(&self) -> Vec<GpuFacts> {
        Vec::new()
    }

    fn open(&mut self, node: &DrmNodeId) -> Result<GpuHandle, GpuError> {
        Err(GpuError::UnknownNode(*node))
    }
}
