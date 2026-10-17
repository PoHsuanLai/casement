//! `zwp_linux_dmabuf_v1`: GPU buffers. No global until the backend offers a main device
//! (`--dmabuf soft` in the jail, a render node from P1); `--dmabuf none` advertises nothing.
//! Every imported buffer is tagged with the node of the feedback tranche it came from (A5):
//! the tag is a smithay `DrmNode`, a device type this crate may not name (B6), so the
//! composition root's handler applies it from the [`MainDevice`] that
//! [`DmabufPart::source_device`] reports. The protocol does not say which tranche a client
//! allocated from, so a buffer's tranche is the one that offers its format and modifier; the
//! plan's `feedback_for` offers each pair once, so the answer is unambiguous.

use crate::states::{GlobalFilter, WaylandFault};
use casement_core::gpu::{FeedbackPlan, TrancheUse};
use casement_core::policy::GlobalKind;
use smithay::backend::allocator::dmabuf::Dmabuf;
use smithay::backend::allocator::{Buffer, Format, Fourcc, Modifier};
use smithay::reexports::rustix::fs::{SeekFrom, seek};
use std::os::fd::{AsRawFd, BorrowedFd};
use smithay::reexports::wayland_protocols::wp::linux_dmabuf::zv1::server::zwp_linux_dmabuf_feedback_v1::TrancheFlags;
use smithay::reexports::wayland_protocols::wp::linux_dmabuf::zv1::server::zwp_linux_dmabuf_v1::ZwpLinuxDmabufV1;
use smithay::reexports::wayland_server::{DisplayHandle, GlobalDispatch};
use smithay::wayland::buffer::BufferHandler;
use smithay::wayland::dmabuf::{
    DmabufFeedbackBuilder, DmabufGlobal, DmabufGlobalData, DmabufHandler, DmabufState,
};

/// `zwp_linux_dmabuf_v1`, once offered.
#[derive(Debug)]
pub struct DmabufPart {
    /// smithay's state.
    pub state: DmabufState,
    /// The global, once [`apply_offer`] created it.
    pub global: Option<DmabufGlobal>,
    /// The filter the global is created with.
    pub filter: GlobalFilter,
    /// What the global was created from; none while nothing is advertised.
    pub offered: Option<DmabufOffer>,
}

impl DmabufPart {
    /// The part without a global.
    pub fn new(role_filter: GlobalFilter) -> DmabufPart {
        DmabufPart {
            state: DmabufState::new(),
            global: None,
            filter: role_filter,
            offered: None,
        }
    }

    /// The offered main device.
    pub fn main_device(&self) -> Option<MainDevice> {
        self.offered.as_ref().map(|offer| offer.main_device)
    }

    /// Every format and modifier the global accepts from clients, over all tranches, each once
    /// in the order offered; none while nothing is advertised.
    pub fn accepted(&self) -> Vec<Format> {
        let mut seen = Vec::new();
        for format in self
            .offered
            .iter()
            .flat_map(|offer| offer.tranche_list())
            .flat_map(|tranche| tranche.formats)
        {
            if !seen.contains(&format) {
                seen.push(format);
            }
        }
        seen
    }

    /// The device a buffer of `format` is tagged with (A5): the target of the tranche that
    /// offers it, or the main device for a format no tranche names.
    pub fn source_device(&self, format: Format) -> Option<MainDevice> {
        let offer = self.offered.as_ref()?;
        let tranche = offer
            .tranche_list()
            .into_iter()
            .find(|tranche| tranche.formats.contains(&format));
        Some(tranche.map_or(offer.main_device, |tranche| tranche.device))
    }
}

/// A device's `dev_t` (glibc's encoding: major and minor split across the word).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct MainDevice(pub u64);

impl MainDevice {
    /// The device with this major and minor, as glibc's `makedev` encodes them.
    pub const fn of(major: u32, minor: u32) -> MainDevice {
        let (major, minor) = (major as u64, minor as u64);
        MainDevice(
            ((major & 0xfff) << 8)
                | (minor & 0xff)
                | ((major & !0xfff) << 32)
                | ((minor & !0xff) << 12),
        )
    }
}

