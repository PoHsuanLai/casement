//! `workspace` (P1-M2): Spaces as ext-workspace. Ignored until the lane advertises
//! `ext_workspace_manager_v1`.

use casement_core::policy::ConnectionRole;
use casement_testkit::fakes::MemorySpaceStore;
use casement_testkit::{Fixture, FixtureSpec};

/// The workspace `id` is the minted `SpaceId`.
#[test]
#[ignore = "P1-M2"]
fn the_workspace_id_is_the_minted_space_id() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let _client = fixture.connect(ConnectionRole::Public);
    todo!("P1-M2: bind, read the group and workspace, compare with model().spaces()");
}

/// The id is the same after a restart: a second compositor started on the same store finds the
/// stored Spaces (`spaces.toml`).
#[test]
#[ignore = "P1-M2"]
fn the_workspace_id_survives_a_restart() {
    let _store = MemorySpaceStore::empty();
    todo!("P1-M2: start, note the id, start again on the same store, compare");
}

/// activate, create_workspace and remove reach the model as `Event::Workspace`.
#[test]
#[ignore = "P1-M2"]
fn requests_reach_the_model() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let _client = fixture.connect(ConnectionRole::Public);
    todo!("P1-M2: activate, create, remove");
}
