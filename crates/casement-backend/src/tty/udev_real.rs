//! The real udev monitor (feature `tty`): `DeviceSource` over the device crates, a thin
//! adapter (B7). P1-Z declares it; P1-D1 fills it.

use super::udev::{DeviceEvent, DevicePath, DeviceSource};

/// The udev monitor on the session's seat.
#[derive(Debug)]
pub struct RealUdev {
    /// The seat it watches.
    pub seat: String,
}

impl RealUdev {
    /// Starts watching the DRM devices of `seat`.
    pub fn watch(seat: &str) -> Result<RealUdev, std::io::Error> {
        let _ = seat;
        todo!("P1-D1: RealUdev::watch")
    }
}

impl DeviceSource for RealUdev {
    fn present(&self) -> Vec<DevicePath> {
        todo!("P1-D1: RealUdev::present")
    }

    fn next_event(&mut self) -> Option<DeviceEvent> {
        todo!("P1-D1: RealUdev::next_event")
    }
}
