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

/// A toplevel that was unmapped by a null buffer and attaches a buffer again at once (wlcs'
/// remap tests) maps again, with fresh state, and the pointer reaches it.
#[test]
fn a_buffer_after_a_null_buffer_maps_the_window_again_under_the_lenient_policy() {
    use casement_core::input::DeviceClass;
    use casement_testclient::surface::toplevel::bind_pointer;
    use casement_testkit::TestOrigin;
    relax::set_policy(AckPolicy::Lenient);
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let id = fixture.connect(ConnectionRole::Public);
    let base = WmBase::bind(fixture.client(id), 7);
    let seat = bind_pointer(fixture.client(id));
    let top = base.toplevel(fixture.client(id));
    let size = PhysicalSize { w: 200, h: 100 };
    let buffer = shm_buffer(fixture.client(id), size, Pattern::Gradient);
    top.show(&buffer);
    fixture.pump();
    assert_eq!(fixture.model().windows().len(), 1, "the window mapped");
    top.surface.attach(None, 0, 0);
    top.surface.commit();
    fixture.pump();
    assert!(fixture.model().windows().is_empty(), "a null buffer unmaps");
    // Straight back, with no new configure acked.
    let again = shm_buffer(fixture.client(id), size, Pattern::Gradient);
    top.show(&again);
    fixture.pump();
    assert_eq!(fixture.client(id).protocol_error(), None);
    assert_eq!(
        fixture.model().windows().len(),
        1,
        "the window mapped again"
    );
    // It is hit where it is drawn.
    fixture.advance(std::time::Duration::from_secs(1));
    let mouse = fixture.input().plug(
        DeviceClass::Pointer,
        TestOrigin::Physical(DeviceClass::Pointer),
    );
    let centre = fixture
        .model()
        .outputs()
        .0
        .values()
        .next()
        .expect("an output")
        .config
        .logical_rect()
        .centre();
    fixture.input().pointer_to(mouse, centre);
    fixture.pump();
    assert!(
        seat.log.with(|seen| !seen.enters.is_empty()),
        "the pointer entered the remapped window"
    );
}
