//! Which display devices are worth driving: a firmware framebuffer (simpledrm and its kin) is a
//! boot-time stand-in for a GPU and must not become an output while a real display GPU is there,
//! and a virtual test display (vkms) is never an output of a session casement.
//!
//! Detection (the backend's sysfs adapter) turns a device into a [`DisplayKind`]; [`keep_displays`]
//! is the pure rule over what was found.
//!
//! ```
//! use casement_core::gpu::{DisplayKind, keep_displays};
//!
//! let found = [("card0", DisplayKind::Firmware), ("card2", DisplayKind::Gpu)];
//! assert_eq!(keep_displays(&found), ["card2"]);
//! ```

/// What drives a display device.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[non_exhaustive]
pub enum DisplayKind {
    /// A GPU driver (amdgpu, nvidia-drm, i915, a display controller): a real screen.
    Gpu,
    /// A firmware or boot framebuffer (simpledrm, efidrm, vesadrm, ofdrm): one fixed mode, no
    /// render node, usually a phantom once the GPU driver has loaded.
    Firmware,
    /// A virtual test display (vkms): driven only by `--drm-node`, never by a session.
    Virtual,
}

/// The kernel drivers that are firmware framebuffers, by the name sysfs shows under
/// `device/driver`.
const FIRMWARE_DRIVERS: [&str; 6] = [
    "simpledrm",
    "simple-drm",
    "simple-framebuffer",
    "efidrm",
    "vesadrm",
    "ofdrm",
];

/// The bus a configfs-made vkms device sits on (kernel 7.x): its bound driver is `faux_driver`
/// and the DRM driver name is only visible through the version ioctl.
const VIRTUAL_BUS: &str = "faux";

/// The DRM driver name of the virtual test display.
const VIRTUAL_DRIVER: &str = "vkms";

impl DisplayKind {
    /// The kind of the device bound to `driver` (a name from `device/driver`), whose platform
    /// device is called `device` (`simple-framebuffer.0`) and which sits on `bus` (the name of
    /// its `subsystem` link); each may be absent.
    pub fn of_names(driver: Option<&str>, device: Option<&str>, bus: Option<&str>) -> DisplayKind {
        if bus == Some(VIRTUAL_BUS) || driver == Some(VIRTUAL_DRIVER) {
            return DisplayKind::Virtual;
        }
        let firmware = driver.is_some_and(|name| FIRMWARE_DRIVERS.contains(&name))
            || device.is_some_and(|name| {
                let stem = name.split('.').next().unwrap_or(name);
                FIRMWARE_DRIVERS.contains(&stem)
            });
        match firmware {
            true => DisplayKind::Firmware,
            false => DisplayKind::Gpu,
        }
    }
}

/// The devices of `found` to drive, in order: never a virtual display; every other one when
/// none is a firmware framebuffer, the real GPUs when any is, and the firmware ones only when
/// nothing else is there.
pub fn keep_displays<P: Clone>(found: &[(P, DisplayKind)]) -> Vec<P> {
    let any_gpu = found.iter().any(|(_, kind)| *kind == DisplayKind::Gpu);
    found
        .iter()
        .filter(|(_, kind)| *kind != DisplayKind::Virtual)
        .filter(|(_, kind)| !any_gpu || *kind == DisplayKind::Gpu)
        .map(|(path, _)| path.clone())
        .collect()
}
