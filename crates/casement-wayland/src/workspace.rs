//! `ext_workspace_manager_v1`: the Spaces as workspaces, one group per output, with ids that
//! are casement's minted `SpaceId`s and so stable across a restart (C16). Its requests become
//! `Event::Workspace`.
//!
//! smithay has no module for it, so this one carries its own `GlobalDispatch2` and `Dispatch2`
//! impls (as `quire_window` does). A client's requests wait until its `commit` and then go to
//! the model together; the model's answer is published by `apply_spaces`, which brings every
//! bound manager from what it was told to the [`Layout`] of the Spaces now (a pure value, so
//! what is published is tested without a wire).
//!
//! Seeing and managing the Spaces is a power (`Power::Windows`, `gate`): a manager bound while
//! the person has not answered hears nothing until they allow, and is `finished` if they
//! refuse.

mod gate;

pub use gate::{WaitingManager, on_resolved};

use crate::consent::{self, Admission, ConsentHandler, Held};
use crate::states::{GlobalFilter, WaylandFault};
use casement_core::policy::GlobalKind;
use casement_core::space::{SpaceBook, WorkspaceRequest};
use casement_core::{Event, OutputKey};
use casement_protocols::vocab::{Power, SpaceKind};
use porter_core::SpaceId;
use smithay::output::Output;
use smithay::reexports::wayland_protocols::ext::workspace::v1::server::{
    ext_workspace_group_handle_v1::{self, ExtWorkspaceGroupHandleV1, GroupCapabilities},
    ext_workspace_handle_v1::{self, ExtWorkspaceHandleV1, State, WorkspaceCapabilities},
    ext_workspace_manager_v1::{self, ExtWorkspaceManagerV1},
};
use smithay::reexports::wayland_server::backend::{ClientId, GlobalId};
use smithay::reexports::wayland_server::protocol::wl_output::WlOutput;
use smithay::reexports::wayland_server::{
    Client, DataInit, Dispatch, DisplayHandle, GlobalDispatch, New, Resource,
};
use smithay::wayland::{Dispatch2, GlobalDispatch2};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex, PoisonError};

/// The version casement serves.
pub const VERSION: u32 = 1;

/// What a client asked since its last `commit`.
type Pending = Arc<Mutex<Vec<WorkspaceRequest>>>;

/// `ext_workspace_manager_v1`, its groups and the workspaces published on it.
#[derive(Debug, Default)]
pub struct WorkspacePart {
    /// The outputs with a workspace group.
    pub groups: BTreeSet<OutputKey>,
    /// The Spaces published as workspaces.
    pub published: BTreeSet<SpaceId>,
    global: Option<GlobalId>,
    bound: Vec<Bound>,
    layout: Layout,
    outputs: BTreeMap<OutputKey, Output>,
    spawn: Option<Spawn>,
    /// The managers bound while the person had not answered, inert until they do.
    waiting: Held<WaitingManager>,
}

/// How to make handles where the state type is known ([`WorkspacePart::new`]).
#[derive(Debug, Clone)]
struct Spawn {
    display: DisplayHandle,
    group:
        fn(&DisplayHandle, &ExtWorkspaceManagerV1, GroupData) -> Option<ExtWorkspaceGroupHandleV1>,
    workspace:
        fn(&DisplayHandle, &ExtWorkspaceManagerV1, WorkspaceData) -> Option<ExtWorkspaceHandleV1>,
}

/// One bound manager and what it was told.
#[derive(Debug)]
struct Bound {
    manager: ExtWorkspaceManagerV1,
    pending: Pending,
    groups: BTreeMap<OutputKey, GroupHold>,
    workspaces: BTreeMap<SpaceId, SpaceHold>,
}

/// A group handle and the `wl_output`s it said the group has.
#[derive(Debug)]
struct GroupHold {
    handle: ExtWorkspaceGroupHandleV1,
    entered: Vec<WlOutput>,
}

/// A workspace handle, what it was told and the group it is in.
#[derive(Debug)]
struct SpaceHold {
    handle: ExtWorkspaceHandleV1,
    told: Placed,
    group: Option<OutputKey>,
}

