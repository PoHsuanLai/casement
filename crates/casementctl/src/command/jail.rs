//! The structural jail check (20-FREEZE-PLAN §6.1) for this process: the conditions are
//! `casement_privacy::breach`'s. casementctl never opens a device.

use casement_privacy::{JailBreach, breach};
use std::os::unix::fs::MetadataExt;
use std::path::Path;

/// [`breach`] for this process on the real root.
pub fn breach_here() -> Option<JailBreach> {
    let uid = std::fs::metadata("/proc/self").map_or(0, |m| m.uid());
    breach(Path::new("/"), uid)
}
