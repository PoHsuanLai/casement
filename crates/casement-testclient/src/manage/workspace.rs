//! `ext_workspace_manager_v1` as a client (P1-M2): the workspace groups and workspaces the
//! compositor announced, with their ids, names, coordinates, states and capabilities, and the
//! requests a switcher makes (which the compositor takes at `commit`).
//!
//! Self-contained like `foreign_toplevel_list`: it reads its own events on a side queue of the
//! client's connection, and binds every `wl_output` first so groups can name them.

use crate::misc::idle::{pump_queue, side_queue};
use crate::{ClientError, GlobalList, Pumped};
use wayland_client::protocol::wl_output::{self, WlOutput};
use wayland_client::{Connection, Dispatch, EventQueue, QueueHandle, WEnum, event_created_child};
use wayland_protocols::ext::workspace::v1::client::{
    ext_workspace_group_handle_v1::{self, ExtWorkspaceGroupHandleV1},
    ext_workspace_handle_v1::{self, ExtWorkspaceHandleV1, State},
    ext_workspace_manager_v1::{self, ExtWorkspaceManagerV1},
};

/// Whether a workspace is the one its group shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Shown {
    /// Not active.
    #[default]
    No,
    /// The `active` state bit is set.
    Yes,
}

/// One workspace as announced.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WorkspaceSeen {
    /// The `id` event (none when the compositor sent none).
    pub id: Option<String>,
    /// The `name` event.
    pub name: String,
    /// The `coordinates` event, as unsigned words.
    pub coordinates: Vec<u32>,
    /// The `state` event's active bit.
    pub shown: Shown,
    /// The `capabilities` event's bits (1 activate, 2 deactivate, 4 remove, 8 assign).
    pub capabilities: u32,
    /// The index (in [`WorkspaceWatch::groups`]) of the group it is in.
    pub group: Option<usize>,
}

/// One workspace group as announced.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GroupSeen {
    /// The `capabilities` event's bits (1 create_workspace).
    pub capabilities: u32,
    /// How many `wl_output`s it has.
    pub outputs: usize,
}

#[derive(Debug)]
struct Space {
    handle: ExtWorkspaceHandleV1,
    seen: WorkspaceSeen,
    group: Option<ExtWorkspaceGroupHandleV1>,
}

#[derive(Debug)]
struct Group {
    handle: ExtWorkspaceGroupHandleV1,
    seen: GroupSeen,
    on: Vec<WlOutput>,
}

/// What the manager announced.
#[derive(Debug, Default)]
struct Roster {
    groups: Vec<Group>,
    spaces: Vec<Space>,
    dones: usize,
    finished: bool,
}

/// A bound `ext_workspace_manager_v1`.
#[derive(Debug)]
pub struct WorkspaceWatch {
    connection: Connection,
    queue: EventQueue<Roster>,
    roster: Roster,
    manager: ExtWorkspaceManagerV1,
    _outputs: Vec<WlOutput>,
}

impl WorkspaceWatch {
    /// Binds the first `wl_output` and the manager; the compositor announces its workspaces at
    /// the fixture's next pump.
    pub fn start(globals: &GlobalList) -> Result<WorkspaceWatch, ClientError> {
        let (connection, queue) = side_queue::<Roster>(globals)?;
        let qh = queue.handle();
        let missing = |name: &str| ClientError::MissingGlobal(name.to_owned());
        let output = globals
            .bind::<WlOutput, _, _>(&qh, 1..=4, ())
            .map_err(|_| missing("wl_output"))?;
        let manager = globals
            .bind::<ExtWorkspaceManagerV1, _, _>(&qh, 1..=1, ())
            .map_err(|_| missing("ext_workspace_manager_v1"))?;
        connection
            .flush()
            .map_err(|error| ClientError::Dispatch(error.to_string()))?;
        Ok(WorkspaceWatch {
            connection,
            queue,
            roster: Roster::default(),
            manager,
            _outputs: vec![output],
        })
    }

    /// Reads what arrived.
    pub fn pump(&mut self) -> Result<Pumped, ClientError> {
        pump_queue(&self.connection, &mut self.queue, &mut self.roster)
    }

