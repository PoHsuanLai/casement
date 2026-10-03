//! `xdg_dialog` (P0-H2).

use casement_core::policy::ConnectionRole;
use casement_testkit::{Fixture, FixtureSpec};

/// Set_modal reaches the model as Modality::Modal on the dialog's window.
#[test]
#[ignore = "P0-H2"]
fn a_modal_dialog_is_stored_as_modal() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let client = fixture.connect(ConnectionRole::Public);
    let _ = fixture.client(client);
    todo!("P0-H2: set_modal reaches the model as Modality::Modal on the dialog's window")
}
