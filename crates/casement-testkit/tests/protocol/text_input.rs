//! `text_input` (P0-H7).

use casement_core::policy::ConnectionRole;
use casement_testkit::{Fixture, FixtureSpec};

/// Enter, enable, then the first key goes to the input method, not the document (SP1 A5).
#[test]
#[ignore = "P0-H7"]
fn the_first_key_after_enable_reaches_the_input_method() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let client = fixture.connect(ConnectionRole::Public);
    let _ = fixture.client(client);
    todo!(
        "P0-H7: enter, enable, then the first key goes to the input method, not the document (SP1 A5)"
    )
}

/// Preedit then commit_string: the transcript matches tests/golden/transcripts/text_input-commit.txt.
#[test]
#[ignore = "P0-H7"]
fn ime_commit_transcript() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let client = fixture.connect(ConnectionRole::Public);
    let _ = fixture.client(client);
    todo!(
        "P0-H7: preedit then commit_string: the transcript matches tests/golden/transcripts/text_input-commit.txt"
    )
}
