//! `zwp_linux_dmabuf_v1` (P0-H6).

use crate::state::State;
use smithay::backend::allocator::dmabuf::Dmabuf;
use smithay::wayland::dmabuf::{DmabufGlobal, DmabufHandler, DmabufState, ImportNotifier};

impl DmabufHandler for State {
    fn dmabuf_state(&mut self) -> &mut DmabufState {
        &mut self.wl.dmabuf.state
    }

    fn dmabuf_imported(&mut self, global: &DmabufGlobal, dmabuf: Dmabuf, notifier: ImportNotifier) {
        let _ = (global, dmabuf, notifier);
        todo!("P0-H6: DmabufHandler::dmabuf_imported")
    }
}
