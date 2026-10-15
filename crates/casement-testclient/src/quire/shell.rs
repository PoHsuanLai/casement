//! `quire_shell_v1` as a client (P2-Q2): roles, minimise targets and the events the compositor
//! sends the shell.

use crate::conn::TestClient;
use crate::surface::toplevel::{Host, Shared};
use casement_protocols::client::quire_shell_v1::quire_shell_v1::{self, QuireShellV1};
use wayland_client::protocol::wl_output::{self, WlOutput};
use wayland_client::protocol::wl_surface::WlSurface;
use wayland_client::{Connection, Dispatch, QueueHandle, WEnum};

pub use casement_protocols::client::quire_desktop_v1::quire_desktop_window_v1::Toggle;
pub use casement_protocols::client::quire_shell_v1::quire_shell_v1::{
    DragOutcome, Error as ShellError, MacMods, OverviewMode, OverviewState, ShellRole, Shown,
    SplitSide, WindowEntry,
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
    /// Every `overview_state`, in order: the state (`None` for a code this client does not
    /// know) and the progress in thousandths.
    pub overview_states: Vec<(Option<OverviewState>, u32)>,
    /// Every `overview_output` (version 2), in order: the output and whether its Mission Control
    /// is up (`None` for a code this client does not know).
    pub overview_outputs: Vec<(WlOutput, Option<Shown>)>,
    /// Every `drop_target` (version 2), in order: the output and the Space under the dragged
    /// window, or none.
    pub drop_targets: Vec<(WlOutput, Option<String>)>,
    /// Every `drop_target_new` (version 3), in order: the output and whether the dragged window
    /// is over its new-Space tile (`None` for a code this client does not know).
    pub drop_targets_new: Vec<(WlOutput, Option<Shown>)>,
    /// Every `window_drag_begin` (version 4), in order: the output and the window's identifier.
    pub drag_begins: Vec<(WlOutput, String)>,
    /// Every `window_drag_end` (version 4), in order: the output, the window's identifier and the
    /// outcome (`None` for a code this client does not know).
    pub drag_ends: Vec<(WlOutput, String, Option<DragOutcome>)>,
}

/// A bound `wl_output` and the connector name it reported.
#[derive(Debug, Clone)]
pub struct BoundOutput {
    /// The object.
    pub object: WlOutput,
    /// Its connector name, once it has arrived.
    pub name: Shared<Option<String>>,
}

impl BoundOutput {
    /// The connector name, if it has arrived.
    pub fn connector(&self) -> Option<String> {
        self.name.with(|name| name.clone())
    }
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
    /// Binds the global at `version` (1, 2, 3 or 4).
    pub fn bind(client: &mut TestClient, version: u32) -> ShellHandle {
        let log = ShellLog::default();
        let object = client
            .bind(version..=version, log.clone())
            .expect("the compositor advertises quire_shell_v1 to this client");
        ShellHandle { object, log }
    }

    /// Binds every `wl_output` (the events that name an output reach this client only for the
    /// ones it has bound). The names arrive at the fixture's next pump.
    pub fn bind_outputs(client: &TestClient) -> Vec<BoundOutput> {
        let globals = client.globals();
        let qh = client.qh();
        let outputs: Vec<BoundOutput> = globals
            .contents()
            .clone_list()
            .into_iter()
            .filter(|global| global.interface == "wl_output")
            .map(|global| {
                let name = Shared::<Option<String>>::default();
                let object = globals.registry().bind::<WlOutput, _, _>(
                    global.name,
                    global.version.min(4),
                    &qh,
                    name.clone(),
                );
                BoundOutput { object, name }
            })
            .collect();
        let _ = client.connection().flush();
        outputs
    }

    /// Forgets the Space targets of `output` (version 2).
    pub fn space_target_clear(&self, output: &WlOutput) {
        self.object.space_target_clear(output);
    }

