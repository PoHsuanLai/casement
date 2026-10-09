//! `xdg_decoration` (P0-H2): client-side is preferred at P0 (server-side titles come with
//! `casement-chrome` at P3). Ignored until the fixture and the core loop are filled.

use crate::protocol::xdg_shell::{boot, check_golden, golden, lines_of};
use casement_core::policy::ConnectionRole;
use casement_testclient::misc::decoration::{DecorationError, DecorationManager, Mode};
use casement_testclient::surface::toplevel::WmBase;

/// A client asking for server-side decoration is told client_side at P0.
#[test]
fn client_side_is_preferred() {
    let (mut fixture, id) = boot();
    let base = WmBase::bind(fixture.client(id), 7);
    let manager =
        DecorationManager::bind(fixture.client(id)).expect("advertised to a public client");
    let top = base.toplevel(fixture.client(id));
    let decoration = manager.decorate(fixture.client(id), &top);
    top.commit_bare();
    fixture.pump();
    // The initial configure already carries the decision, for an object that wished nothing.
    assert_eq!(decoration.newest(), Some(Mode::ClientSide));
    let configures = top.configures().len();
    decoration.set_mode(Mode::ServerSide);
    fixture.pump();
    assert_eq!(
        decoration.newest(),
        Some(Mode::ClientSide),
        "the answer to server_side is client_side"
    );
    assert!(
        top.configures().len() > configures,
        "every set_mode is answered with a configure"
    );
    decoration.set_mode(Mode::ClientSide);
    decoration.decoration.unset_mode();
    fixture.pump();
    assert_eq!(decoration.newest(), Some(Mode::ClientSide));
    assert!(fixture.client(id).protocol_error().is_none());
}

/// A second decoration object for one toplevel is `already_constructed`, and only that client
/// is disconnected.
#[test]
fn a_second_decoration_is_already_constructed() {
    let (mut fixture, id) = boot();
    let other = fixture.connect(ConnectionRole::Public);
    fixture.pump();
    let base = WmBase::bind(fixture.client(id), 7);
    let manager = DecorationManager::bind(fixture.client(id)).expect("advertised");
    let top = base.toplevel(fixture.client(id));
    let _first = manager.decorate(fixture.client(id), &top);
    let _second = manager.decorate(fixture.client(id), &top);
    fixture.pump();
    let error = fixture
        .client(id)
        .protocol_error()
        .expect("a protocol error");
    assert_eq!(error.code, DecorationError::AlreadyConstructed as u32);
    assert!(fixture.client(other).protocol_error().is_none());
}

/// The decoration object is told the mode once, with the initial configure (a later request for
/// the same mode is answered by a configure, not a repeated `configure(mode)`): its transcript
/// matches tests/golden/transcripts/xdg_decoration-mode.txt.
#[test]
fn the_decoration_transcript_matches_its_golden() {
    let (mut fixture, id) = boot();
    let base = WmBase::bind(fixture.client(id), 7);
    let manager = DecorationManager::bind(fixture.client(id)).expect("advertised");
    let top = base.toplevel(fixture.client(id));
    let decoration = manager.decorate(fixture.client(id), &top);
    top.commit_bare();
    fixture.pump();
    decoration.set_mode(Mode::ServerSide);
    fixture.pump();
    let text = lines_of(&mut fixture, id, &["zxdg_toplevel_decoration_v1#"]);
    check_golden(&golden("xdg_decoration-mode.txt"), &text).expect("the decoration flow");
}
