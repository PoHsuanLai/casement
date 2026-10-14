//! Which display devices are firmware framebuffers (simpledrm and kin) or virtual test displays
//! (vkms), read from sysfs.
//!
//! The kernel keeps the boot framebuffer's DRM device after the GPU driver loads, so a machine
//! with a real GPU still lists a `card0` whose only connector is a phantom. The rule that keeps
//! it out of the outputs is pure (`casement_core::gpu::keep_displays`); this file only says what
//! each device is. A configfs vkms device sits on the `faux` bus (kernel 7.x), bound to
//! `faux_driver`; its DRM driver name is only visible once opened (see `kms_real.rs`). It reads
//! `sys/class/drm/<card>/device/driver` (the bound driver), `.../device` (the device name) and
//! `.../device/subsystem` (the bus) under an injected [`SysRoot`] (A6), so a test hands
//! it a fabricated tree.

use super::udev::DevicePath;
use crate::gpu::SysRoot;
use casement_core::gpu::DisplayKind;
use std::path::Path;

/// What a display device is.
pub trait DeviceKinds: std::fmt::Debug {
    /// The kind of the device at `path`.
    fn kind(&self, path: &DevicePath) -> DisplayKind;
}

/// Device kinds read from a sys root.
///
/// ```
/// use casement_backend::gpu::SysRoot;
/// use casement_backend::tty::{DeviceKinds, DevicePath, SysDeviceKinds};
/// use casement_core::gpu::DisplayKind;
///
/// let kinds = SysDeviceKinds::new(SysRoot::under("/nonexistent-sys-root".as_ref()));
/// let card = DevicePath("/dev/dri/card0".into());
/// assert_eq!(kinds.kind(&card), DisplayKind::Gpu, "an unreadable device is taken as a GPU");
/// ```
#[derive(Debug)]
pub struct SysDeviceKinds {
    root: SysRoot,
}

impl SysDeviceKinds {
    /// Kinds read under `root`.
    pub fn new(root: SysRoot) -> SysDeviceKinds {
        SysDeviceKinds { root }
    }
}

impl DeviceKinds for SysDeviceKinds {
    fn kind(&self, path: &DevicePath) -> DisplayKind {
        let Some(card) = path.as_path().file_name() else {
            return DisplayKind::Gpu;
        };
        let device = self.root.sys().join("class/drm").join(card).join("device");
        let driver = link_name(&device.join("driver"));
        let platform = link_name(&device);
        let bus = link_name(&device.join("subsystem"));
        DisplayKind::of_names(driver.as_deref(), platform.as_deref(), bus.as_deref())
    }
}

/// The final component of the link at `path`.
fn link_name(path: &Path) -> Option<String> {
    let target = std::fs::read_link(path).ok()?;
    Some(target.file_name()?.to_str()?.to_owned())
}
