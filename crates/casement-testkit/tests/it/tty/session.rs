//! The session through `FakeSession` and `FakeUdev` (P1-D1).

use casement_backend::tty::{
    DeviceEvent, DevicePath, SessionNote, TtySession, VtNumber, vt_of_keysym,
};
use casement_core::input::Keysym;
use casement_testkit::fakes::{FakeSession, FakeUdev, SessionFixture};
use std::path::PathBuf;

fn card0() -> DevicePath {
    DevicePath(PathBuf::from("/dev/dri/card0"))
}

#[test]
#[ignore = "P1-D1"]
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
#[ignore = "P1-D1"]
fn prepare_for_sleep_pauses_through_the_fake_logind() {
    todo!("P1-D1: PrepareForSleep(true) from the fake logind pauses the backend, false resumes it");
}

#[test]
#[ignore = "P1-D1"]
fn the_vt_chords_map_to_vt_numbers() {
    assert_eq!(vt_of_keysym(Keysym(0x1008fe01)), Some(VtNumber(1)));
    assert_eq!(vt_of_keysym(Keysym(0x1008fe0c)), Some(VtNumber(12)));
    assert_eq!(vt_of_keysym(Keysym(0x20)), None);
}

#[test]
#[ignore = "P1-D1"]
fn udev_events_reach_the_backend_in_order() {
    let udev = FakeUdev::scripted(vec![
        DeviceEvent::Added(card0()),
        DeviceEvent::Changed(card0()),
        DeviceEvent::Removed(card0()),
    ]);
    let _ = udev;
    todo!("P1-D1: TtyBackend::dispatch yields the connector events of each, in order");
}
