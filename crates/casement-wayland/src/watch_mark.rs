//! A mark on a surface saying a protocol module already holds it in its watch list, so the
//! module need not search the list to know (CASW-20): `commit_timing` and `fifo` keep the
//! surfaces whose barriers wait for a frame, and a search per commit or per destroyed surface
//! made a client with many surfaces cost the square of their number.
//!
//! The kind `K` says whose mark it is, so one surface can carry a mark of each module.

use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::wayland::compositor::with_states;
use std::marker::PhantomData;
use std::sync::atomic::{AtomicBool, Ordering};

/// The mark of kind `K` in a surface's user data.
struct Mark<K> {
    set: AtomicBool,
    kind: PhantomData<fn() -> K>,
}

impl<K> Default for Mark<K> {
    fn default() -> Mark<K> {
        Mark {
            set: AtomicBool::new(false),
            kind: PhantomData,
        }
    }
}

/// Marks `surface` with `K`; whether it was marked already.
pub(crate) fn set<K: 'static>(surface: &WlSurface) -> bool {
    with_states(surface, |states| {
        let mark = states.data_map.get_or_insert_threadsafe(Mark::<K>::default);
        mark.set.swap(true, Ordering::AcqRel)
    })
}

/// Whether `surface` carries the mark `K`.
pub(crate) fn is_set<K: 'static>(surface: &WlSurface) -> bool {
    with_states(surface, |states| {
        states
            .data_map
            .get::<Mark<K>>()
            .is_some_and(|mark| mark.set.load(Ordering::Acquire))
    })
}

/// Takes the mark `K` off `surface`.
pub(crate) fn clear<K: 'static>(surface: &WlSurface) {
    with_states(surface, |states| {
        if let Some(mark) = states.data_map.get::<Mark<K>>() {
            mark.set.store(false, Ordering::Release);
        }
    });
}