    /// Every group announced and not removed, in order.
    pub fn groups(&self) -> Vec<GroupSeen> {
        self.roster
            .groups
            .iter()
            .map(|group| GroupSeen {
                outputs: group.on.len(),
                ..group.seen.clone()
            })
            .collect()
    }

    /// Every workspace announced and not removed, in order.
    pub fn workspaces(&self) -> Vec<WorkspaceSeen> {
        self.roster
            .spaces
            .iter()
            .map(|space| WorkspaceSeen {
                group: space.group.as_ref().and_then(|group| {
                    self.roster
                        .groups
                        .iter()
                        .position(|held| &held.handle == group)
                }),
                ..space.seen.clone()
            })
            .collect()
    }

    /// How many `done` events arrived.
    pub fn dones(&self) -> usize {
        self.roster.dones
    }

    /// Whether the compositor sent `finished`.
    pub fn finished(&self) -> bool {
        self.roster.finished
    }

    /// Asks to activate workspace `index` (takes effect at [`commit`](Self::commit)).
    pub fn activate(&mut self, index: usize) -> Result<(), ClientError> {
        self.space(index)?.handle.activate();
        self.flush()
    }

    /// Asks to deactivate workspace `index`.
    pub fn deactivate(&mut self, index: usize) -> Result<(), ClientError> {
        self.space(index)?.handle.deactivate();
        self.flush()
    }

    /// Asks to remove workspace `index`.
    pub fn remove(&mut self, index: usize) -> Result<(), ClientError> {
        self.space(index)?.handle.remove();
        self.flush()
    }

    /// Asks to assign workspace `index` to group `group`.
    pub fn assign(&mut self, index: usize, group: usize) -> Result<(), ClientError> {
        let group = self
            .roster
            .groups
            .get(group)
            .ok_or_else(|| ClientError::Dispatch(format!("no group {group}")))?;
        self.space(index)?.handle.assign(&group.handle);
        self.flush()
    }

    /// Asks group `group` for a new workspace called `name`.
    pub fn create_workspace(&mut self, group: usize, name: &str) -> Result<(), ClientError> {
        let group = self
            .roster
            .groups
            .get(group)
            .ok_or_else(|| ClientError::Dispatch(format!("no group {group}")))?;
        group.handle.create_workspace(name.to_owned());
        self.flush()
    }

    /// `commit`: the requests made since the last commit take effect together.
    pub fn commit(&mut self) -> Result<(), ClientError> {
        self.manager.commit();
        self.flush()
    }

    /// Asks the compositor to stop (`stop`); it answers `finished`.
    pub fn stop(&mut self) -> Result<(), ClientError> {
        self.manager.stop();
        self.flush()
    }

    fn space(&self, index: usize) -> Result<&Space, ClientError> {
        self.roster
            .spaces
            .get(index)
            .ok_or_else(|| ClientError::Dispatch(format!("no workspace {index}")))
    }

    fn flush(&self) -> Result<(), ClientError> {
        self.connection
            .flush()
            .map_err(|error| ClientError::Dispatch(error.to_string()))
    }
}

impl Dispatch<ExtWorkspaceManagerV1, ()> for Roster {
    fn event(
        state: &mut Roster,
        _proxy: &ExtWorkspaceManagerV1,
        event: ext_workspace_manager_v1::Event,
        _data: &(),
        _connection: &Connection,
        _qh: &QueueHandle<Roster>,
    ) {
        match event {
            ext_workspace_manager_v1::Event::WorkspaceGroup { workspace_group } => {
                state.groups.push(Group {
                    handle: workspace_group,
                    seen: GroupSeen::default(),
                    on: Vec::new(),
                });
            }
            ext_workspace_manager_v1::Event::Workspace { workspace } => {
                state.spaces.push(Space {
                    handle: workspace,
                    seen: WorkspaceSeen::default(),
                    group: None,
                });
            }
            ext_workspace_manager_v1::Event::Done => state.dones += 1,
            ext_workspace_manager_v1::Event::Finished => state.finished = true,
            _ => {}
        }
    }