impl WorkspacePart {
    /// Creates the global (version 1), filtered by the I7 table.
    pub fn new<D>(display: &DisplayHandle, role_filter: GlobalFilter) -> WorkspacePart
    where
        D: GlobalDispatch<ExtWorkspaceManagerV1, WorkspaceGlobal>
            + Dispatch<ExtWorkspaceGroupHandleV1, GroupData>
            + Dispatch<ExtWorkspaceHandleV1, WorkspaceData>
            + 'static,
    {
        WorkspacePart {
            global: Some(display.create_global::<D, ExtWorkspaceManagerV1, _>(
                VERSION,
                WorkspaceGlobal {
                    filter: role_filter,
                },
            )),
            spawn: Some(Spawn {
                display: display.clone(),
                group: |display, manager, data| {
                    let client = manager.client()?;
                    let handle = client
                        .create_resource::<ExtWorkspaceGroupHandleV1, GroupData, D>(
                            display,
                            manager.version(),
                            data,
                        )
                        .ok()?;
                    manager.workspace_group(&handle);
                    Some(handle)
                },
                workspace: |display, manager, data| {
                    let client = manager.client()?;
                    let handle = client
                        .create_resource::<ExtWorkspaceHandleV1, WorkspaceData, D>(
                            display,
                            manager.version(),
                            data,
                        )
                        .ok()?;
                    manager.workspace(&handle);
                    Some(handle)
                },
            }),
            ..WorkspacePart::default()
        }
    }

    /// The global, when it was created.
    pub fn global(&self) -> Option<&GlobalId> {
        self.global.as_ref()
    }
}

/// What the composition root provides to this protocol's dispatch.
pub trait WorkspaceHandler: ConsentHandler {
    /// The part.
    fn workspace_part(&mut self) -> &mut WorkspacePart;
}

/// Adds `manager` to the bound ones and tells it every group and workspace as they are.
fn attach(part: &mut WorkspacePart, manager: ExtWorkspaceManagerV1, pending: Pending) {
    let mut bound = Bound {
        manager,
        pending,
        groups: BTreeMap::new(),
        workspaces: BTreeMap::new(),
    };
    if let Some(spawn) = &part.spawn {
        sync(&mut bound, &part.layout, &part.outputs, spawn);
    }
    part.bound.push(bound);
}

/// The global's data: the filter its `can_view` asks.
#[derive(Debug, Clone, Copy)]
pub struct WorkspaceGlobal {
    filter: GlobalFilter,
}

/// A bound manager's data.
#[derive(Debug, Clone)]
pub struct ManagerData {
    pending: Pending,
}

/// A group handle's data: the output it groups.
#[derive(Debug, Clone)]
pub struct GroupData {
    output: OutputKey,
    pending: Pending,
}

/// A workspace handle's data: the Space it stands for.
#[derive(Debug, Clone)]
pub struct WorkspaceData {
    space: SpaceId,
    pending: Pending,
}

/// Queues `request` until the client's next `commit`.
fn queue(pending: &Pending, request: WorkspaceRequest) {
    pending
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .push(request);
}

impl<D> GlobalDispatch2<ExtWorkspaceManagerV1, D> for WorkspaceGlobal
where
    D: WorkspaceHandler + Dispatch<ExtWorkspaceManagerV1, ManagerData> + 'static,
{
    fn bind(
        &self,
        state: &mut D,
        _handle: &DisplayHandle,
        client: &Client,
        resource: New<ExtWorkspaceManagerV1>,
        data_init: &mut DataInit<'_, D>,
    ) {
        let pending = Pending::default();
        let manager = data_init.init(
            resource,
            ManagerData {
                pending: pending.clone(),
            },
        );
        match consent::admit(state, client, Power::Windows) {
            Admission::Go => attach(state.workspace_part(), manager, pending),
            Admission::Hold => gate::wait(state, client, WaitingManager::new(manager, pending)),
            Admission::Refuse => consent::finish_later(consent::Farewell::Workspaces(manager)),
        }
    }

    fn can_view(&self, client: &Client) -> bool {
        self.filter.sees(GlobalKind::Workspace, client)
    }
}

impl<D> Dispatch2<ExtWorkspaceManagerV1, D> for ManagerData
where
    D: WorkspaceHandler,
{
    fn request(
        &self,
        state: &mut D,
        client: &Client,
        resource: &ExtWorkspaceManagerV1,
        request: ext_workspace_manager_v1::Request,
        _dhandle: &DisplayHandle,
        _data_init: &mut DataInit<'_, D>,
    ) {
        match request {
            ext_workspace_manager_v1::Request::Commit => {
                let asked: Vec<WorkspaceRequest> = std::mem::take(
                    &mut *self.pending.lock().unwrap_or_else(PoisonError::into_inner),
                );
                // A manager acts only while its client may manage the windows: a power taken
                // back, or not given yet, makes a commit change nothing.
                if consent::allowed(state, client, Power::Windows) != Admission::Go {
                    return;
                }
                let part = state.workspace_part();
                let events: Vec<Event> = asked
                    .into_iter()
                    .flat_map(|request| on_request(part, request))
                    .collect();
                state.feed(events);
            }
            ext_workspace_manager_v1::Request::Stop => {
                state
                    .workspace_part()
                    .bound
                    .retain(|bound| bound.manager != *resource);
                resource.finished();
            }
            _ => {}
        }
    }

    fn destroyed(&self, state: &mut D, _client: ClientId, resource: &ExtWorkspaceManagerV1) {
        state
            .workspace_part()
            .bound
            .retain(|bound| bound.manager != *resource);
    }
}

