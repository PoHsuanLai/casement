//! `zwp_linux_dmabuf_v1`: GPU buffers. No global until the backend offers a main device
//! (`--dmabuf soft` in the jail, a render node from P1); `--dmabuf none` advertises nothing.
//! Every imported buffer is tagged with the node of the feedback tranche it came from (A5).

use crate::states::{GlobalFilter, WaylandFault};
use casement_core::policy::GlobalKind;
use smithay::backend::allocator::Format;
use smithay::backend::allocator::dmabuf::Dmabuf;
use smithay::reexports::wayland_protocols::wp::linux_dmabuf::zv1::server::zwp_linux_dmabuf_v1::ZwpLinuxDmabufV1;
use smithay::reexports::wayland_server::{DisplayHandle, GlobalDispatch};
use smithay::wayland::buffer::BufferHandler;
use smithay::wayland::dmabuf::{DmabufGlobal, DmabufGlobalData, DmabufHandler, DmabufState};

/// `zwp_linux_dmabuf_v1`, once offered.
#[derive(Debug)]
pub struct DmabufPart {
    /// smithay's state.
    pub state: DmabufState,
    /// The global, once [`apply_offer`] created it.
    pub global: Option<DmabufGlobal>,
    /// The filter the global is created with.
    pub filter: GlobalFilter,
}

impl DmabufPart {
    /// The part without a global.
    pub fn new(role_filter: GlobalFilter) -> DmabufPart {
        DmabufPart {
            state: DmabufState::new(),
            global: None,
            filter: role_filter,
        }
    }
}

/// What the backend offers clients: its main device and the formats it imports. (P1's
/// `core::gpu::FeedbackPlan` with tranches replaces it.)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DmabufOffer {
    /// The main device's `dev_t`.
    pub main_device: u64,
    /// The formats and modifiers it imports.
    pub formats: Vec<Format>,
}

/// Whether an imported buffer was accepted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Import {
    /// The renderer can sample it.
    Accepted,
    /// It cannot.
    Refused,
}

/// A client imported `dmabuf` (tagging it with its tranche's node, A5).
pub fn on_import(part: &mut DmabufPart, dmabuf: &mut Dmabuf) -> Import {
    let _ = (part, dmabuf);
    todo!("P0-H6: dmabuf::on_import")
}

/// Creates (or replaces) the global with default feedback for `offer`, filtered by the I7
/// table (`GlobalKind::Dmabuf`).
pub fn apply_offer<D>(
    part: &mut DmabufPart,
    display: &DisplayHandle,
    offer: &DmabufOffer,
) -> Result<(), WaylandFault>
where
    D: GlobalDispatch<ZwpLinuxDmabufV1, DmabufGlobalData> + BufferHandler + DmabufHandler + 'static,
{
    let _ = (part, display, offer, GlobalKind::Dmabuf);
    todo!("P0-H6: dmabuf::apply_offer")
}
