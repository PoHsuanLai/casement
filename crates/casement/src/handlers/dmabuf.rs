//! `zwp_linux_dmabuf_v1` (P0-H6): accept or refuse a client's import, tagging an accepted
//! buffer with the node it came from (A5).

use crate::state::State;
use casement_wayland::dmabuf::{self, Import, MainDevice};
use smithay::backend::allocator::Buffer;
use smithay::backend::allocator::dmabuf::Dmabuf;
use smithay::reexports::drm::node::DrmNode;
use smithay::wayland::dmabuf::{DmabufGlobal, DmabufHandler, DmabufState, ImportNotifier};

impl DmabufHandler for State {
    fn dmabuf_state(&mut self) -> &mut DmabufState {
        &mut self.wl.dmabuf.state
    }

    fn dmabuf_imported(&mut self, global: &DmabufGlobal, dmabuf: Dmabuf, notifier: ImportNotifier) {
        let mut dmabuf = dmabuf;
        let ours = self.wl.dmabuf.global == Some(*global);
        match (ours, dmabuf::on_import(&mut self.wl.dmabuf, &mut dmabuf)) {
            (true, Import::Accepted) => {
                let source = self.wl.dmabuf.source_device(dmabuf.format());
                tag_with_node(&dmabuf, source);
                if notifier.successful::<State>().is_err() {
                    tracing::debug!("a client left before its dma-buf import was answered");
                }
            }
            (true, Import::Refused) | (false, _) => notifier.failed(),
        }
    }
}

/// Tags `dmabuf` with the render node of `device`, the device of the tranche that offers its
/// format (A5): smithay's multi-GPU renderer copies a
/// buffer only when it knows which GPU holds it, and silently skips the copy otherwise.
/// `DrmNode::from_dev_id` reads no file, so the jail's fabricated minors (A3) type as render
/// nodes. The type arrives through smithay's re-export: this one conversion lives here because
/// the protocol crate may not reach drm (B6).
fn tag_with_node(dmabuf: &Dmabuf, device: Option<MainDevice>) {
    let node = device.and_then(|device| match DrmNode::from_dev_id(device.0) {
        Ok(node) => Some(node),
        Err(error) => {
            tracing::warn!(%error, device = device.0, "main device is not a drm node; dma-buf left untagged");
            None
        }
    });
    dmabuf.set_node(node);
}
