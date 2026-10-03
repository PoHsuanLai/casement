//! `xdg_foreign` (P0-H2).

use casement_core::policy::ConnectionRole;
use casement_testkit::{Fixture, FixtureSpec};

/// A dialog of client B parented to client A's exported toplevel gets that parent in the model.
#[test]
#[ignore = "P0-H2"]
fn an_imported_parent_becomes_the_parent() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let client = fixture.connect(ConnectionRole::Public);
    let _ = fixture.client(client);
    todo!(
        "P0-H2: a dialog of client B parented to client A's exported toplevel gets that parent in the model"
    )
}