/// The jail's first fabricated render node, `226:128` (A3): the soft main device.
pub const SOFT_MAIN_DEVICE: MainDevice = MainDevice::of(226, 128);

/// What a tranche is for, as the feedback flags it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum OfferUse {
    /// The device samples (composites) buffers of these formats.
    Sampling,
    /// A plane scans buffers of these formats out directly.
    Scanout,
}

/// One tranche of the feedback: a device and its formats, in the order offered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OfferTranche {
    /// The tranche's target device.
    pub device: MainDevice,
    /// What the tranche is for.
    pub purpose: OfferUse,
    /// Its formats and modifiers.
    pub formats: Vec<Format>,
}

/// What the backend offers clients: its main device and the formats it imports, plus the
/// tranches of a multi-GPU plan.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct DmabufOffer {
    /// The main device.
    pub main_device: MainDevice,
    /// The formats and modifiers it imports.
    pub formats: Vec<Format>,
    /// The tranches, most preferred first. Empty means one tranche: `formats` on the main
    /// device.
    pub tranches: Vec<OfferTranche>,
}

impl DmabufOffer {
    /// The offer a `FeedbackPlan` describes (P1-D4): its main device, and its tranches in
    /// order. A fourcc smithay has no name for is left out.
    ///
    /// ```
    /// use casement_core::gpu::{DrmNodeId, FormatTable, RenderPlan, feedback_for};
    /// use casement_wayland::dmabuf::{DmabufOffer, MainDevice};
    /// use std::collections::BTreeMap;
    ///
    /// let render = DrmNodeId::new(226, 128);
    /// let plan = RenderPlan::new(render, BTreeMap::new());
    /// let offer = DmabufOffer::from_feedback(&feedback_for(&plan, &FormatTable::default()));
    /// assert_eq!(offer.main_device, MainDevice::of(226, 128));
    /// ```
    pub fn from_feedback(feedback: &FeedbackPlan) -> DmabufOffer {
        let main_device = MainDevice::of(feedback.main_device.major, feedback.main_device.minor);
        let tranches: Vec<OfferTranche> = feedback
            .tranches
            .iter()
            .map(|tranche| OfferTranche {
                device: MainDevice::of(tranche.target.major, tranche.target.minor),
                purpose: match tranche.purpose {
                    TrancheUse::Scanout => OfferUse::Scanout,
                    _ => OfferUse::Sampling,
                },
                formats: tranche
                    .formats
                    .iter()
                    .filter_map(|entry| {
                        Some(Format {
                            code: Fourcc::try_from(entry.format.0).ok()?,
                            modifier: Modifier::from(entry.modifier.0),
                        })
                    })
                    .collect(),
            })
            .collect();
        let formats = tranches
            .iter()
            .filter(|tranche| tranche.device == main_device)
            .flat_map(|tranche| tranche.formats.iter().copied())
            .collect();
        DmabufOffer {
            main_device,
            formats,
            tranches,
        }
    }

    /// The tranches as they are sent: `tranches`, or the main device's `formats` alone.
    fn tranche_list(&self) -> Vec<OfferTranche> {
        if self.tranches.is_empty() {
            vec![OfferTranche {
                device: self.main_device,
                purpose: OfferUse::Sampling,
                formats: self.formats.clone(),
            }]
        } else {
            self.tranches.clone()
        }
    }

    /// `--dmabuf soft`: the fake main device over `/dev/udmabuf`, linear ARGB8888 and XRGB8888
    /// (the topology's `FormatSet::LinearOnly`).
    pub fn soft() -> DmabufOffer {
        DmabufOffer {
            main_device: SOFT_MAIN_DEVICE,
            formats: [Fourcc::Argb8888, Fourcc::Xrgb8888]
                .into_iter()
                .map(|code| Format {
                    code,
                    modifier: Modifier::Linear,
                })
                .collect(),
            tranches: Vec::new(),
        }
    }
}

