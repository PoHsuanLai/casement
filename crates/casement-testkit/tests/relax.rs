//! A37, in a process of its own (the policy is per process, and this is the only test here):
//! with the lenient policy a buffer attached before the first ack maps the window instead of
//! killing the client, which is what wlcs' helpers do.

use casement::relax::{self, AckPolicy};
use casement_core::geometry::PhysicalSize;
use casement_core::policy::ConnectionRole;
use casement_testclient::surface::toplevel::WmBase;
use casement_testclient::{Pattern, shm_buffer};
use casement_testkit::{Fixture, FixtureSpec};

#[test]
fn a_buffer_before_the_ack_maps_the_window_under_the_lenient_policy() {
    relax::set_policy(AckPolicy::Lenient);
    assert_eq!(relax::policy(), AckPolicy::Lenient);
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let id = fixture.connect(ConnectionRole::Public);
    let base = WmBase::bind(fixture.client(id), 7);
    let top = base.toplevel(fixture.client(id));
    // No bare commit, no ack: the buffer goes first.
    let buffer = shm_buffer(
        fixture.client(id),
        PhysicalSize { w: 64, h: 48 },
        Pattern::Gradient,
    );
    top.show(&buffer);
    fixture.pump();
    assert_eq!(
        fixture.client(id).protocol_error(),
        None,
        "no protocol error"
    );
    assert_eq!(fixture.model().windows().len(), 1, "the window mapped");
}
