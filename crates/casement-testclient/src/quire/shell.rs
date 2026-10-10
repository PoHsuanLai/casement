//! `quire_shell_v1` as a client (P2-Q2): roles, minimise targets and the events the compositor
//! sends the shell.

use crate::conn::TestClient;
use crate::surface::toplevel::{Host, Shared};
use casement_protocols::client::quire_shell_v1::quire_shell_v1::{self, QuireShellV1};
use wayland_client::protocol::wl_surface::WlSurface;
use wayland_client::{Connection, Dispatch, QueueHandle, WEnum};

pub use casement_protocols::client::quire_shell_v1::quire_shell_v1::{
    Error as ShellError, MacMods, ShellRole,
};

/// What a shell object was told.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ShellEvents {
    /// Every `modifiers`, in order: the surface it was addressed to and the key set.
    pub modifiers: Vec<(WlSurface, u32)>,
}

/// A shell object's log.
pub type ShellLog = Shared<ShellEvents>;

/// A bound `quire_shell_v1`.
#[derive(Debug)]
pub struct ShellHandle {
    /// The global.
    pub object: QuireShellV1,
    /// What it was told.
    pub log: ShellLog,
}

impl ShellHandle {
    /// Binds the global at `version` (1 is the only one).
    pub fn bind(client: &mut TestClient, version: u32) -> ShellHandle {
        let log = ShellLog::default();
        let object = client
            .bind(version..=version, log.clone())
            .expect("the compositor advertises quire_shell_v1 to this client");
        ShellHandle { object, log }
    }

    /// Gives `surface` the role `role`.
    pub fn set_role(&self, surface: &WlSurface, role: ShellRole) {
        self.object.set_role(surface, role);
    }

    /// Sets where the window `toplevel` minimises to: a rectangle local to `dock`.
    pub fn set_minimise_target(
        &self,
        dock: &WlSurface,
        toplevel: &str,
        at: (i32, i32),
        size: (u32, u32),
    ) {
        self.object
            .set_minimise_target(dock, toplevel.to_owned(), at.0, at.1, size.0, size.1);
    }

    /// Forgets the minimise target of `toplevel`.
    pub fn clear_minimise_target(&self, toplevel: &str) {
        self.object.clear_minimise_target(toplevel.to_owned());
    }

    /// The key sets sent to `surface`, in order.
    pub fn modifiers_of(&self, surface: &WlSurface) -> Vec<u32> {
        self.log.with(|seen| {
            seen.modifiers
                .iter()
                .filter(|(to, _)| to == surface)
                .map(|(_, mods)| *mods)
                .collect()
        })
    }
}

impl Dispatch<QuireShellV1, ShellLog> for Host {
    fn event(
        _: &mut Self,
        _: &QuireShellV1,
        event: quire_shell_v1::Event,
        log: &ShellLog,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let quire_shell_v1::Event::Modifiers { surface, mods } = event {
            let bits = match mods {
                WEnum::Value(mods) => mods.bits(),
                WEnum::Unknown(raw) => raw,
            };
            log.with(|seen| seen.modifiers.push((surface, bits)));
        }
    }
}