impl<D> Dispatch2<ExtWorkspaceGroupHandleV1, D> for GroupData
where
    D: WorkspaceHandler,
{
    fn request(
        &self,
        _state: &mut D,
        _client: &Client,
        _resource: &ExtWorkspaceGroupHandleV1,
        request: ext_workspace_group_handle_v1::Request,
        _dhandle: &DisplayHandle,
        _data_init: &mut DataInit<'_, D>,
    ) {
        if let ext_workspace_group_handle_v1::Request::CreateWorkspace { workspace } = request {
            queue(
                &self.pending,
                WorkspaceRequest::Create {
                    output: self.output.clone(),
                    name: workspace,
                },
            );
        }
    }
}

impl<D> Dispatch2<ExtWorkspaceHandleV1, D> for WorkspaceData
where
    D: WorkspaceHandler,
{
    fn request(
        &self,
        _state: &mut D,
        _client: &Client,
        _resource: &ExtWorkspaceHandleV1,
        request: ext_workspace_handle_v1::Request,
        _dhandle: &DisplayHandle,
        _data_init: &mut DataInit<'_, D>,
    ) {
        let space = self.space.clone();
        let asked = match request {
            ext_workspace_handle_v1::Request::Activate => Some(WorkspaceRequest::Activate(space)),
            ext_workspace_handle_v1::Request::Deactivate => {
                Some(WorkspaceRequest::Deactivate(space))
            }
            ext_workspace_handle_v1::Request::Remove => Some(WorkspaceRequest::Remove(space)),
            ext_workspace_handle_v1::Request::Assign { workspace_group } => workspace_group
                .data::<GroupData>()
                .map(|group| WorkspaceRequest::Assign {
                    space,
                    output: group.output.clone(),
                }),
            _ => None,
        };
        if let Some(asked) = asked {
            queue(&self.pending, asked);
        }
    }
}

/// A request on a workspace or group.
///
/// ```no_run
/// use casement_core::space::WorkspaceRequest;
/// use casement_wayland::workspace::{WorkspacePart, on_request};
///
/// # fn demo(id: porter_core::SpaceId) {
/// let mut part = WorkspacePart::default();
/// let events = on_request(&mut part, WorkspaceRequest::Activate(id));
/// assert_eq!(events.len(), 1);
/// # }
/// ```
pub fn on_request(part: &mut WorkspacePart, request: WorkspaceRequest) -> Vec<Event> {
    let _ = part;
    vec![Event::Workspace(request)]
}

/// One Space as a workspace client sees it.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Placed {
    /// The output (and so the group) it is in.
    pub output: OutputKey,
    /// The human-readable name.
    pub name: String,
    /// Its place in the group's row, from 0 (the workspace's one coordinate).
    pub position: u32,
    /// What it holds.
    pub kind: SpaceKind,
    /// Whether its output shows it.
    pub activity: Activity,
}

/// Whether a Space is the one its output shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Activity {
    /// Shown.
    Active,
    /// Not shown.
    Inactive,
}

/// The Spaces as they are published: a group per output with Spaces, each Space once.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct Layout {
    /// Each output's Spaces, in row order.
    pub groups: BTreeMap<OutputKey, Vec<SpaceId>>,
    /// Each Space, by id (the workspace's `id` event).
    pub spaces: BTreeMap<SpaceId, Placed>,
}

impl Layout {
    /// The layout of `book`.
    pub fn of(book: &SpaceBook) -> Layout {
        let mut layout = Layout::default();
        for space in book.iter() {
            let active = book
                .shown_on(&space.output)
                .is_some_and(|shown| shown.id == space.id);
            let kind = space.content.kind();
            let name = space.plain_name();
            let activity = match active {
                true => Activity::Active,
                false => Activity::Inactive,
            };
            layout.spaces.insert(
                space.id.clone(),
                Placed {
                    output: space.output.clone(),
                    name,
                    position: space.position.0,
                    kind,
                    activity,
                },
            );
        }
        for output in book.iter().map(|space| space.output.clone()) {
            layout.groups.entry(output.clone()).or_insert_with(|| {
                book.row(&output)
                    .iter()
                    .map(|space| space.id.clone())
                    .collect()
            });
        }
        layout
    }
}

