//! The FakeKms suite (P1-D2): modeset, mode pick, hotplug, DPMS, VRR policy, topologies.

use casement_backend::tty::{CommitVerdict, Kms, KmsCommit, ModesetPolicy};
use casement_testkit::fakes::{FakeKms, SyntheticTopology};

#[test]
#[ignore = "P1-D2"]
fn the_laptop_panel_modesets_to_its_preferred_mode() {
    let mut kms = FakeKms::with(SyntheticTopology::LaptopPanel.resources());
    let verdict = kms.test(&KmsCommit::new(Vec::new(), ModesetPolicy::Allow));
    assert_eq!(verdict, CommitVerdict::Accepted);
    todo!("P1-D2: pick the mode, commit, read it back from the fake");
}

#[test]
#[ignore = "P1-D2"]
fn a_hundred_hotplug_cycles_lose_and_repeat_nothing() {
    todo!(
        "P1-D2: plug and unplug 100 times; each cycle gives exactly one Plugged and one Unplugged"
    );
}

#[test]
#[ignore = "P1-D2"]
fn dpms_off_and_on_reach_the_device() {
    todo!("P1-D2: set_power Off then On; the fake's world records both");
}

#[test]
#[ignore = "P1-D2"]
fn variable_refresh_follows_the_policy_only_on_capable_monitors() {
    todo!("P1-D2: VRR on a capable panel, never on an incapable monitor");
}

#[test]
#[ignore = "P1-D2"]
fn a_connector_with_no_modes_is_left_dark() {
    let _ = SyntheticTopology::NoModes.resources();
    todo!("P1-D2: the output is plugged with a 0x0 mode and left off, no scale or place derived");
}

#[test]
#[ignore = "P1-D2"]
fn recorded_fixtures_replay() {
    todo!("P1-D2: every file under fixtures/kms/ parses and replays through FakeKms");
}
