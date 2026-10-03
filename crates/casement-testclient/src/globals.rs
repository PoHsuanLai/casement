//! The registry's globals.
//!
//! `wayland_client::globals::registry_queue_init` blocks until the compositor answers, which a
//! fixture serving the compositor on the same thread can never do. So the list here fills as the
//! registry's events are dispatched (the fixture pumps until quiet), and is otherwise the same
//! shape: [`GlobalList::contents`], [`GlobalList::bind`], [`GlobalList::registry`].

use crate::conn::ClientState;
use std::ops::RangeInclusive;
use std::sync::{Arc, Mutex};
use wayland_client::globals::BindError;
use wayland_client::protocol::wl_registry::{self, WlRegistry};
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle};

pub use wayland_client::globals::Global;

/// The globals the registry has advertised so far (the registry object's user data).
#[derive(Debug, Clone, Default)]
pub struct GlobalListContents(Arc<Mutex<Vec<Global>>>);

impl GlobalListContents {
    /// Runs `f` on the current list.
    pub fn with_list<T, F: FnOnce(&[Global]) -> T>(&self, f: F) -> T {
        f(&self.lock())
    }

    /// A copy of the current list, in advertisement order.
    pub fn clone_list(&self) -> Vec<Global> {
        self.lock().clone()
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Vec<Global>> {
        // A poisoned list is still a list: the writer only pushes and removes.
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

/// The registry and what it advertised.
#[derive(Debug)]
pub struct GlobalList {
    registry: WlRegistry,
    contents: GlobalListContents,
}

impl GlobalList {
    pub(crate) fn new(registry: WlRegistry, contents: GlobalListContents) -> GlobalList {
        GlobalList { registry, contents }
    }

    /// What the registry has advertised so far.
    pub fn contents(&self) -> &GlobalListContents {
        &self.contents
    }

    /// The `wl_registry` itself.
    pub fn registry(&self) -> &WlRegistry {
        &self.registry
    }

    /// Binds the global of interface `I`, at the lower of the advertised version and
    /// `versions.end()`; an error when it is not advertised or only below `versions.start()`.
    pub fn bind<I, State, U>(
        &self,
        qh: &QueueHandle<State>,
        versions: RangeInclusive<u32>,
        udata: U,
    ) -> Result<I, BindError>
    where
        I: Proxy + 'static,
        State: Dispatch<I, U> + 'static,
        U: Send + Sync + 'static,
    {
        let interface = I::interface();
        let (name, advertised) = self
            .contents
            .with_list(|list| {
                list.iter()
                    .find(|global| global.interface == interface.name)
                    .map(|global| (global.name, global.version))
            })
            .ok_or(BindError::NotPresent)?;
        if advertised < *versions.start() {
            return Err(BindError::UnsupportedVersion);
        }
        let version = advertised.min(*versions.end()).min(interface.version);
        Ok(self.registry.bind(name, version, qh, udata))
    }
}

impl Dispatch<WlRegistry, GlobalListContents> for ClientState {
    fn event(
        _: &mut Self,
        _: &WlRegistry,
        event: wl_registry::Event,
        contents: &GlobalListContents,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        let mut list = contents.lock();
        match event {
            wl_registry::Event::Global {
                name,
                interface,
                version,
            } => list.push(Global {
                name,
                interface,
                version,
            }),
            wl_registry::Event::GlobalRemove { name } => list.retain(|global| global.name != name),
            _ => {}
        }
    }
}