/// The workspace `state` event of a Space.
fn state_of(placed: &Placed) -> State {
    match placed.activity {
        Activity::Active => State::Active,
        Activity::Inactive => State::empty(),
    }
}

/// The requests the compositor takes on a Space: a desktop can be shown, hidden, removed and
/// moved to another output; the others can only be shown.
fn capabilities_of(placed: &Placed) -> WorkspaceCapabilities {
    match placed.kind {
        SpaceKind::Desktop => {
            WorkspaceCapabilities::Activate
                | WorkspaceCapabilities::Deactivate
                | WorkspaceCapabilities::Remove
                | WorkspaceCapabilities::Assign
        }
        _ => WorkspaceCapabilities::Activate,
    }
}

/// Brings `bound` from what it was told to `layout`, then `done` when anything was sent.
fn sync(bound: &mut Bound, layout: &Layout, outputs: &BTreeMap<OutputKey, Output>, spawn: &Spawn) {
    let mut sent = 0usize;
    // New groups.
    for output in layout.groups.keys() {
        if bound.groups.contains_key(output) {
            continue;
        }
        let data = GroupData {
            output: output.clone(),
            pending: bound.pending.clone(),
        };
        if let Some(handle) = (spawn.group)(&spawn.display, &bound.manager, data) {
            handle.capabilities(GroupCapabilities::CreateWorkspace);
            bound.groups.insert(
                output.clone(),
                GroupHold {
                    handle,
                    entered: Vec::new(),
                },
            );
            sent += 1;
        }
    }
    // Spaces that are gone leave their group and are removed.
    let gone: Vec<SpaceId> = bound
        .workspaces
        .keys()
        .filter(|id| !layout.spaces.contains_key(*id))
        .cloned()
        .collect();
    for id in gone {
        if let Some(hold) = bound.workspaces.remove(&id) {
            leave(bound, &hold);
            hold.handle.removed();
            sent += 1;
        }
    }
    // New Spaces, changed Spaces, and each in the group of its output.
    for (id, placed) in &layout.spaces {
        if !bound.workspaces.contains_key(id) {
            let data = WorkspaceData {
                space: id.clone(),
                pending: bound.pending.clone(),
            };
            if let Some(handle) = (spawn.workspace)(&spawn.display, &bound.manager, data) {
                handle.id(id.as_str().to_owned());
                handle.name(placed.name.clone());
                handle.coordinates(placed.position.to_ne_bytes().to_vec());
                handle.state(state_of(placed));
                handle.capabilities(capabilities_of(placed));
                bound.workspaces.insert(
                    id.clone(),
                    SpaceHold {
                        handle,
                        told: placed.clone(),
                        group: None,
                    },
                );
                sent += 1;
            }
        }
        sent += update(bound, id, placed);
    }
    // Groups that are gone, once nothing is left in them.
    let empty: Vec<OutputKey> = bound
        .groups
        .keys()
        .filter(|output| !layout.groups.contains_key(*output))
        .cloned()
        .collect();
    for output in empty {
        if let Some(hold) = bound.groups.remove(&output) {
            hold.handle.removed();
            sent += 1;
        }
    }
    // Each group says which `wl_output`s the client has for its output.
    for (output, hold) in bound.groups.iter_mut() {
        sent += enter_outputs(&bound.manager, hold, outputs.get(output));
    }
    if sent > 0 {
        bound.manager.done();
    }
}

/// `hold`'s workspace leaves the group it was in.
fn leave(bound: &Bound, hold: &SpaceHold) {
    if let Some(group) = hold
        .group
        .as_ref()
        .and_then(|output| bound.groups.get(output))
    {
        group.handle.workspace_leave(&hold.handle);
    }
}

/// Sends what changed of the Space `id` since the workspace was told, and moves it to the
/// group of its output; says how many things were sent.
fn update(bound: &mut Bound, id: &SpaceId, placed: &Placed) -> usize {
    let Some(hold) = bound.workspaces.get_mut(id) else {
        return 0;
    };
    let mut sent = 0;
    if hold.told.name != placed.name {
        hold.handle.name(placed.name.clone());
        sent += 1;
    }
    if hold.told.position != placed.position {
        hold.handle
            .coordinates(placed.position.to_ne_bytes().to_vec());
        sent += 1;
    }
    if hold.told.activity != placed.activity {
        hold.handle.state(state_of(placed));
        sent += 1;
    }
    if hold.told.kind != placed.kind {
        hold.handle.capabilities(capabilities_of(placed));
        sent += 1;
    }
    hold.told = placed.clone();
    if hold.group.as_ref() == Some(&placed.output) {
        return sent;
    }
    let old = hold.group.replace(placed.output.clone());
    let handle = hold.handle.clone();
    if let Some(group) = old.and_then(|output| bound.groups.get(&output)) {
        group.handle.workspace_leave(&handle);
    }
    if let Some(group) = bound.groups.get(&placed.output) {
        group.handle.workspace_enter(&handle);
    }
    sent + 1
}

