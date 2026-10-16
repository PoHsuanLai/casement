//! ext-session-lock: lock, lock surfaces, unlock. Filled by P0-H3.
//!
//! The wire events reach the transcript through the connection's tap; the handles remember
//! `locked` and `finished`, and the latest configure of a lock surface.

use crate::conn::{ClientError, ClientState, TestClient};
use crate::shm::ShmBuffer;
use std::sync::{Arc, Mutex, PoisonError};
use wayland_client::protocol::wl_compositor::WlCompositor;
use wayland_client::protocol::wl_output::{self, WlOutput};
use wayland_client::protocol::wl_surface::WlSurface;
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle};
use wayland_protocols::ext::session_lock::v1::client::ext_session_lock_manager_v1::{
    self, ExtSessionLockManagerV1,
};
use wayland_protocols::ext::session_lock::v1::client::ext_session_lock_surface_v1::{
    self, ExtSessionLockSurfaceV1,
};
use wayland_protocols::ext::session_lock::v1::client::ext_session_lock_v1::{
    self, ExtSessionLockV1,
};

/// The user data of the objects here that carry no state.
#[derive(Debug, Clone, Copy, Default)]
pub struct LockPlain;

/// Whether the compositor has said `locked` or `finished`.
#[derive(Debug, Clone, Default)]
pub struct LockLog(Arc<Mutex<LockState>>);

#[derive(Debug, Default, Clone, Copy)]
struct LockState {
    locked: bool,
    finished: bool,
}

/// The latest configure of a lock surface: serial, width, height.
#[derive(Debug, Clone, Default)]
pub struct LockSurfaceLog(Arc<Mutex<Option<(u32, u32, u32)>>>);

/// A session lock the client asked for.
#[derive(Debug)]
pub struct Locker {
    lock: ExtSessionLockV1,
    log: LockLog,
}

/// A lock surface covering one output.
#[derive(Debug)]
pub struct LockSurface {
    surface: WlSurface,
    shell: ExtSessionLockSurfaceV1,
    log: LockSurfaceLog,
}

impl Locker {
    /// Asks to lock the session.
    pub fn request(client: &TestClient) -> Result<Locker, ClientError> {
        let manager: ExtSessionLockManagerV1 = client.bind(1..=1, LockPlain)?;
        let log = LockLog::default();
        let lock = manager.lock(&client.qh(), log.clone());
        let _ = client.connection().flush();
        Ok(Locker { lock, log })
    }

    /// Whether the compositor said `locked`.
    pub fn is_locked(&self) -> bool {
        self.log
            .0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .locked
    }

    /// Whether the compositor said `finished` (the lock was refused or is over).
    pub fn is_finished(&self) -> bool {
        self.log
            .0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .finished
    }

    /// Creates a surface covering the first output the compositor advertises.
    pub fn cover(&self, client: &TestClient) -> Result<LockSurface, ClientError> {
        self.cover_on(client, 0)
    }

    /// Creates a surface covering the `nth` output the compositor advertises (0 is the first).
    pub fn cover_on(&self, client: &TestClient, nth: usize) -> Result<LockSurface, ClientError> {
        let qh = client.qh();
        let compositor: WlCompositor = client.bind(1..=5, LockPlain)?;
        let global = client
            .globals()
            .contents()
            .clone_list()
            .into_iter()
            .filter(|global| global.interface == "wl_output")
            .nth(nth)
            .ok_or_else(|| ClientError::MissingGlobal("wl_output".to_owned()))?;
        let output: WlOutput =
            client
                .globals()
                .registry()
                .bind(global.name, global.version.min(4), &qh, LockPlain);
        let surface = compositor.create_surface(&qh, LockPlain);
        let log = LockSurfaceLog::default();
        let shell = self
            .lock
            .get_lock_surface(&surface, &output, &qh, log.clone());
        let _ = client.connection().flush();
        Ok(LockSurface {
            surface,
            shell,
            log,
        })
    }

    /// Unlocks and destroys the lock.
    pub fn unlock(self, client: &TestClient) {
        self.lock.unlock_and_destroy();
        let _ = client.connection().flush();
    }
}

impl LockSurface {
    /// The latest configure: serial, width, height.
    pub fn configure(&self) -> Option<(u32, u32, u32)> {
        *self.log.0.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Acknowledges the latest configure; false when there is none yet.
    pub fn ack(&self, client: &TestClient) -> bool {
        let Some((serial, _, _)) = self.configure() else {
            return false;
        };
        self.shell.ack_configure(serial);
        let _ = client.connection().flush();
        true
    }

    /// Removes the buffer (`attach(null)`) and commits: the surface shows nothing.
    pub fn unmap(&self, client: &TestClient) {
        self.surface.attach(None, 0, 0);
        self.surface.commit();
        let _ = client.connection().flush();
    }

    /// Destroys the lock surface and its `wl_surface`.
    pub fn destroy(self, client: &TestClient) {
        self.shell.destroy();
        self.surface.destroy();
        let _ = client.connection().flush();
    }

    /// Attaches `buffer` over the whole surface and commits.
    pub fn show(&self, client: &TestClient, buffer: &ShmBuffer) {
        let size = buffer.size();
        self.surface.attach(Some(buffer.buffer()), 0, 0);
        self.surface.damage_buffer(
            0,
            0,
            i32::try_from(size.w).unwrap_or(i32::MAX),
            i32::try_from(size.h).unwrap_or(i32::MAX),
        );
        self.surface.commit();
        let _ = client.connection().flush();
    }
}

macro_rules! plain {
    ($($object:ty => $event:ty),* $(,)?) => {$(
        impl Dispatch<$object, LockPlain> for ClientState {
            fn event(
                _: &mut Self,
                _: &$object,
                _: $event,
                _: &LockPlain,
                _: &Connection,
                _: &QueueHandle<Self>,
            ) {
            }
        }
    )*};
}

plain! {
    WlCompositor => <WlCompositor as Proxy>::Event,
    WlSurface => <WlSurface as Proxy>::Event,
    WlOutput => wl_output::Event,
    ExtSessionLockManagerV1 => ext_session_lock_manager_v1::Event,
}

impl Dispatch<ExtSessionLockV1, LockLog> for ClientState {
    fn event(
        _: &mut Self,
        _: &ExtSessionLockV1,
        event: ext_session_lock_v1::Event,
        log: &LockLog,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        let mut state = log.0.lock().unwrap_or_else(PoisonError::into_inner);
        match event {
            ext_session_lock_v1::Event::Locked => state.locked = true,
            ext_session_lock_v1::Event::Finished => state.finished = true,
            _ => {}
        }
    }
}

impl Dispatch<ExtSessionLockSurfaceV1, LockSurfaceLog> for ClientState {
    fn event(
        _: &mut Self,
        _: &ExtSessionLockSurfaceV1,
        event: ext_session_lock_surface_v1::Event,
        log: &LockSurfaceLog,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let ext_session_lock_surface_v1::Event::Configure {
            serial,
            width,
            height,
        } = event
        {
            *log.0.lock().unwrap_or_else(PoisonError::into_inner) = Some((serial, width, height));
        }
    }
}
