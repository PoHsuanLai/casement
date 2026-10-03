//! The surface side of the P0-H1 protocols that have no module of their own in the frozen
//! tree: `wl_compositor` and `wl_surface`, `wl_subcompositor` and `wl_subsurface` (this
//! module's own), raw `wl_shm` pools for geometry `shm_buffer` would never make, and the
//! `wl_output`s with their xdg-output. Events are not logged here: the client's transcript has
//! every one, and [`seen`] reads one object's.

use crate::conn::{ClientError, TestClient};
use crate::shm::ShmBuffer;
use rustix::fs::{MemfdFlags, ftruncate, memfd_create};
use std::fs::File;
use std::os::fd::AsFd;
use std::os::unix::fs::FileExt;
use wayland_client::Proxy;
use wayland_client::protocol::wl_buffer::WlBuffer;
use wayland_client::protocol::wl_compositor::WlCompositor;
use wayland_client::protocol::wl_output::WlOutput;
use wayland_client::protocol::wl_region::WlRegion;
use wayland_client::protocol::wl_shm::{Format, WlShm};
use wayland_client::protocol::wl_shm_pool::WlShmPool;
use wayland_client::protocol::wl_subcompositor::WlSubcompositor;
use wayland_client::protocol::wl_subsurface::WlSubsurface;
use wayland_client::protocol::wl_surface::WlSurface;
use wayland_protocols::xdg::xdg_output::zv1::client::{
    zxdg_output_manager_v1::ZxdgOutputManagerV1, zxdg_output_v1::ZxdgOutputV1,
};

/// The user data of every object the P0-H1 helpers make: nothing, because what an object was
/// told is in the transcript.
#[derive(Debug, Clone, Copy, Default)]
pub struct H1Data;

/// `impl Dispatch<object, H1Data> for ClientState` ignoring every event, for each object named.
macro_rules! quiet {
    ($($object:ty),+ $(,)?) => {$(
        impl wayland_client::Dispatch<$object, $crate::surface::subsurface::H1Data>
            for $crate::conn::ClientState
        {
            fn event(
                _: &mut Self,
                _: &$object,
                _: <$object as wayland_client::Proxy>::Event,
                _: &$crate::surface::subsurface::H1Data,
                _: &wayland_client::Connection,
                _: &wayland_client::QueueHandle<Self>,
            ) {
            }
        }
    )+};
}
pub(crate) use quiet;

quiet!(
    WlCompositor,
    WlSurface,
    WlSubcompositor,
    WlSubsurface,
    WlRegion,
    WlShm,
    WlShmPool,
    WlBuffer,
    WlOutput,
    ZxdgOutputManagerV1,
    ZxdgOutputV1
);

/// The events `object` received, in order, as (event, arguments as the transcript prints them).
pub fn seen(client: &TestClient, object: &impl Proxy) -> Vec<(String, Vec<String>)> {
    let name = format!(
        "{}#{}",
        object.id().interface().name,
        object.id().protocol_id()
    );
    client
        .transcript()
        .0
        .iter()
        .filter(|line| line.object == name)
        .map(|line| (line.event.clone(), line.args.clone()))
        .collect()
}

/// A bound `wl_compositor`.
#[derive(Debug)]
pub struct Compositor {
    /// The global.
    pub compositor: WlCompositor,
}

impl Compositor {
    /// Binds `wl_compositor` at exactly `version`.
    pub fn bind(client: &TestClient, version: u32) -> Compositor {
        Compositor {
            compositor: client
                .bind(version..=version, H1Data)
                .expect("the compositor advertises wl_compositor"),
        }
    }

    /// A new surface.
    pub fn surface(&self, client: &TestClient) -> Surface {
        Surface {
            surface: self.compositor.create_surface(&client.qh(), H1Data),
        }
    }
}

/// A `wl_surface`; every request is queued and goes out with the client's next pump.
#[derive(Debug)]
pub struct Surface {
    /// The surface object.
    pub surface: WlSurface,
}

impl Surface {
    /// Attaches `buffer` at the origin.
    pub fn attach(&self, buffer: &ShmBuffer) {
        self.surface.attach(Some(buffer.buffer()), 0, 0);
    }

    /// Attaches a bare `wl_buffer` at the origin.
    pub fn attach_raw(&self, buffer: &WlBuffer) {
        self.surface.attach(Some(buffer), 0, 0);
    }

    /// Damages the whole buffer.
    pub fn damage_all(&self) {
        if self.surface.version() >= 4 {
            self.surface.damage_buffer(0, 0, i32::MAX, i32::MAX);
        } else {
            self.surface.damage(0, 0, i32::MAX, i32::MAX);
        }
    }

    /// Commits.
    pub fn commit(&self) {
        self.surface.commit();
    }

    /// Destroys the surface.
    pub fn destroy(&self) {
        self.surface.destroy();
    }
}

/// A bound `wl_subcompositor`.
#[derive(Debug)]
pub struct Subcompositor {
    /// The global.
    pub subcompositor: WlSubcompositor,
}

