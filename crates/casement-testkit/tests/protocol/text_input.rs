//! `text_input` (P0-H7).

use casement_core::policy::ConnectionRole;
use casement_testkit::{Fixture, FixtureSpec};

/// Every connection sees the text-input manager (I7 shows it to everyone).
#[test]
#[ignore = "P0-H7: written; runs once P0-B-headless (Fixture::start) and P0-T-client (TestClient) land"]
fn text_input_is_advertised_to_every_role() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let public = fixture.connect(ConnectionRole::Public);
    let advertised = fixture
        .client(public)
        .globals()
        .contents()
        .with_list(|list| {
            list.iter().any(|global| {
                global.interface == "zwp_text_input_manager_v3" && global.version == 1
            })
        });
    assert!(advertised, "zwp_text_input_manager_v3 v1 is advertised");
}

/// Enter, enable, then the first key goes to the input method, not the document (SP1 A5).
///
/// Steps: map a toplevel and give it keyboard focus; bind the input method (helper
/// `casement_testclient::ime::input_method`) and the text input (`ime::text_input`) on one
/// connection; the field gets `enter` before it enables; it sends `enable` and `commit`; the
/// method gets `activate` and `done` and grabs the keyboard; inject one key through
/// `fixture.input()` *without* pumping in between the `enable` and the key; after the pump
/// the method's grab saw the key and the document's `wl_keyboard` saw none.
#[test]
#[ignore = "P0-H7: written; runs once P0-B-headless (Fixture::start) and P0-T-client (TestClient) land"]
fn the_first_key_after_enable_reaches_the_input_method() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let client = fixture.connect(ConnectionRole::Public);
    let _ = fixture.client(client);
    todo!(
        "P0-T-client: attach ime::text_input and ime::input_method to the client's queue (delegate_dispatch!), then the steps in this test's doc"
    )
}

/// Preedit then commit_string: the transcript matches tests/golden/transcripts/text_input-commit.txt.
///
/// Steps: as above up to `activate`; the method sends `set_preedit_string("ni", 2, 2)`,
/// `commit(serial)`, then `commit_string("你")`, `commit(serial)`; the field's transcript
/// (with object ids stripped) is the golden's lines.
#[test]
#[ignore = "P0-H7: written; runs once P0-B-headless (Fixture::start) and P0-T-client (TestClient) land"]
fn ime_commit_transcript() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let client = fixture.connect(ConnectionRole::Public);
    let _ = fixture.client(client);
    todo!(
        "P0-T-client: the steps in this test's doc; compare with tests/golden/transcripts/text_input-commit.txt"
    )
}
