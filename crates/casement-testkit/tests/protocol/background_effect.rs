//! `background_effect` (P0-H8). Runs once the fixture and test client core land.

use casement_core::geometry::{LogicalPoint, LogicalRect, LogicalSize};
use casement_core::policy::ConnectionRole;
use casement_testclient::misc::background_effect::BlurAsk;
use casement_testkit::{Fixture, FixtureSpec};

fn rect(x: i32, y: i32, w: u32, h: u32) -> LogicalRect {
    LogicalRect::new(LogicalPoint { x, y }, LogicalSize { w, h })
}

/// The manager announces that it blurs when it is bound.
#[test]
fn the_manager_announces_blur() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let client = fixture.connect(ConnectionRole::Public);
    fixture.pump();
    let mut ask = BlurAsk::start(fixture.client(client).globals()).expect("the effect");
    fixture.pump();
    ask.pump().expect("pump");
    assert!(ask.blur_announced());
}

/// set_blur_region is stored for the surface and cleared by an empty region; the client is
/// never disconnected for it. (What the compositor stored is read through
/// `background_effect::blur_region_of`, which the scene tests of P0-R-scene draw from; this
/// file checks the protocol exchange stays clean through set, commit, unset, commit, destroy.)
#[test]
fn a_blur_region_is_stored() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let client = fixture.connect(ConnectionRole::Public);
    fixture.pump();
    let mut ask = BlurAsk::start(fixture.client(client).globals()).expect("the effect");
    fixture.pump();
    ask.set_blur(&[rect(0, 0, 100, 40), rect(20, 10, 10, 10)])
        .expect("set");
    ask.commit().expect("commit");
    fixture.pump();
    ask.unset_blur().expect("unset");
    ask.commit().expect("commit");
    fixture.pump();
    ask.destroy_effect().expect("destroy");
    fixture.pump();
    assert!(
        ask.pump().is_ok(),
        "the connection survived the whole exchange"
    );
}
