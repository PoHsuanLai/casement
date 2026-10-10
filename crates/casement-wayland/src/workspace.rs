//! `ext_workspace_manager_v1`: the Spaces as workspaces, one group per output, with ids that
//! are casement's minted `SpaceId`s and so stable across a restart (C16). P1-Z declares it;
//! P1-M2 creates the global and fills the functions. Its requests become
//! `Event::Workspace`.

use crate::states::{GlobalFilter, WaylandFault};
use casement_core::space::{SpaceBook, WorkspaceRequest};
use casement_core::{Event, OutputKey};
use porter_core::SpaceId;
use smithay::reexports::wayland_server::DisplayHandle;
use std::collections::BTreeSet;

/// `ext_workspace_manager_v1`, its groups and the workspaces published on it.
#[derive(Debug, Default)]
pub struct WorkspacePart {
    /// The outputs with a workspace group.
    pub groups: BTreeSet<OutputKey>,
    /// The Spaces published as workspaces.
    pub published: BTreeSet<SpaceId>,
}

impl WorkspacePart {
    /// Creates the global (version 1), filtered by the I7 table. P1-Z: creates nothing.
    pub fn new<D: 'static>(display: &DisplayHandle, role_filter: GlobalFilter) -> WorkspacePart {
        let _ = (display, role_filter, std::marker::PhantomData::<D>);
        WorkspacePart::default()
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
    let _ = (part, request);
    todo!("P1-M2: workspace::on_request")
}

/// Publishes `spaces` (groups, workspaces with their ids, names, coordinates and states) and
/// sends `done`.
pub fn apply_spaces(part: &mut WorkspacePart, spaces: &SpaceBook) -> Result<(), WaylandFault> {
    let _ = (part, spaces);
    todo!("P1-M2: workspace::apply_spaces")
}
