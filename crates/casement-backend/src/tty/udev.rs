//! The udev seam (P1-Z; filled by P1-D1): which DRM devices exist and when they change.

use std::path::{Path, PathBuf};

/// A device node's path (`/dev/dri/card0`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct DevicePath(pub PathBuf);

impl DevicePath {
    /// The path as a `Path`.
    pub fn as_path(&self) -> &Path {
        &self.0
    }
}

/// One device event.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum DeviceEvent {
    /// A device appeared.
    Added(DevicePath),
    /// A device changed (a connector's hotplug).
    Changed(DevicePath),
    /// A device went away.
    Removed(DevicePath),
}

/// A stream of device events. Real: `udev_real.rs`; fake: `casement_testkit::fakes::FakeUdev`.
///
/// ```no_run
/// use casement_backend::tty::DeviceSource;
///
/// fn drain(source: &mut dyn DeviceSource) -> usize {
///     std::iter::from_fn(|| source.next_event()).count()
/// }
/// ```
pub trait DeviceSource: std::fmt::Debug {
    /// The display devices present when casement started.
    fn present(&self) -> Vec<DevicePath>;

    /// The next event, if one is waiting.
    fn next_event(&mut self) -> Option<DeviceEvent>;
}