impl Subcompositor {
    /// Binds `wl_subcompositor` at exactly `version`.
    pub fn bind(client: &TestClient, version: u32) -> Subcompositor {
        Subcompositor {
            subcompositor: client
                .bind(version..=version, H1Data)
                .expect("the compositor advertises wl_subcompositor"),
        }
    }

    /// Makes `surface` a subsurface of `parent`.
    pub fn subsurface(
        &self,
        client: &TestClient,
        surface: &Surface,
        parent: &Surface,
    ) -> WlSubsurface {
        self.subcompositor
            .get_subsurface(&surface.surface, &parent.surface, &client.qh(), H1Data)
    }
}

/// A `wl_shm_pool` over a memfd of exactly the size asked for, with whatever buffer geometry
/// the test wants (protocol errors included).
#[derive(Debug)]
pub struct RawPool {
    /// The pool object.
    pub pool: WlShmPool,
    file: File,
}

impl RawPool {
    /// A pool of `bytes` bytes on a `wl_shm` bound at `version` (1 or 2).
    pub fn new(client: &TestClient, version: u32, bytes: usize) -> RawPool {
        let shm: WlShm = client
            .bind(version..=version, H1Data)
            .expect("the compositor advertises wl_shm");
        let fd = memfd_create("casement-test-pool", MemfdFlags::CLOEXEC)
            .expect("a memfd is available in the jail");
        ftruncate(&fd, bytes as u64).expect("a memfd can be sized");
        let pool = shm.create_pool(
            fd.as_fd(),
            i32::try_from(bytes).unwrap_or(i32::MAX),
            &client.qh(),
            H1Data,
        );
        RawPool {
            pool,
            file: File::from(fd),
        }
    }

    /// Writes `bytes` at `offset` in the pool.
    pub fn write(&self, offset: u64, bytes: &[u8]) {
        self.file
            .write_all_at(bytes, offset)
            .expect("the pool's memfd is writable");
    }

    /// A buffer on the pool.
    pub fn buffer(
        &self,
        client: &TestClient,
        offset: i32,
        (width, height): (i32, i32),
        stride: i32,
        format: Format,
    ) -> WlBuffer {
        self.pool
            .create_buffer(offset, width, height, stride, format, &client.qh(), H1Data)
    }
}

/// A bound `wl_output` and its xdg-output.
#[derive(Debug)]
pub struct BoundOutput {
    /// The `wl_output`.
    pub output: WlOutput,
    /// Its `zxdg_output_v1`, when asked for.
    pub xdg: Option<ZxdgOutputV1>,
    /// The registry name it was bound under.
    pub name: u32,
}

/// What a bound output was told, read from the transcript.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct OutputFacts {
    /// `name` (version 4).
    pub name: Option<String>,
    /// `mode`: width, height, refresh in mHz.
    pub mode: Option<(i32, i32, i32)>,
    /// `scale`.
    pub scale: Option<i32>,
    /// How many `done` events arrived.
    pub done: usize,
    /// xdg-output `logical_size`.
    pub logical_size: Option<(i32, i32)>,
}

impl BoundOutput {
    /// What this output and its xdg-output were told so far.
    pub fn facts(&self, client: &TestClient) -> OutputFacts {
        let number = |text: &str| text.parse::<i32>().unwrap_or(i32::MIN);
        let mut facts = OutputFacts::default();
        for (event, args) in seen(client, &self.output) {
            match (event.as_str(), args.as_slice()) {
                ("name", [name]) => facts.name = Some(name.trim_matches('"').to_owned()),
                ("mode", [_, w, h, refresh]) => {
                    facts.mode = Some((number(w), number(h), number(refresh)));
                }
                ("scale", [factor]) => facts.scale = Some(number(factor)),
                ("done", _) => facts.done += 1,
                _ => {}
            }
        }
        if let Some(xdg) = &self.xdg {
            for (event, args) in seen(client, xdg) {
                if let ("logical_size", [w, h]) = (event.as_str(), args.as_slice()) {
                    facts.logical_size = Some((number(w), number(h)));
                }
            }
        }
        facts
    }
}

/// Binds every advertised `wl_output` at `version` (one per plugged output), with the
/// xdg-output of each when `with_xdg`.
pub fn bind_outputs(
    client: &TestClient,
    version: u32,
    with_xdg: bool,
) -> Result<Vec<BoundOutput>, ClientError> {
    let names: Vec<u32> = client
        .globals()
        .contents()
        .clone_list()
        .into_iter()
        .filter(|global| global.interface == "wl_output")
        .map(|global| global.name)
        .collect();
    let manager: Option<ZxdgOutputManagerV1> = if with_xdg {
        Some(client.bind(1..=3, H1Data)?)
    } else {
        None
    };
    let registry = client.globals().registry();
    Ok(names
        .into_iter()
        .map(|name| {
            let output: WlOutput = registry.bind(name, version, &client.qh(), H1Data);
            let xdg = manager
                .as_ref()
                .map(|manager| manager.get_xdg_output(&output, &client.qh(), H1Data));
            BoundOutput { output, xdg, name }
        })
        .collect())
}
