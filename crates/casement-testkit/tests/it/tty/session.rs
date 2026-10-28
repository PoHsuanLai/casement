//! The session through `FakeSession` and `FakeUdev` (P1-D1).

use casement_backend::headless::ConnectorEvent;
use casement_backend::tty::{
    DeviceEvent, DevicePath, NoGpus, SeatState, SessionNote, TtyBackend, TtyNote, TtySeams,
    TtySession, VtNumber, vt_of_chord, vt_of_keysym,
};
use casement_core::gpu::{DrmNodeId, GpuChoice};
use casement_core::input::Keysym;
use casement_testkit::fakes::logind::manager::SleepPhase;
use casement_testkit::fakes::{
    FakeKms, FakeKmsOpener, FakeSession, FakeUdev, SessionFixture, SharedSession, SyntheticTopology,
};
use std::path::PathBuf;

fn card0() -> DevicePath {
    DevicePath(PathBuf::from("/dev/dri/card0"))
}

/// A backend over `session` and `udev` and a KMS opener holding `card0`.
fn backend(session: &SharedSession, udev: FakeUdev) -> TtyBackend {
    let mut opener = FakeKmsOpener::default();
    opener.devices.insert(
        card0(),
        FakeKms::with(SyntheticTopology::LaptopPanel.resources()),
    );
    let seams = TtySeams::new(
        Box::new(session.clone()),
        Box::new(udev),
        Box::new(opener),
        Box::new(NoGpus),
        GpuChoice::Auto,
    );
    TtyBackend::start(seams).expect("start")
}

fn session_notes(notes: &[TtyNote]) -> Vec<SessionNote> {
    notes
        .iter()
        .filter_map(|note| match note {
            TtyNote::Session(note) => Some(*note),
            _ => None,
        })
        .collect()
}

#[test]
fn a_vt_switch_pauses_and_the_return_activates() {
    let mut session = FakeSession::with(vec![SessionFixture {
        path: card0().0,
        bytes: Vec::new(),
    }]);
    session.switch_vt(VtNumber(3)).expect("switch");
    assert_eq!(session.notes(), [SessionNote::Paused]);
    session.resume();
    assert_eq!(session.notes(), [SessionNote::Activated]);
}

#[test]
fn the_backend_starts_holding_the_seat_and_touches_no_device() {
    let session = SharedSession::new(FakeSession::default());
    let mut backend = backend(&session, FakeUdev::default());
    assert_eq!(backend.seat, SeatState::Held);
    assert!(backend.devices.devices.is_empty());
    assert!(backend.dispatch().is_empty());
}

#[test]
fn a_vt_chord_switches_and_the_backend_pauses_then_resumes() {
    let session = SharedSession::new(FakeSession::default());
    let mut backend = backend(&session, FakeUdev::default());
    // Ctrl+Alt+F3 is XF86Switch_VT_3 after xkb.
    let asked = backend.request_vt(Keysym(0x1008fe03)).expect("switch");
    assert_eq!(asked, Some(VtNumber(3)));
    assert_eq!(session.switched_to(), Some(3));
    assert_eq!(session_notes(&backend.dispatch()), [SessionNote::Paused]);
    assert_eq!(backend.seat, SeatState::Lent);
    session.resume();
    assert_eq!(session_notes(&backend.dispatch()), [SessionNote::Activated]);
    assert_eq!(backend.seat, SeatState::Held);
}

#[test]
fn a_key_that_names_no_vt_switches_nothing() {
    let session = SharedSession::new(FakeSession::default());
    let mut backend = backend(&session, FakeUdev::default());
    assert_eq!(backend.request_vt(Keysym(0x20)).expect("no switch"), None);
    assert_eq!(session.switched_to(), None);
    assert!(backend.dispatch().is_empty());
}

#[test]
fn prepare_for_sleep_pauses_through_the_fake_logind() {
    let session = SharedSession::new(FakeSession::default());
    let mut backend = backend(&session, FakeUdev::default());
    session.prepare_for_sleep(SleepPhase::Starting);
    assert_eq!(session_notes(&backend.dispatch()), [SessionNote::Paused]);
    assert_eq!(backend.seat, SeatState::Lent);
    session.prepare_for_sleep(SleepPhase::Resumed);
    assert_eq!(session_notes(&backend.dispatch()), [SessionNote::Activated]);
    assert_eq!(backend.seat, SeatState::Held);
}

