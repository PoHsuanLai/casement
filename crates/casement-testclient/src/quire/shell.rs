//! `quire_shell_v1` as a client (P2-Q2): roles, minimise targets and the events the compositor
//! sends the shell.

use crate::conn::TestClient;
use crate::surface::toplevel::{Host, Shared};
use casement_protocols::client::quire_shell_v1::quire_shell_v1::{self, QuireShellV1};
use wayland_client::protocol::wl_surface::WlSurface;
use wayland_client::{Connection, Dispatch, QueueHandle, WEnum};

pub use casement_protocols::client::quire_shell_v1::quire_shell_v1::{
    Error as ShellError, MacMods, ShellRole, SplitSide, WindowEntry,
};

/// What a shell object was told.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ShellEvents {
    /// Every `modifiers`, in order: the surface it was addressed to and the key set.
    pub modifiers: Vec<(WlSurface, u32)>,
    /// Every `window_menu`, in order.
    pub window_menus: Vec<WindowMenuSeen>,
    /// Every `split_choose`, in order, with the candidates that followed it.
    pub split_choices: Vec<SplitChoiceSeen>,
    /// The first window of every `split_choice_end`, in order.
    pub split_ends: Vec<String>,
}

/// One `split_choose` the shell was told, and the `split_candidate`s after it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SplitChoiceSeen {
    /// The first window's identifier.
    pub first: String,
    /// The half it took (`None` for a code this client does not know).
    pub side: Option<SplitSide>,
    /// The candidates, in the order sent.
    pub candidates: Vec<String>,
}

/// One `window_menu` the shell was told.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowMenuSeen {
    /// The window's identifier.
    pub toplevel: String,
    /// Where the menu opens, in global logical pixels.
    pub at: (i32, i32),
    /// The bit per entry code that can be picked.
    pub enabled: u32,
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

    /// Picks `entry` in the menu of the window `toplevel`.
    pub fn window_menu_pick(&self, toplevel: &str, entry: WindowEntry) {
        self.object.window_menu_pick(toplevel.to_owned(), entry);
    }

    /// Picks `toplevel` for the other half of the Split View being chosen.
    pub fn split_pick(&self, toplevel: &str) {
        self.object.split_pick(toplevel.to_owned());
    }

    /// Turns the choice of a Split View down.
    pub fn split_cancel(&self) {
        self.object.split_cancel();
    }

    /// Moves the divider of the Split View `toplevel` is in: the leading window takes `share`
    /// thousandths of the width.
    pub fn split_divide(&self, toplevel: &str, share: u32) {
        self.object.split_divide(toplevel.to_owned(), share);
    }

    /// The Split View offers the compositor made, in order.
    pub fn split_choices(&self) -> Vec<SplitChoiceSeen> {
        self.log.with(|seen| seen.split_choices.clone())
    }

    /// The first windows of the offers the compositor took away, in order.
    pub fn split_ends(&self) -> Vec<String> {
        self.log.with(|seen| seen.split_ends.clone())
    }

    /// The window menus the compositor asked for, in order.
    pub fn window_menus(&self) -> Vec<WindowMenuSeen> {
        self.log.with(|seen| seen.window_menus.clone())
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
        match event {
            quire_shell_v1::Event::Modifiers { surface, mods } => {
                let bits = match mods {
                    WEnum::Value(mods) => mods.bits(),
                    WEnum::Unknown(raw) => raw,
                };
                log.with(|seen| seen.modifiers.push((surface, bits)));
            }
            quire_shell_v1::Event::WindowMenu {
                toplevel,
                x,
                y,
                enabled,
            } => log.with(|seen| {
                seen.window_menus.push(WindowMenuSeen {
                    toplevel,
                    at: (x, y),
                    enabled,
                });
            }),
            quire_shell_v1::Event::SplitChoose { first, side } => log.with(|seen| {
                seen.split_choices.push(SplitChoiceSeen {
                    first,
                    side: match side {
                        WEnum::Value(side) => Some(side),
                        WEnum::Unknown(_) => None,
                    },
                    candidates: Vec::new(),
                });
            }),
            quire_shell_v1::Event::SplitCandidate { toplevel } => log.with(|seen| {
                if let Some(choice) = seen.split_choices.last_mut() {
                    choice.candidates.push(toplevel);
                }
            }),
            quire_shell_v1::Event::SplitChoiceEnd { first } => {
                log.with(|seen| seen.split_ends.push(first));
            }
            _ => {}
        }
    }
}