/// Tells a group of the `wl_output`s its client has for `output`, the ones it was not told of.
fn enter_outputs(
    manager: &ExtWorkspaceManagerV1,
    hold: &mut GroupHold,
    output: Option<&Output>,
) -> usize {
    let (Some(output), Some(client)) = (output, manager.client()) else {
        return 0;
    };
    let current: Vec<WlOutput> = output.client_outputs(&client).collect();
    let fresh: Vec<WlOutput> = current
        .into_iter()
        .filter(|wl| !hold.entered.contains(wl))
        .collect();
    for wl in &fresh {
        hold.handle.output_enter(wl);
    }
    let count = fresh.len();
    hold.entered.extend(fresh);
    count
}

/// Publishes `spaces` (groups, workspaces with their ids, names, coordinates and states) and
/// sends `done`. `outputs` are the `wl_output` globals of the plugged displays, by key.
pub fn apply_spaces(
    part: &mut WorkspacePart,
    spaces: &SpaceBook,
    outputs: &BTreeMap<OutputKey, Output>,
) -> Result<(), WaylandFault> {
    part.layout = Layout::of(spaces);
    part.outputs = outputs.clone();
    part.groups = part.layout.groups.keys().cloned().collect();
    part.published = part.layout.spaces.keys().cloned().collect();
    let WorkspacePart {
        bound,
        layout,
        outputs,
        spawn,
        ..
    } = part;
    if let Some(spawn) = spawn {
        for each in bound.iter_mut() {
            sync(each, layout, outputs, spawn);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use casement_core::OutputName;
    use casement_core::output::{EdidFacts, EdidSerial};
    use casement_core::space::{Space, SpaceContent, SpaceIndex};

    fn output(name: &str) -> OutputKey {
        let edid = EdidFacts {
            make: String::new(),
            model: String::new(),
            serial: EdidSerial::Unknown,
        };
        OutputKey::of(&edid, &OutputName::parse(name).expect("name"))
    }

    fn space(id: &str, on: &str, position: u32) -> Space {
        Space {
            id: SpaceId::parse(id).expect("id"),
            output: output(on),
            content: SpaceContent::Desktop,
            position: SpaceIndex(position),
        }
    }

    #[test]
    fn the_layout_has_a_group_per_output_and_a_workspace_per_space() {
        let book = SpaceBook::empty()
            .with_space(space("s-b", "DP-1", 1))
            .with_space(space("s-a", "DP-1", 0))
            .with_space(space("s-c", "DP-2", 0))
            .with_shown(output("DP-1"), SpaceId::parse("s-b").expect("id"));
        let layout = Layout::of(&book);
        let ids = |o: &str| -> Vec<&str> {
            layout.groups[&output(o)]
                .iter()
                .map(|id| id.as_str())
                .collect()
        };
        assert_eq!(ids("DP-1"), ["s-a", "s-b"], "in row order");
        assert_eq!(ids("DP-2"), ["s-c"]);
        let at = |id: &str| &layout.spaces[&SpaceId::parse(id).expect("id")];
        assert_eq!(at("s-b").activity, Activity::Active);
        assert_eq!(at("s-a").activity, Activity::Inactive);
        assert_eq!(at("s-c").activity, Activity::Inactive);
        assert_eq!(at("s-a").name, "Desktop 1");
        assert_eq!(at("s-b").position, 1);
    }

    #[test]
    fn the_same_book_is_the_same_layout() {
        let book = SpaceBook::empty().with_space(space("s-a", "DP-1", 0));
        assert_eq!(Layout::of(&book), Layout::of(&book.clone()));
        assert_ne!(Layout::of(&book), Layout::default());
    }

    #[test]
    fn only_a_desktop_can_be_removed_or_moved() {
        let desktop = Layout::of(&SpaceBook::empty().with_space(space("s-a", "DP-1", 0)));
        let placed = &desktop.spaces[&SpaceId::parse("s-a").expect("id")];
        assert!(capabilities_of(placed).contains(WorkspaceCapabilities::Remove));
        let full = Placed {
            kind: SpaceKind::FullScreen,
            ..placed.clone()
        };
        assert_eq!(capabilities_of(&full), WorkspaceCapabilities::Activate);
    }
}