    event_created_child!(Roster, ExtWorkspaceManagerV1, [
        ext_workspace_manager_v1::EVT_WORKSPACE_GROUP_OPCODE => (ExtWorkspaceGroupHandleV1, ()),
        ext_workspace_manager_v1::EVT_WORKSPACE_OPCODE => (ExtWorkspaceHandleV1, ()),
    ]);
}

impl Dispatch<ExtWorkspaceGroupHandleV1, ()> for Roster {
    fn event(
        state: &mut Roster,
        proxy: &ExtWorkspaceGroupHandleV1,
        event: ext_workspace_group_handle_v1::Event,
        _data: &(),
        _connection: &Connection,
        _qh: &QueueHandle<Roster>,
    ) {
        match event {
            ext_workspace_group_handle_v1::Event::WorkspaceEnter { workspace } => {
                if let Some(space) = state.spaces.iter_mut().find(|s| s.handle == workspace) {
                    space.group = Some(proxy.clone());
                }
            }
            ext_workspace_group_handle_v1::Event::WorkspaceLeave { workspace } => {
                if let Some(space) = state.spaces.iter_mut().find(|s| s.handle == workspace) {
                    space.group = None;
                }
            }
            ext_workspace_group_handle_v1::Event::Removed => {
                state.groups.retain(|group| &group.handle != proxy);
            }
            ext_workspace_group_handle_v1::Event::Capabilities { capabilities } => {
                if let Some(group) = state.groups.iter_mut().find(|g| &g.handle == proxy) {
                    group.seen.capabilities = match capabilities {
                        WEnum::Value(bits) => bits.bits(),
                        WEnum::Unknown(raw) => raw,
                    };
                }
            }
            ext_workspace_group_handle_v1::Event::OutputEnter { output } => {
                if let Some(group) = state.groups.iter_mut().find(|g| &g.handle == proxy)
                    && !group.on.contains(&output)
                {
                    group.on.push(output);
                }
            }
            ext_workspace_group_handle_v1::Event::OutputLeave { output } => {
                if let Some(group) = state.groups.iter_mut().find(|g| &g.handle == proxy) {
                    group.on.retain(|held| held != &output);
                }
            }
            _ => {}
        }
    }
}

impl Dispatch<ExtWorkspaceHandleV1, ()> for Roster {
    fn event(
        state: &mut Roster,
        proxy: &ExtWorkspaceHandleV1,
        event: ext_workspace_handle_v1::Event,
        _data: &(),
        _connection: &Connection,
        _qh: &QueueHandle<Roster>,
    ) {
        if let ext_workspace_handle_v1::Event::Removed = event {
            state.spaces.retain(|space| &space.handle != proxy);
            return;
        }
        let Some(space) = state.spaces.iter_mut().find(|s| &s.handle == proxy) else {
            return;
        };
        match event {
            ext_workspace_handle_v1::Event::Id { id } => space.seen.id = Some(id),
            ext_workspace_handle_v1::Event::Name { name } => space.seen.name = name,
            ext_workspace_handle_v1::Event::Coordinates { coordinates } => {
                space.seen.coordinates = coordinates
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .map(|word| u32::from_ne_bytes(*word))
                    .collect();
            }
            ext_workspace_handle_v1::Event::State { state: bits } => {
                let active = match bits {
                    WEnum::Value(bits) => bits.contains(State::Active),
                    WEnum::Unknown(raw) => raw & State::Active.bits() != 0,
                };
                space.seen.shown = match active {
                    true => Shown::Yes,
                    false => Shown::No,
                };
            }
            ext_workspace_handle_v1::Event::Capabilities { capabilities } => {
                space.seen.capabilities = match capabilities {
                    WEnum::Value(bits) => bits.bits(),
                    WEnum::Unknown(raw) => raw,
                };
            }
            _ => {}
        }
    }
}

impl Dispatch<WlOutput, ()> for Roster {
    fn event(
        _state: &mut Roster,
        _proxy: &WlOutput,
        _event: wl_output::Event,
        _data: &(),
        _connection: &Connection,
        _qh: &QueueHandle<Roster>,
    ) {
    }
}
