//! The fake logind (P1-F2): inhibitors held and released, `Lock` and `Unlock` signals.

use casement_testkit::fakes::{FakeLogind, LogindCall};

#[test]
#[ignore = "P1-F2"]
fn an_inhibitor_is_held_until_its_pipe_closes() {
    let _ = LogindCall::Inhibit(String::new());
    let _ = FakeLogind;
    todo!("P1-F2: Inhibit returns a pipe; closing it records InhibitReleased");
}

#[test]
#[ignore = "P1-F2"]
fn lock_and_unlock_signals_reach_a_listener() {
    todo!("P1-F2: emit Lock then Unlock on the session object, a listener on the bus hears both");
}

#[test]
#[ignore = "P1-F2"]
fn prepare_for_sleep_waits_for_inhibitors() {
    todo!("P1-F2: PrepareForSleep(true) is held until the inhibitor is released");
}
