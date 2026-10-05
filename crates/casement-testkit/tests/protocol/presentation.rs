//! `presentation` (P0-H1).

use casement_core::OutputName;
use casement_core::geometry::PhysicalSize;
use casement_testclient::misc::presentation::{FeedbackEvent, Presentation, feedback_events};
use casement_testclient::surface::subsurface::Compositor;
use casement_testclient::{Pattern, shm_buffer};
use casement_testkit::FrameCount;

use casement_core::policy::ConnectionRole;
use casement_testkit::{ClientId, Fixture, FixtureSpec};

fn boot() -> (Fixture, ClientId) {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let id = fixture.connect(ConnectionRole::Public);
    fixture.pump();
    (fixture, id)
}

const CLOCK_MONOTONIC: u32 = 1;
const OUTPUT: &str = "HEADLESS-1";

/// The compositor announces CLOCK_MONOTONIC at both versions.
#[test]
fn every_version_binds_and_announces_its_clock() {
    let (mut fixture, id) = boot();
    for version in [1, 2] {
        let presentation = Presentation::bind(fixture.client(id), version);
        fixture.pump();
        assert_eq!(
            presentation.clock(fixture.client(id)),
            Some(CLOCK_MONOTONIC)
        );
    }
    assert!(fixture.client(id).protocol_error().is_none());
}

/// Presented carries the virtual vblank instant and sequence; discarded when the surface is covered.
#[test]
#[ignore = "P0-B: needs Fixture::start (P0-B-headless), the loop (P0-B-loop) and the client wiring of Fixture::connect"]
fn feedback_arrives_on_virtual_time() {
    let (mut fixture, id) = boot();
    let client = fixture.client(id);
    let presentation = Presentation::bind(client, 2);
    let surface = Compositor::bind(client, 5).surface(client);
    let buffer = shm_buffer(client, PhysicalSize { w: 16, h: 16 }, Pattern::Gradient);
    surface.attach(&buffer);
    let feedback = presentation.feedback(client, &surface);
    surface.commit();
    fixture.pump();
    assert!(
        feedback_events(fixture.client(id), &feedback).is_empty(),
        "nothing before a frame"
    );
    let started = fixture.model().now();
    fixture.frames(&OutputName::parse(OUTPUT).expect("a name"), FrameCount(1));
    fixture.pump();
    let ended = fixture.model().now();
    let seen = feedback_events(fixture.client(id), &feedback);
    let presented: Vec<_> = seen
        .iter()
        .filter_map(|event| match event {
            FeedbackEvent::Presented { at, refresh, .. } => Some((*at, *refresh)),
            _ => None,
        })
        .collect();
    assert_eq!(presented.len(), 1, "{seen:?}");
    let (at, refresh) = presented[0];
    // 60 Hz is 16 666 666 ns (truncated), and the frame was shown at a vblank inside the advance.
    assert_eq!(refresh, 16_666_666);
    assert!(
        started.0 <= at && at <= ended.0,
        "{at:?} not in {started:?}..{ended:?}"
    );
    assert!(fixture.client(id).protocol_error().is_none());
}

/// Feedback of a surface destroyed before its frame is discarded, never presented.
#[test]
fn feedback_of_a_destroyed_surface_is_discarded() {
    let (mut fixture, id) = boot();
    let client = fixture.client(id);
    let presentation = Presentation::bind(client, 2);
    let surface = Compositor::bind(client, 5).surface(client);
    let feedback = presentation.feedback(client, &surface);
    surface.commit();
    surface.destroy();
    fixture.pump();
    fixture.frames(&OutputName::parse(OUTPUT).expect("a name"), FrameCount(1));
    fixture.pump();
    assert_eq!(
        feedback_events(fixture.client(id), &feedback),
        vec![FeedbackEvent::Discarded]
    );
}

/// A second commit with feedback supersedes the first: the first is discarded, the second shown.
#[test]
fn a_superseded_commit_is_discarded() {
    let (mut fixture, id) = boot();
    let client = fixture.client(id);
    let presentation = Presentation::bind(client, 2);
    let surface = Compositor::bind(client, 5).surface(client);
    let first = presentation.feedback(client, &surface);
    surface.commit();
    let second = presentation.feedback(client, &surface);
    surface.commit();
    fixture.pump();
    fixture.frames(&OutputName::parse(OUTPUT).expect("a name"), FrameCount(1));
    fixture.pump();
    let client = fixture.client(id);
    assert_eq!(
        feedback_events(client, &first),
        vec![FeedbackEvent::Discarded]
    );
    assert!(
        feedback_events(client, &second)
            .iter()
            .any(|event| matches!(event, FeedbackEvent::Presented { .. }))
    );
}
