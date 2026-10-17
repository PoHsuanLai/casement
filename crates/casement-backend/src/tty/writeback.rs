//! KMS writeback (P1-V2): reading one frame of what a CRTC scans out, back through the
//! device's writeback connector. Only the `--drm-node` test mode on the vkms device turns it
//! on (`dev/vkms/accept.sh` compares the frame with the pixman golden).
//!
//! [`Writeback`] is the seam. Real: `writeback_real.rs` over the device crates; fake:
//! `casement_testkit::fakes::FakeWriteback`. A job is two calls: [`Writeback::begin`] queues
//! the job on the next commit (a dumb buffer is attached to the writeback connector), and
//! [`Writeback::finish`] reads the buffer once the device has composed a frame into it. A
//! device tells nothing about when that is without a fence, so the backend lets
//! [`FLIPS_TO_WAIT`] flips of the output pass between the two
//! ([`TtyBackend::request_writeback`], [`TtyBackend::take_writeback`]).
//!
//! The raw buffer is written to a file next to a one-line description, `W H PITCH FOURCC`
//! ([`RawCapture::meta_line`]); `casement/src/test_mode.rs` does the writing.
//!
//! ```
//! use casement_backend::tty::RawCapture;
//! use casement_core::geometry::PhysicalSize;
//! use casement_core::gpu::Fourcc;
//!
//! let frame = RawCapture::new(PhysicalSize { w: 4, h: 2 }, 16, Fourcc(0x3432_5258), vec![0; 32]);
//! assert_eq!(frame.meta_line(), "4 2 16 XR24");
//! ```

use super::kms::CrtcId;
use super::{SeatState, TtyBackend};
use casement_core::OutputName;
use casement_core::geometry::PhysicalSize;
use casement_core::gpu::Fourcc;
use casement_core::output::OutputPower;

/// How many flips of the output pass between queueing a writeback job and reading it: the
/// device composes the job's frame at the next vblank, and one more is slack.
pub const FLIPS_TO_WAIT: u64 = 3;

/// One frame read back: the buffer exactly as the device wrote it.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct RawCapture {
    /// The frame's size in pixels.
    pub size: PhysicalSize,
    /// Bytes from one row's start to the next.
    pub pitch: u32,
    /// The pixel format.
    pub format: Fourcc,
    /// The rows, `pitch` bytes each.
    pub bytes: Vec<u8>,
}

impl RawCapture {
    /// A frame of `size` whose rows are `pitch` bytes, in `format`.
    pub fn new(size: PhysicalSize, pitch: u32, format: Fourcc, bytes: Vec<u8>) -> RawCapture {
        RawCapture {
            size,
            pitch,
            format,
            bytes,
        }
    }

    /// `WIDTH HEIGHT PITCH FOURCC`, the fourcc spelled as its four letters (`XR24`).
    pub fn meta_line(&self) -> String {
        let letters = self.format.0.to_le_bytes().map(|byte| match byte {
            0x20..=0x7e => char::from(byte),
            _ => '?',
        });
        format!(
            "{} {} {} {}",
            self.size.w,
            self.size.h,
            self.pitch,
            letters.iter().collect::<String>()
        )
    }
}

/// Why a writeback call failed.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum WritebackError {
    /// The device returned an I/O error.
    #[error("writeback: {0}")]
    Io(#[from] std::io::Error),
    /// The device has no writeback connector.
    #[error("the device has no writeback connector")]
    NoConnector,
    /// The device has no such CRTC.
    #[error("the device has no CRTC {0:?}")]
    NoCrtc(CrtcId),
    /// `finish` without a job.
    #[error("no writeback job is pending")]
    NotBegun,
    /// `begin` while a job is pending.
    #[error("a writeback job is already pending")]
    Busy,
    /// This run has no writeback (it is only available in `--drm-node` mode).
    #[error("writeback is not enabled in this mode")]
    NotEnabled,
    /// The seat is lent away: the device is not ours to commit on.
    #[error("the seat is lent away")]
    SeatLent,
    /// No output is lit, so there is nothing to read back.
    #[error("no output is lit")]
    NoOutput,
}

/// The writeback connector of one display device.
pub trait Writeback: std::fmt::Debug {
    /// Queues a job that reads the frame `crtc` (scanning out `size` pixels) shows next.
    fn begin(&mut self, crtc: CrtcId, size: PhysicalSize) -> Result<(), WritebackError>;

    /// The frame of the job `begin` queued, once the device has composed it; the job's
    /// resources are released.
    fn finish(&mut self) -> Result<RawCapture, WritebackError>;
}

/// Whether the real adapter is asked to build a writeback seam.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum WritebackMode {
    /// No writeback connector is touched.
    Disabled,
    /// The device's writeback connector is used.
    Enabled,
}

/// Where a backend's writeback job stands.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(super) enum WritebackJob {
    /// Nothing queued.
    #[default]
    Idle,
    /// Queued on `output`; ready once it has shown `ready_at` flips.
    Waiting {
        /// The output whose CRTC the job reads.
        output: OutputName,
        /// Its flip count at which the frame is there.
        ready_at: u64,
    },
}

impl TtyBackend {
    /// Queues one writeback job on the first lit output (SIGHUP in `--drm-node` mode). Refused
    /// while the seat is lent, while another job is pending, when no output is lit, and when the
    /// run has no writeback seam.
    pub fn request_writeback(&mut self) -> Result<(), WritebackError> {
        if self.seams.writeback.is_none() {
            return Err(WritebackError::NotEnabled);
        }
        if self.seat != SeatState::Held {
            return Err(WritebackError::SeatLent);
        }
        if self.writeback_job != WritebackJob::Idle {
            return Err(WritebackError::Busy);
        }
        let (output, crtc, size) = self
            .lit
            .iter()
            .find(|(_, lit)| lit.power() == OutputPower::On)
            .map(|(name, lit)| (name.clone(), lit.crtc, lit.config.mode.size))
            .ok_or(WritebackError::NoOutput)?;
        let seam = self
            .seams
            .writeback
            .as_mut()
            .ok_or(WritebackError::NotEnabled)?;
        seam.begin(crtc, size)?;
        let shown = self.output_flips(&output);
        self.writeback_job = WritebackJob::Waiting {
            output,
            ready_at: shown.saturating_add(FLIPS_TO_WAIT),
        };
        Ok(())
    }

    /// The frame of the queued job, once the output has flipped enough times; none while it
    /// has not (and when no job is queued).
    pub fn take_writeback(&mut self) -> Option<Result<RawCapture, WritebackError>> {
        let WritebackJob::Waiting { output, ready_at } = &self.writeback_job else {
            return None;
        };
        if self.output_flips(output) < *ready_at && self.scanouts.get(output).is_some() {
            return None;
        }
        self.writeback_job = WritebackJob::Idle;
        let seam = self.seams.writeback.as_mut()?;
        Some(seam.finish())
    }

    /// The flips `output` has shown (zero for an output with no scanout).
    fn output_flips(&self, output: &OutputName) -> u64 {
        self.scanouts
            .get(output)
            .map_or(0, |scanout| scanout.tally().presented)
    }
}
