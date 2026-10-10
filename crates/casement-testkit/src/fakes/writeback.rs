//! The fake writeback connector (P1-V2): [`FakeWriteback`] stands in for the vkms writeback
//! connector. A job it is given returns a frame of the asked size filled with a known pattern
//! ([`FakeWriteback::pattern`]), so a test can tell the bytes came through.

use casement_backend::tty::{CrtcId, RawCapture, Writeback, WritebackError};
use casement_core::geometry::PhysicalSize;
use casement_core::gpu::Fourcc;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

/// `XRGB8888`, the format a vkms writeback connector offers.
const XRGB8888: Fourcc = Fourcc(0x3432_5258);

/// What the fake has been asked.
#[derive(Debug, Default)]
pub struct WritebackWorld {
    /// Every job begun, in order.
    pub begun: Vec<(CrtcId, PhysicalSize)>,
    /// The job waiting to be read.
    pending: Option<PhysicalSize>,
    /// How many frames were read.
    pub finished: u32,
}

/// A writeback connector a backend and a test both hold.
///
/// ```
/// use casement_backend::tty::{CrtcId, Writeback};
/// use casement_core::geometry::PhysicalSize;
/// use casement_testkit::fakes::FakeWriteback;
///
/// let mut fake = FakeWriteback::default();
/// let size = PhysicalSize { w: 2, h: 2 };
/// fake.begin(CrtcId(10), size).expect("begun");
/// let frame = fake.finish().expect("read");
/// assert_eq!(frame.bytes, FakeWriteback::pattern(size));
/// ```
#[derive(Debug, Clone, Default)]
pub struct FakeWriteback(Arc<Mutex<WritebackWorld>>);

impl FakeWriteback {
    /// What the fake has been asked so far.
    pub fn world(&self) -> MutexGuard<'_, WritebackWorld> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// The bytes of a frame of `size`: 4 bytes a pixel, the byte at `n` being `n % 251`.
    pub fn pattern(size: PhysicalSize) -> Vec<u8> {
        let len = (size.w as usize) * (size.h as usize) * 4;
        (0..len).map(|n| (n % 251) as u8).collect()
    }
}

impl Writeback for FakeWriteback {
    fn begin(&mut self, crtc: CrtcId, size: PhysicalSize) -> Result<(), WritebackError> {
        let mut world = self.world();
        if world.pending.is_some() {
            return Err(WritebackError::Busy);
        }
        world.begun.push((crtc, size));
        world.pending = Some(size);
        Ok(())
    }

    fn finish(&mut self) -> Result<RawCapture, WritebackError> {
        let mut world = self.world();
        let size = world.pending.take().ok_or(WritebackError::NotBegun)?;
        world.finished += 1;
        Ok(RawCapture::new(
            size,
            size.w * 4,
            XRGB8888,
            FakeWriteback::pattern(size),
        ))
    }
}
