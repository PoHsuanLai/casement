//! `zwp_linux_dmabuf_v1`: GPU buffers. No global until the backend offers a main device
//! (`--dmabuf soft` in the jail, a render node from P1); `--dmabuf none` advertises nothing.
//! Every imported buffer is tagged with the node of the feedback tranche it came from (A5):
//! the tag is a smithay `DrmNode`, a device type this crate may not name (B6), so the
//! composition root's handler applies it from the [`MainDevice`] this module reports.

use crate::states::{GlobalFilter, WaylandFault};
use casement_core::policy::GlobalKind;
use smithay::backend::allocator::dmabuf::Dmabuf;
use smithay::backend::allocator::{Buffer, Format, Fourcc, Modifier};
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

    /// The device client buffers are tagged with: the offered main device.
    pub fn main_device(&self) -> Option<MainDevice> {
        self.offered.as_ref().map(|offer| offer.main_device)
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

/// What the backend offers clients: its main device and the formats it imports. (P1's
/// `core::gpu::FeedbackPlan` with tranches replaces it.)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DmabufOffer {
    /// The main device.
    pub main_device: MainDevice,
    /// The formats and modifiers it imports.
    pub formats: Vec<Format>,
}

impl DmabufOffer {
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
        }
    }
}

/// Whether an imported buffer was accepted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Import {
    /// The renderer can sample it.
    Accepted,
    /// It cannot.
    Refused,
}

/// A client imported `dmabuf`: accepted when something is advertised and the buffer's format
/// and modifier are among the offered ones (the renderer's own check joins at P1-D4). The
/// handler tags an accepted buffer with [`DmabufPart::main_device`]'s node (A5).
pub fn on_import(part: &mut DmabufPart, dmabuf: &mut Dmabuf) -> Import {
    let format = dmabuf.format();
    let offered = part
        .offered
        .as_ref()
        .is_some_and(|offer| offer.formats.contains(&format));
    if offered {
        Import::Accepted
    } else {
        Import::Refused
    }
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
    let feedback = match DmabufFeedbackBuilder::new(
        offer.main_device.0,
        offer.formats.iter().copied(),
    )
    .build()
    {
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
    use std::fs::File;
    use std::os::fd::OwnedFd;

    fn buffer_of(code: Fourcc, modifier: Modifier) -> Dmabuf {
        let fd = OwnedFd::from(File::open("/dev/null").expect("/dev/null opens"));
        let mut builder = Dmabuf::builder((64, 64), code, modifier, DmabufFlags::empty());
        assert!(builder.add_plane(fd, 0, 256), "one plane");
        builder.build().expect("a dmabuf with a plane")
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
}