/// The largest a client dma-buf may be on either side: 16384 is the biggest texture the GPUs
/// casement targets sample, so a buffer past it could only fail on every frame it is drawn.
pub const MAX_DMABUF_SIDE: u32 = 16384;

/// Whether an imported buffer was accepted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Import {
    /// The renderer can sample it.
    Accepted,
    /// It cannot.
    Refused,
}

/// A client imported `dmabuf`: accepted when something is advertised and the buffer's format
/// and modifier are among the formats of any offered tranche. The handler tags an accepted
/// buffer with [`DmabufPart::source_device`]'s node (A5).
///
/// A buffer is refused too when a side is past [`MAX_DMABUF_SIDE`], or when it is linear and a
/// plane reaches past the end of its file (CASW-17): the protocol would say "created" for a
/// buffer the GPU can then only fail to import, on every frame, or read past with attacker data.
pub fn on_import(part: &mut DmabufPart, dmabuf: &mut Dmabuf) -> Import {
    if dmabuf.width() > MAX_DMABUF_SIDE || dmabuf.height() > MAX_DMABUF_SIDE {
        return Import::Refused;
    }
    if dmabuf.format().modifier == Modifier::Linear && !linear_planes_fit(dmabuf) {
        return Import::Refused;
    }
    let format = dmabuf.format();
    let offered = part.offered.as_ref().is_some_and(|offer| {
        offer
            .tranche_list()
            .iter()
            .any(|tranche| tranche.formats.contains(&format))
    });
    if offered {
        Import::Accepted
    } else {
        Import::Refused
    }
}

/// Whether every plane of the linear `dmabuf` (its rows at its stride, from its offset) lies
/// inside the file it sits in; a file whose size cannot be read is a plane that does not fit.
fn linear_planes_fit(dmabuf: &Dmabuf) -> bool {
    let planes: Vec<Plane> = dmabuf
        .offsets()
        .zip(dmabuf.strides())
        .map(|(offset, stride)| Plane { offset, stride })
        .collect();
    let sizes: Vec<Option<u64>> = dmabuf.handles().map(memory_file_size).collect();
    planes_fit(&planes, &sizes, dmabuf.height())
}

/// The size of `fd`, when it is a file in kernel memory (a dma-buf, or a memory file standing in
/// for one); nothing for any other, and no call is made on it. `lseek` on a file of a FUSE or a
/// network mount asks the file system's daemon, which can be a program of the client's own that
/// never answers, on the loop (NEW-CAS-3). The kind is read from the descriptor's link name in
/// `/proc/self/fd`, a read that reaches no file system.
fn memory_file_size(fd: BorrowedFd<'_>) -> Option<u64> {
    let link = std::fs::read_link(format!("/proc/self/fd/{}", fd.as_raw_fd())).ok()?;
    let in_memory = is_memory_file(&link.to_string_lossy());
    in_memory.then(|| seek(fd, SeekFrom::End(0)).ok()).flatten()
}

/// Whether a descriptor's link name (`/proc/self/fd/N`) is that of a dma-buf or a memory file.
fn is_memory_file(link: &str) -> bool {
    link.starts_with("/dmabuf:") || link.starts_with("/memfd:")
}

/// Where a plane starts in its file and how far apart its rows are.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Plane {
    offset: u32,
    stride: u32,
}

/// Whether `planes` of a buffer `height` rows tall lie inside files of `sizes` bytes (none: the
/// size could not be read). The first plane has every row; any other has at least half of them
/// (a subsampled chroma plane), so this never refuses a valid buffer. The arithmetic is wide,
/// so a stride of `u32::MAX` cannot overflow into a small number.
fn planes_fit(planes: &[Plane], sizes: &[Option<u64>], height: u32) -> bool {
    planes.len() == sizes.len()
        && planes
            .iter()
            .zip(sizes)
            .enumerate()
            .all(|(n, (plane, size))| {
                let rows = match n {
                    0 => u64::from(height),
                    _ => u64::from(height).div_ceil(2),
                };
                let end = u64::from(plane.offset) + u64::from(plane.stride) * rows;
                size.is_some_and(|size| end <= size)
            })
}

