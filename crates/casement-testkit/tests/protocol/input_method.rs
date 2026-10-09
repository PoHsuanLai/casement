//! `input_method` (P0-H7).

use crate::text_input::Typing;
use casement_testclient::ime::text_input::TextInputEvent;

/// The input method's commit_string reaches the focused text field: `commit_string` then `done`
/// carrying the serial of the field's own last `commit`.
#[test]
fn an_input_method_commits_to_the_focused_field() {
    let mut typing = Typing::start();
    typing.enable_and_grab();
    typing.ime.commit_string("你").expect("commit_string");
    typing.ime.commit_batch().expect("commit");
    typing.pump();
    let events: Vec<_> = typing
        .ime
        .field_events()
        .iter()
        .map(|line| line.event.clone())
        .collect();
    assert_eq!(
        events[events.len().saturating_sub(2)..],
        [
            TextInputEvent::CommitString(Some("你".to_owned())),
            TextInputEvent::Done(1)
        ]
    );
}
