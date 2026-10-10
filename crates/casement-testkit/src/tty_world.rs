//! The fakes of one tty seat, held by a test beside the compositor that runs over them (P1-T):
//! the session, udev, a display device, the input devices, the GPUs and their formats.
//! `Fixture::start_tty` takes the seams [`TtyWorld::seams`] builds and the test keeps the world to
//! drive the seat: switch VT, suspend, plug a monitor, press a key, then read back what the
//! compositor did (the commits, the frames queued, the gamma loaded).

use crate::fakes::{
    FakeInputFeed, FakeKms, FakeKmsOpener, FakeSession, FakeUdev, SessionFixture, SharedSession,
    SharedUdev,
};
use casement_backend::gpu::{FormatSource, RenderNodes};
use casement_backend::tty::{DeviceEvent, DevicePath, TtySeams};
use casement_core::gpu::GpuChoice;
use std::path::PathBuf;

/// The one display device a world has.
pub const CARD: &str = "/dev/dri/card0";

/// The fakes of one seat. A clone of each is what the compositor holds.
#[derive(Debug, Clone)]
pub struct TtyWorld {
    /// The seat session.
    pub session: SharedSession,
    /// The hotplug stream.
    pub udev: SharedUdev,
    /// The display device at [`CARD`].
    pub kms: FakeKms,
    /// The input devices.
    pub input: FakeInputFeed,
}

impl TtyWorld {
    /// A seat with `kms` as its one display device, present from the start. The session hears
    /// a suspend through its [`SleepLink`](casement_backend::tty::SleepLink).
    ///
    /// ```
    /// use casement_testkit::TtyWorld;
    /// use casement_testkit::fakes::{FakeKms, SyntheticTopology};
    ///
    /// let world = TtyWorld::new(FakeKms::synthetic(SyntheticTopology::Dock));
    /// assert_eq!(world.kms.seat_changes(), (0, 0));
    /// ```
    pub fn new(kms: FakeKms) -> TtyWorld {
        let card = DevicePath(PathBuf::from(CARD));
        let session = FakeSession::with(vec![SessionFixture {
            path: card.0.clone(),
            bytes: Vec::new(),
        }])
        .with_sleep();
        TtyWorld {
            session: SharedSession::new(session),
            udev: SharedUdev::new(FakeUdev::default().with_present(vec![card])),
            kms,
            input: FakeInputFeed::new(),
        }
    }

    /// A monitor is plugged on the device: udev reports it changed.
    pub fn changed(&self) {
        self.udev
            .push(DeviceEvent::Changed(DevicePath(PathBuf::from(CARD))));
    }

    /// The seams the compositor starts over: this world's session, udev, display device and
    /// input, with `nodes` as the machine's GPUs and `formats` as what they draw into.
    pub fn seams(
        &self,
        nodes: Box<dyn RenderNodes>,
        formats: Box<dyn FormatSource>,
        choice: GpuChoice,
    ) -> TtySeams {
        let mut opener = FakeKmsOpener::default();
        opener
            .devices
            .insert(DevicePath(PathBuf::from(CARD)), self.kms.clone());
        TtySeams::new(
            Box::new(self.session.clone()),
            Box::new(self.udev.clone()),
            Box::new(opener),
            nodes,
            choice,
        )
        .with_formats(formats)
        .with_input(Box::new(self.input.clone()))
    }
}