/// Creates (or replaces) the global with default feedback for `offer`, filtered by the I7
/// table (`GlobalKind::Dmabuf`). A feedback table that cannot be built is logged and nothing
/// is advertised (clients fall back to shm).
pub fn apply_offer<D>(
    part: &mut DmabufPart,
    display: &DisplayHandle,
    offer: &DmabufOffer,
) -> Result<(), WaylandFault>
where
    D: GlobalDispatch<ZwpLinuxDmabufV1, DmabufGlobalData> + BufferHandler + DmabufHandler + 'static,
{
    // Every tranche is a preference tranche, added in order: smithay sends the "main" tranche
    // of `new` after all of them, which would put the render GPU's formats last.
    let builder = offer.tranche_list().into_iter().fold(
        DmabufFeedbackBuilder::new(offer.main_device.0, std::iter::empty::<Format>()),
        |builder, tranche| {
            let flags = match tranche.purpose {
                OfferUse::Scanout => TrancheFlags::Scanout,
                _ => TrancheFlags::Sampling,
            };
            builder.add_preference_tranche(tranche.device.0, flags, tranche.formats, 3u32..=6)
        },
    );
    let feedback = match builder.build() {
        Ok(feedback) => feedback,
        Err(error) => {
            tracing::error!(%error, "dmabuf feedback table not built; no dma-buf offered");
            return Ok(());
        }
    };
    if let Some(old) = part.global.take() {
        part.state.disable_global::<D>(display, &old);
        part.state.destroy_global::<D>(display, old);
    }
    let filter = part.filter.for_global(GlobalKind::Dmabuf);
    part.global = Some(
        part.state
            .create_global_with_filter_and_default_feedback::<D, _>(display, &feedback, filter),
    );
    part.offered = Some(offer.clone());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use smithay::backend::allocator::dmabuf::DmabufFlags;
    use smithay::reexports::rustix::fs::{MemfdFlags, ftruncate, memfd_create};
    use std::os::fd::OwnedFd;

    /// NEW-CAS-3: only kernel-memory files are asked their size; a file of a mounted file system
    /// (whose daemon may never answer) is not, and so does not fit.
    #[test]
    fn only_a_dmabuf_or_a_memory_file_is_asked_its_size() {
        assert!(is_memory_file("/dmabuf:"));
        assert!(is_memory_file("/memfd:buffer (deleted)"));
        assert!(!is_memory_file("/mnt/fuse/file"));
        assert!(!is_memory_file("/tmp/regular"));
        assert!(!is_memory_file("pipe:[1234]"));
        let file = std::fs::File::open("/proc/self/exe").expect("a regular file");
        assert_eq!(memory_file_size(std::os::fd::AsFd::as_fd(&file)), None);
        let memory = sized_file(4096);
        assert_eq!(
            memory_file_size(std::os::fd::AsFd::as_fd(&memory)),
            Some(4096)
        );
    }

    /// A 64 x 64 buffer of `code` with a plane of 256-byte rows in a memory file of the whole
    /// buffer's size (a file standing in for the GPU's: `lseek(SEEK_END)` reads its size).
    fn buffer_of(code: Fourcc, modifier: Modifier) -> Dmabuf {
        let fd = sized_file(256 * 64);
        let mut builder = Dmabuf::builder((64, 64), code, modifier, DmabufFlags::empty());
        assert!(builder.add_plane(fd, 0, 256), "one plane");
        builder.build().expect("a dmabuf with a plane")
    }

    fn sized_file(length: u64) -> OwnedFd {
        let fd = memfd_create("dmabuf-test", MemfdFlags::CLOEXEC).expect("a memfd");
        ftruncate(&fd, length).expect("a size");
        fd
    }

    fn part_with(offer: Option<DmabufOffer>) -> DmabufPart {
        DmabufPart {
            offered: offer,
            ..DmabufPart::new(GlobalFilter::policy())
        }
    }

    #[test]
    fn the_soft_main_device_is_the_first_fabricated_render_node() {
        // glibc: makedev(226, 128) = (226 << 8) | 128.
        assert_eq!(SOFT_MAIN_DEVICE, MainDevice(0xE280));
        assert_eq!(MainDevice::of(0x1234, 0x5678), MainDevice(0x1000_0562_3478));
    }

    fn two_gpu_offer() -> DmabufOffer {
        use casement_core::gpu::{
            DrmNodeId, FormatEntry, FormatTable, Fourcc as CoreFourcc, Modifier as CoreModifier,
            RenderPlan, feedback_for,
        };
        use std::collections::BTreeMap;
        let (render, other) = (DrmNodeId::new(226, 128), DrmNodeId::new(226, 129));
        let entry = |code: Fourcc, modifier: u64| FormatEntry {
            format: CoreFourcc(code as u32),
            modifier: CoreModifier(modifier),
        };
        let plan = RenderPlan::new(
            render,
            BTreeMap::from([(
                casement_core::OutputName::parse("DP-1").expect("name"),
                other,
            )]),
        );
        let table = FormatTable::new(BTreeMap::from([
            (render, vec![entry(Fourcc::Argb8888, 0)]),
            (
                other,
                vec![entry(Fourcc::Argb8888, 0), entry(Fourcc::Xrgb8888, 0)],
            ),
        ]));
        DmabufOffer::from_feedback(&feedback_for(&plan, &table))
    }

    #[test]
    fn an_offer_from_a_plan_keeps_the_render_tranche_first() {
        let offer = two_gpu_offer();
        assert_eq!(offer.main_device, MainDevice::of(226, 128));
        let devices: Vec<_> = offer
            .tranches
            .iter()
            .map(|tranche| tranche.device)
            .collect();
        assert_eq!(
            devices,
            [MainDevice::of(226, 128), MainDevice::of(226, 129)]
        );
        let main_formats = [Format {
            code: Fourcc::Argb8888,
            modifier: Modifier::Linear,
        }];
        assert_eq!(offer.formats, main_formats, "the main device's own formats");
    }

    #[test]
    fn a_scanout_tranche_in_the_plan_is_a_scanout_tranche_in_the_offer() {
        use casement_core::gpu::{
            DrmNodeId, FormatEntry, FormatTable, Fourcc as CoreFourcc, Modifier as CoreModifier,
            RenderPlan, feedback_for,
        };
        use std::collections::BTreeMap;
        let render = DrmNodeId::new(226, 128);
        let entry = FormatEntry {
            format: CoreFourcc(Fourcc::Xrgb8888 as u32),
            modifier: CoreModifier(0),
        };
        let plan = RenderPlan::new(
            render,
            BTreeMap::from([(
                casement_core::OutputName::parse("eDP-1").expect("name"),
                render,
            )]),
        );
        let table = FormatTable::new(BTreeMap::from([(render, vec![entry])]))
            .with_scanout(BTreeMap::from([(render, vec![entry])]));
        let offer = DmabufOffer::from_feedback(&feedback_for(&plan, &table));
        let purposes: Vec<_> = offer.tranches.iter().map(|t| t.purpose).collect();
        assert_eq!(purposes, [OfferUse::Scanout], "the pair is offered once");
        assert_eq!(
            offer.formats.len(),
            1,
            "the main device's formats include it"
        );
    }

    #[test]
    fn a_buffer_is_tagged_with_the_device_of_the_tranche_that_offers_its_format() {
        let part = part_with(Some(two_gpu_offer()));
        let on = |code| {
            part.source_device(Format {
                code,
                modifier: Modifier::Linear,
            })
        };
        assert_eq!(on(Fourcc::Argb8888), Some(MainDevice::of(226, 128)));
        assert_eq!(on(Fourcc::Xrgb8888), Some(MainDevice::of(226, 129)));
        assert_eq!(
            on(Fourcc::Abgr8888),
            Some(MainDevice::of(226, 128)),
            "a format nobody offers falls back to the main device"
        );
        assert_eq!(
            part_with(None).source_device(Format {
                code: Fourcc::Argb8888,
                modifier: Modifier::Linear,
            }),
            None
        );
    }

    #[test]
    fn imports_are_accepted_from_any_tranche() {
        let mut part = part_with(Some(two_gpu_offer()));
        for code in [Fourcc::Argb8888, Fourcc::Xrgb8888] {
            let mut buffer = buffer_of(code, Modifier::Linear);
            assert_eq!(
                on_import(&mut part, &mut buffer),
                Import::Accepted,
                "{code:?}"
            );
        }
    }

    #[test]
    fn nothing_offered_refuses_every_import() {
        let mut part = part_with(None);
        let mut buffer = buffer_of(Fourcc::Argb8888, Modifier::Linear);
        assert_eq!(on_import(&mut part, &mut buffer), Import::Refused);
        assert_eq!(part.main_device(), None);
    }

    #[test]
    fn the_soft_offer_accepts_its_linear_formats_only() {
        let mut part = part_with(Some(DmabufOffer::soft()));
        assert_eq!(part.main_device(), Some(SOFT_MAIN_DEVICE));
        for (code, modifier, want) in [
            (Fourcc::Argb8888, Modifier::Linear, Import::Accepted),
            (Fourcc::Xrgb8888, Modifier::Linear, Import::Accepted),
            (Fourcc::Abgr8888, Modifier::Linear, Import::Refused),
            (Fourcc::Argb8888, Modifier::Invalid, Import::Refused),
        ] {
            let mut buffer = buffer_of(code, modifier);
            assert_eq!(
                on_import(&mut part, &mut buffer),
                want,
                "{code:?} {modifier:?}"
            );
        }
    }

    /// CASW-17: a plane that reaches past its file, or a side past the texture limit, is
    /// refused; a plane that fits exactly is not.
    #[test]
    fn a_buffer_larger_than_its_file_or_the_limit_is_refused() {
        let mut part = part_with(Some(DmabufOffer::soft()));
        let build = |side: (i32, i32), length: u64, stride: u32| {
            let mut builder = Dmabuf::builder(
                side,
                Fourcc::Argb8888,
                Modifier::Linear,
                DmabufFlags::empty(),
            );
            assert!(
                builder.add_plane(sized_file(length), 0, stride),
                "one plane"
            );
            builder.build().expect("a dmabuf with a plane")
        };
        let cases = [
            (
                "exactly the file",
                build((64, 64), 256 * 64, 256),
                Import::Accepted,
            ),
            (
                "one row too many",
                build((64, 65), 256 * 64, 256),
                Import::Refused,
            ),
            (
                "a stride past the file",
                build((64, 64), 256 * 64, u32::MAX),
                Import::Refused,
            ),
            (
                "a side past the limit",
                build(
                    (i32::try_from(MAX_DMABUF_SIDE).expect("small") + 1, 1),
                    1 << 20,
                    1 << 20,
                ),
                Import::Refused,
            ),
        ];
        for (name, mut buffer, want) in cases {
            assert_eq!(on_import(&mut part, &mut buffer), want, "{name}");
        }
    }

    #[test]
    fn plane_arithmetic_cannot_overflow_into_a_fit() {
        let plane = |offset, stride| Plane { offset, stride };
        let cases = [
            ("fits", vec![plane(0, 100)], vec![Some(1000)], 10, true),
            (
                "one byte short",
                vec![plane(0, 100)],
                vec![Some(999)],
                10,
                false,
            ),
            (
                "the offset counts",
                vec![plane(1, 100)],
                vec![Some(1000)],
                10,
                false,
            ),
            ("unknown size", vec![plane(0, 100)], vec![None], 10, false),
            (
                "a chroma plane has half the rows",
                vec![plane(0, 100), plane(0, 100)],
                vec![Some(1000), Some(500)],
                10,
                true,
            ),
            (
                "wide arithmetic",
                vec![plane(u32::MAX, u32::MAX)],
                vec![Some(u64::from(u32::MAX))],
                1 << 20,
                false,
            ),
            (
                "planes and files agree",
                vec![plane(0, 1)],
                vec![],
                1,
                false,
            ),
        ];
        for (name, planes, sizes, height, want) in cases {
            assert_eq!(planes_fit(&planes, &sizes, height), want, "{name}");
        }
    }
}
