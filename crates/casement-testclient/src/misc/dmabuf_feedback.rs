//! zwp_linux_dmabuf_v1 default feedback (P0-H6): a probe that binds the global, asks for the
//! default feedback and collects what the compositor says, so a test can assert the main
//! device and the tranches. A self-contained helper over a [`Connection`] and the registry's
//! [`GlobalList`] with its own event queue: P0-T-client's `TestClient` core was not there
//! when this was written, and the probe asks it for nothing but those two handles. The
//! udmabuf-backed buffers live in `crate::udmabuf` (P0-T-client's).

use crate::globals::GlobalList;
use std::fs::File;
use std::os::fd::OwnedFd;
use std::os::unix::fs::FileExt;
use wayland_client::{Connection, Dispatch, EventQueue, Proxy, QueueHandle};
use wayland_protocols::wp::linux_dmabuf::zv1::client::{
    zwp_linux_dmabuf_feedback_v1::{self, ZwpLinuxDmabufFeedbackV1},
    zwp_linux_dmabuf_v1::{self, ZwpLinuxDmabufV1},
};

/// The version of `zwp_linux_dmabuf_v1` the global is advertised at, if it is advertised.
pub fn advertised_version(globals: &GlobalList) -> Option<u32> {
    globals.contents().with_list(|list| {
        list.iter()
            .find(|global| global.interface == ZwpLinuxDmabufV1::interface().name)
            .map(|global| global.version)
    })
}

/// One format and modifier of the format table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TableRow {
    /// The fourcc code.
    pub format: u32,
    /// The modifier.
    pub modifier: u64,
}

/// One tranche of the feedback.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Tranche {
    /// Its target device's `dev_t`.
    pub target_device: Option<u64>,
    /// Its flags, as sent.
    pub flags: u32,
    /// Its formats, looked up in the table.
    pub formats: Vec<TableRow>,
}

/// What a finished feedback said.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FeedbackReport {
    /// The main device's `dev_t`, when the event was sent (versions 4 and 5).
    pub main_device: Option<u64>,
    /// Every tranche, in order.
    pub tranches: Vec<Tranche>,
}

impl FeedbackReport {
    /// The device the compositor names: `main_device` (sent up to version 5), else the first
    /// tranche's target (version 6 dropped the event).
    pub fn device(&self) -> Option<u64> {
        self.main_device.or_else(|| {
            self.tranches
                .first()
                .and_then(|tranche| tranche.target_device)
        })
    }
}

/// Why the probe could not start.
#[derive(Debug, thiserror::Error)]
pub enum ProbeError {
    /// The compositor does not advertise a dma-buf global of version 4 or later.
    #[error("zwp_linux_dmabuf_v1 version 4+ is not advertised")]
    NotAdvertised,
}

/// The collecting state the event queue dispatches into.
#[derive(Debug, Default)]
struct Collected {
    table: Vec<TableRow>,
    open: Tranche,
    building: FeedbackReport,
    done: Option<FeedbackReport>,
}

/// A bound `zwp_linux_dmabuf_v1` with a default-feedback object, on its own event queue.
#[derive(Debug)]
pub struct FeedbackProbe {
    queue: EventQueue<Collected>,
    collected: Collected,
    #[expect(dead_code, reason = "held so the objects live as long as the probe")]
    dmabuf: ZwpLinuxDmabufV1,
    #[expect(dead_code, reason = "held so the objects live as long as the probe")]
    feedback: ZwpLinuxDmabufFeedbackV1,
}

impl FeedbackProbe {
    /// Binds the global and requests its default feedback. The request is flushed; the events
    /// arrive when the connection is next read (the fixture's pump) and [`FeedbackProbe::report`]
    /// dispatches them.
    pub fn start(
        connection: &Connection,
        globals: &GlobalList,
    ) -> Result<FeedbackProbe, ProbeError> {
        let queue = connection.new_event_queue::<Collected>();
        let qh = queue.handle();
        let dmabuf = globals
            .bind::<ZwpLinuxDmabufV1, _, _>(&qh, 4..=6, ())
            .map_err(|_| ProbeError::NotAdvertised)?;
        let feedback = dmabuf.get_default_feedback(&qh, ());
        let _ = connection.flush();
        Ok(FeedbackProbe {
            queue,
            collected: Collected::default(),
            dmabuf,
            feedback,
        })
    }

    /// Dispatches what has arrived; the report once the feedback said `done`.
    pub fn report(&mut self) -> Option<FeedbackReport> {
        let _ = self.queue.dispatch_pending(&mut self.collected);
        self.collected.done.clone()
    }
}

impl Dispatch<ZwpLinuxDmabufV1, ()> for Collected {
    fn event(
        _: &mut Collected,
        _: &ZwpLinuxDmabufV1,
        _: zwp_linux_dmabuf_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Collected>,
    ) {
    }
}

impl Dispatch<ZwpLinuxDmabufFeedbackV1, ()> for Collected {
    fn event(
        state: &mut Collected,
        _: &ZwpLinuxDmabufFeedbackV1,
        event: zwp_linux_dmabuf_feedback_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Collected>,
    ) {
        use zwp_linux_dmabuf_feedback_v1::Event;
        match event {
            Event::FormatTable { fd, size } => state.table = read_table(fd, size),
            Event::MainDevice { device } => state.building.main_device = device_of(&device),
            Event::TrancheTargetDevice { device } => state.open.target_device = device_of(&device),
            Event::TrancheFlags { flags } => {
                state.open.flags = flags.into_result().map_or(0, |flags| flags.bits());
            }
            Event::TrancheFormats { indices } => {
                state.open.formats = indices
                    .as_chunks::<2>()
                    .0
                    .iter()
                    .filter_map(|pair| state.table.get(usize::from(u16::from_ne_bytes(*pair))))
                    .copied()
                    .collect();
            }
            Event::TrancheDone => {
                let tranche = std::mem::take(&mut state.open);
                state.building.tranches.push(tranche);
            }
            Event::Done => state.done = Some(std::mem::take(&mut state.building)),
            _ => {}
        }
    }
}

/// A `dev_t` from the event's native-endian bytes.
fn device_of(bytes: &[u8]) -> Option<u64> {
    <[u8; 8]>::try_from(bytes).ok().map(u64::from_ne_bytes)
}

/// The format table: 16 bytes per row (format u32, padding u32, modifier u64), read with
/// `read_exact_at` (no mmap, so no `unsafe`).
fn read_table(fd: OwnedFd, size: u32) -> Vec<TableRow> {
    let mut bytes = vec![0u8; size as usize];
    if File::from(fd).read_exact_at(&mut bytes, 0).is_err() {
        return Vec::new();
    }
    bytes
        .as_chunks::<16>()
        .0
        .iter()
        .filter_map(|row| {
            Some(TableRow {
                format: u32::from_ne_bytes(row[0..4].try_into().ok()?),
                modifier: u64::from_ne_bytes(row[8..16].try_into().ok()?),
            })
        })
        .collect()
}