#[test]
fn the_vt_chords_map_to_vt_numbers() {
    assert_eq!(vt_of_keysym(Keysym(0x1008fe01)), Some(VtNumber(1)));
    assert_eq!(vt_of_keysym(Keysym(0x1008fe0c)), Some(VtNumber(12)));
    assert_eq!(vt_of_keysym(Keysym(0x1008fe0d)), None);
    assert_eq!(vt_of_keysym(Keysym(0x1008fe00)), None);
    assert_eq!(vt_of_keysym(Keysym(0x20)), None);
}

/// (held, modified, raw, expected)
type ChordRow<'a> = (bool, &'a [Keysym], &'a [Keysym], Option<VtNumber>);

#[test]
fn ctrl_alt_and_a_function_key_ask_for_that_vt_when_the_keymap_names_none() {
    let f = |n: u32| [Keysym(0xffbd + n)];
    let named = [Keysym(0x1008fe05)];
    let table: [ChordRow; 7] = [
        (true, &named, &f(5), Some(VtNumber(5))),
        (true, &f(1), &f(1), Some(VtNumber(1))),
        (true, &[], &f(12), Some(VtNumber(12))),
        (true, &[Keysym(0x1008ff11)], &[Keysym(0x1008ff11)], None),
        (false, &f(2), &f(2), None),
        (false, &named, &f(5), Some(VtNumber(5))),
        (true, &[Keysym(0xffca)], &[Keysym(0xffca)], None),
    ];
    for (held, modified, raw, want) in table {
        assert_eq!(
            vt_of_chord(held, modified, raw),
            want,
            "{held:?} {modified:?}"
        );
    }
}

#[test]
fn udev_events_reach_the_backend_in_order() {
    let session = SharedSession::new(FakeSession::default());
    let udev = FakeUdev::scripted(vec![
        DeviceEvent::Added(card0()),
        DeviceEvent::Changed(card0()),
        DeviceEvent::Removed(card0()),
    ]);
    let mut backend = backend(&session, udev);
    // Events before the start-up announcement are part of it, not reported twice.
    assert!(backend.announce().is_empty());
    let notes = backend.dispatch();
    let first = notes.first();
    let last = notes.last();
    assert!(matches!(
        first,
        Some(TtyNote::Connector(ConnectorEvent::Plugged(_)))
    ));
    assert!(matches!(
        last,
        Some(TtyNote::Connector(ConnectorEvent::Unplugged(_)))
    ));
}

#[test]
fn hotplug_waits_out_a_pause() {
    let session = SharedSession::new(FakeSession::default());
    let udev = FakeUdev::scripted(vec![DeviceEvent::Added(card0())]);
    let mut backend = backend(&session, udev);
    assert!(backend.announce().is_empty());
    session.prepare_for_sleep(SleepPhase::Starting);
    let paused = backend.dispatch();
    assert_eq!(session_notes(&paused), [SessionNote::Paused]);
    assert_eq!(paused.len(), 1);
    session.prepare_for_sleep(SleepPhase::Resumed);
    let back = backend.dispatch();
    assert_eq!(session_notes(&back), [SessionNote::Activated]);
    assert!(back.len() > 1);
}

#[test]
fn on_a_two_gpu_machine_the_backend_routes_each_output_to_the_render_gpu_or_a_copy() {
    use casement_backend::gpu::SysRenderNodes;
    use casement_core::OutputName;
    use casement_core::gpu::OutputRoute;
    use casement_testkit::gpu::{CrossImport, FabricatedSysRoot, GpuTopology};

    let topology = GpuTopology::pair(CrossImport::Direct).expect("the pair topology");
    let dir = tempfile::tempdir().expect("a directory for the fabricated /sys");
    let root = FabricatedSysRoot::build(&topology, dir.path()).expect("a fabricated tree");
    let seams = TtySeams::new(
        Box::new(SharedSession::new(FakeSession::default())),
        Box::new(FakeUdev::default()),
        Box::new(FakeKmsOpener::default()),
        Box::new(SysRenderNodes::new(root)),
        GpuChoice::Auto,
    );
    let backend = TtyBackend::start(seams).expect("start");
    let (render, other) = (DrmNodeId::new(226, 128), DrmNodeId::new(226, 129));
    let route = |text: &str| backend.route_of(&OutputName::parse(text).expect("a connector"));
    assert_eq!(route("eDP-1"), Some(OutputRoute::Direct { gpu: render }));
    assert_eq!(
        route("HDMI-A-1"),
        Some(OutputRoute::Copy {
            render,
            scanout: other
        })
    );
    assert_eq!(route("DP-9"), None);
}