    /// Sets a Space target: dropping a dragged window on the rectangle at `at` of `size` (global
    /// logical pixels) moves it to the Space `space` (version 2).
    pub fn space_target(&self, output: &WlOutput, space: &str, at: (i32, i32), size: (u32, u32)) {
        self.object
            .space_target(output, space.to_owned(), at.0, at.1, size.0, size.1);
    }

    /// Sets the new-Space target of `output`: dropping a dragged window on the rectangle at `at`
    /// of `size` (global logical pixels) makes a Space with it; an empty rectangle removes the
    /// target (version 3).
    pub fn space_target_new(&self, output: &WlOutput, at: (i32, i32), size: (u32, u32)) {
        self.object
            .space_target_new(output, at.0, at.1, size.0, size.1);
    }

    /// The `window_drag_begin` events, in order.
    pub fn drag_begins(&self) -> Vec<(WlOutput, String)> {
        self.log.with(|seen| seen.drag_begins.clone())
    }

    /// The `window_drag_end` events, in order.
    pub fn drag_ends(&self) -> Vec<(WlOutput, String, Option<DragOutcome>)> {
        self.log.with(|seen| seen.drag_ends.clone())
    }

    /// The `drop_target_new` events, in order.
    pub fn drop_targets_new(&self) -> Vec<(WlOutput, Option<Shown>)> {
        self.log.with(|seen| seen.drop_targets_new.clone())
    }

    /// The `overview_output` events, in order.
    pub fn overview_outputs(&self) -> Vec<(WlOutput, Option<Shown>)> {
        self.log.with(|seen| seen.overview_outputs.clone())
    }

    /// The `drop_target` events, in order.
    pub fn drop_targets(&self) -> Vec<(WlOutput, Option<String>)> {
        self.log.with(|seen| seen.drop_targets.clone())
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

    /// Opens (on) or closes (off) the overview `mode`.
    pub fn overview(&self, mode: OverviewMode, toggle: Toggle) {
        self.object.overview(mode, toggle);
    }

    /// The overview states the compositor sent, in order, with their progress.
    pub fn overview_states(&self) -> Vec<(Option<OverviewState>, u32)> {
        self.log.with(|seen| seen.overview_states.clone())
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
            quire_shell_v1::Event::OverviewState {
                state, progress, ..
            } => log.with(|seen| {
                let state = match state {
                    WEnum::Value(state) => Some(state),
                    WEnum::Unknown(_) => None,
                };
                seen.overview_states.push((state, progress));
            }),
            quire_shell_v1::Event::OverviewOutput { output, shown } => log.with(|seen| {
                let shown = match shown {
                    WEnum::Value(shown) => Some(shown),
                    WEnum::Unknown(_) => None,
                };
                seen.overview_outputs.push((output, shown));
            }),
            quire_shell_v1::Event::DropTarget { output, space_id } => {
                log.with(|seen| seen.drop_targets.push((output, space_id)));
            }
            quire_shell_v1::Event::DropTargetNew { output, over } => log.with(|seen| {
                let over = match over {
                    WEnum::Value(over) => Some(over),
                    WEnum::Unknown(_) => None,
                };
                seen.drop_targets_new.push((output, over));
            }),
            quire_shell_v1::Event::WindowDragBegin { output, toplevel } => {
                log.with(|seen| seen.drag_begins.push((output, toplevel)));
            }
            quire_shell_v1::Event::WindowDragEnd {
                output,
                toplevel,
                outcome,
            } => log.with(|seen| {
                let outcome = match outcome {
                    WEnum::Value(outcome) => Some(outcome),
                    WEnum::Unknown(_) => None,
                };
                seen.drag_ends.push((output, toplevel, outcome));
            }),
            _ => {}
        }
    }
}

impl Dispatch<WlOutput, Shared<Option<String>>> for Host {
    fn event(
        _: &mut Self,
        _: &WlOutput,
        event: wl_output::Event,
        name: &Shared<Option<String>>,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let wl_output::Event::Name { name: connector } = event {
            name.with(|held| *held = Some(connector));
        }
    }
}
