//! The FakeKms suite (P1-D2): modeset, mode pick, hotplug, DPMS, VRR policy, topologies.

use casement_backend::headless::ConnectorEvent;
use casement_backend::tty::{
    CommitVerdict, ConnectorId, ConnectorStatus, CrtcId, DevicePath, EdidError, Kms, KmsCommit,
    KmsError, KmsRefusal, KnownConnectors, ModesetPolicy, OutputCommit, TtyDevices, VrrPolicy,
    VrrState, output_facts, parse_edid, plan_outputs, reconcile,
};
use casement_core::OutputName;
use casement_core::geometry::PhysicalSize;
use casement_core::output::{EdidSerial, GammaRamp, ModeWish, OutputPower, pick_mode};
use casement_testkit::fakes::{FakeKms, FakeKmsOpener, SyntheticTopology, edid, load_fixture};
use std::collections::BTreeMap;
use std::path::PathBuf;

fn card(n: u32) -> DevicePath {
    DevicePath(PathBuf::from(format!("/dev/dri/card{n}")))
}

fn name(text: &str) -> OutputName {
    OutputName::parse(text).expect("name")
}

fn no_wishes() -> BTreeMap<OutputName, ModeWish> {
    BTreeMap::new()
}

fn first_commit(kms: &FakeKms, policy: VrrPolicy) -> KmsCommit {
    let plan = plan_outputs(&kms.resources(), &no_wishes(), policy);
    KmsCommit::new(plan, ModesetPolicy::Allow)
}

#[test]
fn the_laptop_panel_modesets_to_its_preferred_mode() {
    let mut kms = FakeKms::synthetic(SyntheticTopology::LaptopPanel);
    let commit = first_commit(&kms, VrrPolicy::Off);
    assert_eq!(commit.outputs.len(), 1);
    assert_eq!(kms.test(&commit), CommitVerdict::Accepted);
    kms.commit(commit).expect("commit");
    let lit = kms.world().lit.clone();
    let panel = lit.get(&ConnectorId(1)).expect("lit");
    assert_eq!(panel.mode.size, PhysicalSize { w: 2880, h: 1800 });
    assert_eq!(panel.mode.refresh.0, 120_000);
    assert_eq!(kms.commits().len(), 1);
}

#[test]
fn a_frame_commit_after_the_modeset_needs_none_and_a_mode_change_does() {
    let mut kms = FakeKms::synthetic(SyntheticTopology::LaptopPanel);
    let commit = first_commit(&kms, VrrPolicy::Off);
    kms.commit(commit.clone()).expect("modeset");
    let frame = KmsCommit::new(commit.outputs.clone(), ModesetPolicy::Forbid);
    assert_eq!(kms.test(&frame), CommitVerdict::Accepted);
    let slower = kms.resources().connectors[0].modes[1];
    let change = OutputCommit::new(ConnectorId(1), CrtcId(10), slower, VrrState::Off);
    let change = KmsCommit::new(vec![change], ModesetPolicy::Forbid);
    assert_eq!(kms.test(&change), CommitVerdict::NeedsModeset);
    assert!(matches!(
        kms.commit(change),
        Err(KmsError::Refused(KmsRefusal::ModesetForbidden))
    ));
}

#[test]
fn the_kernel_refuses_what_does_not_fit() {
    let mut kms = FakeKms::synthetic(SyntheticTopology::Dock);
    let plan = plan_outputs(&kms.resources(), &no_wishes(), VrrPolicy::Off);
    assert_eq!(plan.len(), 3);
    let mut crowded = plan.clone();
    crowded[1].crtc = crowded[0].crtc;
    let verdict = kms.test(&KmsCommit::new(crowded, ModesetPolicy::Allow));
    assert_eq!(verdict, CommitVerdict::Refused(KmsRefusal::NoCrtc));
    let mut odd = plan[..1].to_vec();
    odd[0].mode = kms.resources().connectors[1].modes[0];
    let verdict = kms.test(&KmsCommit::new(odd, ModesetPolicy::Allow));
    assert_eq!(verdict, CommitVerdict::Refused(KmsRefusal::Mode));
}

#[test]
fn mode_wishes_pick_among_the_modes_the_monitor_lists() {
    let kms = FakeKms::synthetic(SyntheticTopology::Dock);
    let mut wishes = no_wishes();
    wishes.insert(
        name("DP-1"),
        ModeWish::HighestRefreshAt(PhysicalSize { w: 2560, h: 1440 }),
    );
    let plan = plan_outputs(&kms.resources(), &wishes, VrrPolicy::Off);
    let dell = plan
        .iter()
        .find(|o| o.connector == ConnectorId(2))
        .expect("DP-1");
    assert_eq!(dell.mode.refresh.0, 144_000);
    let facts = output_facts(&kms.resources().connectors[2]);
    assert_eq!(pick_mode(&facts, &ModeWish::Preferred).size.w, 1920);
    let crtcs: Vec<CrtcId> = plan.iter().map(|o| o.crtc).collect();
    assert_eq!(crtcs, vec![CrtcId(10), CrtcId(11), CrtcId(12)]);
}

#[test]
fn a_hundred_hotplug_cycles_lose_and_repeat_nothing() {
    let dock = FakeKms::synthetic(SyntheticTopology::Dock);
    let monitor = dock.resources().connectors[1].clone();
    let mut opener = FakeKmsOpener::default();
    opener.devices.insert(card(0), dock.clone());
    let mut devices = TtyDevices::empty();
    let present = devices.added(card(0), &mut opener).expect("open");
    assert!(present.is_empty(), "held for the announcement");
    assert_eq!(devices.announce().len(), 3);
    assert!(devices.announce().is_empty(), "announced once");
    let key = output_facts(&monitor).key;
    for _ in 0..100 {
        dock.unplug(monitor.id);
        let gone = devices.changed(&card(0)).expect("changed");
        assert_eq!(gone, vec![ConnectorEvent::Unplugged(key.clone())]);
        assert!(devices.changed(&card(0)).expect("again").is_empty());
        dock.plug(monitor.clone());
        let back = devices.changed(&card(0)).expect("changed");
        assert_eq!(back, vec![ConnectorEvent::Plugged(output_facts(&monitor))]);
        assert!(devices.changed(&card(0)).expect("again").is_empty());
    }
    assert!(dock.world().pending.is_empty(), "every event was taken");
}

#[test]
fn swapping_the_display_on_a_connector_is_an_unplug_and_a_plug() {
    let kms = FakeKms::synthetic(SyntheticTopology::Dock);
    let before = reconcile(&KnownConnectors::default(), &kms.resources()).0;
    let mut other = kms.resources().connectors[1].clone();
    other.edid = edid("SAM", "ODYSSEY", Some("S1"));
    kms.plug(other.clone());
    let (_, events) = reconcile(&before, &kms.resources());
    assert_eq!(events.len(), 2);
    assert!(matches!(events[0], ConnectorEvent::Unplugged(_)));
    assert!(matches!(&events[1], ConnectorEvent::Plugged(f) if f.name == other.name));
}

#[test]
fn a_retraining_link_reports_changed_and_nothing_else() {
    let kms = FakeKms::synthetic(SyntheticTopology::RetrainingMonitor);
    let mut opener = FakeKmsOpener::default();
    opener.devices.insert(card(0), kms.clone());
    let mut devices = TtyDevices::empty();
    devices.added(card(0), &mut opener).expect("open");
    assert_eq!(devices.announce().len(), 1);
    for _ in 0..4 {
        let commit = first_commit(&kms, VrrPolicy::Off);
        let mut driver = devices.driving(&name("DP-1")).expect("driver");
        driver.commit(commit).expect("commit");
        let events = devices.changed(&card(0)).expect("changed");
        assert!(matches!(events.as_slice(), [ConnectorEvent::Changed(_)]));
    }
}

#[test]
fn removing_a_device_unplugs_its_connectors() {
    let dock = FakeKms::synthetic(SyntheticTopology::Dock);
    let mut opener = FakeKmsOpener::default();
    opener.devices.insert(card(0), dock);
    let mut devices = TtyDevices::empty();
    devices.added(card(0), &mut opener).expect("open");
    devices.announce();
    let events = devices.removed(&card(0));
    assert_eq!(events.len(), 3);
    assert!(
        events
            .iter()
            .all(|e| matches!(e, ConnectorEvent::Unplugged(_)))
    );
    assert!(devices.removed(&card(0)).is_empty());
    assert!(matches!(
        devices.changed(&card(0)),
        Err(KmsError::DeviceGone(_))
    ));
    assert!(matches!(
        devices.added(card(7), &mut opener),
        Err(KmsError::DeviceGone(_))
    ));
}

#[test]
fn a_device_added_after_the_announcement_reports_its_connectors_at_once() {
    let panel = FakeKms::synthetic(SyntheticTopology::LaptopPanel);
    let mut opener = FakeKmsOpener::default();
    opener.devices.insert(card(0), panel);
    let mut devices = TtyDevices::empty();
    assert!(devices.announce().is_empty(), "no device, nothing to say");
    let late = FakeKms::synthetic(SyntheticTopology::Dock);
    opener.devices.insert(card(1), late);
    let events = devices.added(card(1), &mut opener).expect("open");
    assert_eq!(events.len(), 3);
    assert!(
        events
            .iter()
            .all(|e| matches!(e, ConnectorEvent::Plugged(_)))
    );
}

#[test]
fn dpms_off_and_on_reach_the_device() {
    let mut kms = FakeKms::synthetic(SyntheticTopology::Dock);
    let id = ConnectorId(2);
    kms.set_power(id, OutputPower::Off).expect("off");
    assert_eq!(kms.power_of(id), Some(OutputPower::Off));
    kms.set_power(id, OutputPower::On).expect("on");
    assert_eq!(kms.power_of(id), Some(OutputPower::On));
    assert_eq!(kms.power_of(ConnectorId(1)), None);
    assert!(matches!(
        kms.set_power(ConnectorId(99), OutputPower::Off),
        Err(KmsError::UnknownConnector(ConnectorId(99)))
    ));
    kms.set_gamma(id, &GammaRamp::Identity).expect("gamma");
    assert_eq!(kms.world().gamma.get(&id), Some(&GammaRamp::Identity));
}

#[test]
fn variable_refresh_follows_the_policy_only_on_capable_monitors() {
    let mut kms = FakeKms::synthetic(SyntheticTopology::Dock);
    let off = plan_outputs(&kms.resources(), &no_wishes(), VrrPolicy::Off);
    assert!(off.iter().all(|o| o.vrr == VrrState::Off));
    let on = plan_outputs(&kms.resources(), &no_wishes(), VrrPolicy::WhenCapable);
    let states: Vec<VrrState> = on.iter().map(|o| o.vrr).collect();
    // The panel is capable, the Dell and the HDMI monitor are not.
    assert_eq!(states, vec![VrrState::On, VrrState::Off, VrrState::Off]);
    assert_eq!(
        kms.test(&KmsCommit::new(on, ModesetPolicy::Allow)),
        CommitVerdict::Accepted
    );
    let mut forced = off;
    forced[1].vrr = VrrState::On;
    assert_eq!(
        kms.test(&KmsCommit::new(forced, ModesetPolicy::Allow)),
        CommitVerdict::Refused(KmsRefusal::Unsupported)
    );
}

#[test]
fn a_connector_with_no_modes_is_left_dark() {
    let mut kms = FakeKms::synthetic(SyntheticTopology::NoModes);
    let resources = kms.resources();
    let facts = output_facts(&resources.connectors[0]);
    assert!(facts.modes.is_empty());
    assert_eq!(pick_mode(&facts, &ModeWish::Preferred).size.w, 0);
    assert!(plan_outputs(&resources, &no_wishes(), VrrPolicy::WhenCapable).is_empty());
    let commit = KmsCommit::new(Vec::new(), ModesetPolicy::Allow);
    assert_eq!(kms.test(&commit), CommitVerdict::Accepted);
    // It still counts as a plugged display, anonymous and keyed by its connector.
    let (known, events) = reconcile(&KnownConnectors::default(), &resources);
    assert_eq!(known.0.len(), 1);
    assert!(
        matches!(events.as_slice(), [ConnectorEvent::Plugged(f)] if f.key.as_str() == "connector:DP-1")
    );
}

#[test]
fn an_unplugged_connector_goes_dark_and_cannot_be_driven() {
    let mut kms = FakeKms::synthetic(SyntheticTopology::Dock);
    let commit = first_commit(&kms, VrrPolicy::Off);
    kms.commit(commit.clone()).expect("commit");
    kms.unplug(ConnectorId(2));
    assert!(!kms.world().lit.contains_key(&ConnectorId(2)));
    assert_eq!(
        kms.resources().connectors[1].status,
        ConnectorStatus::Disconnected
    );
    assert_eq!(
        kms.test(&commit),
        CommitVerdict::Refused(KmsRefusal::Unsupported)
    );
    assert_eq!(
        plan_outputs(&kms.resources(), &no_wishes(), VrrPolicy::Off).len(),
        2
    );
}

#[test]
fn edids_are_read_and_bad_ones_are_named() {
    let block = edid("DEL", "DELL U2723QE", Some("7GXK123"));
    let facts = parse_edid(&block).expect("edid");
    assert_eq!(facts.make, "DEL");
    assert_eq!(facts.model, "DELL U2723QE");
    assert_eq!(facts.serial, EdidSerial::Known("7GXK123".to_owned()));
    let anonymous = parse_edid(&edid("BOE", "NE135A1M", None)).expect("edid");
    assert_eq!(anonymous.serial, EdidSerial::Unknown);
    assert_eq!(parse_edid(&block[..64]), Err(EdidError::TooShort(64)));
    let mut bad = block.clone();
    bad[0] = 1;
    assert_eq!(parse_edid(&bad), Err(EdidError::BadHeader));
    let mut bad = block;
    bad[20] ^= 0xff;
    assert_eq!(parse_edid(&bad), Err(EdidError::BadChecksum));
}

#[test]
fn recorded_fixtures_replay() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures/kms");
    let mut replayed = 0;
    for entry in std::fs::read_dir(&dir).expect("fixtures dir") {
        let path = entry.expect("entry").path();
        if path.extension().is_none_or(|ext| ext != "json") {
            continue;
        }
        let resources = load_fixture(&path).expect("fixture parses");
        let mut kms = FakeKms::with(resources);
        let commit = first_commit(&kms, VrrPolicy::WhenCapable);
        let attached = kms
            .resources()
            .connectors
            .iter()
            .filter(|c| c.status == ConnectorStatus::Connected)
            .count();
        assert_eq!(commit.outputs.len(), attached, "{}", path.display());
        assert_eq!(
            kms.test(&commit),
            CommitVerdict::Accepted,
            "{}",
            path.display()
        );
        kms.commit(commit).expect("commit");
        replayed += 1;
    }
    assert!(replayed >= 2, "the fixtures are there");
}

#[test]
fn a_fixture_that_is_not_a_topology_is_a_typed_error() {
    let missing = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures/kms/none.json");
    assert!(matches!(
        load_fixture(&missing),
        Err(casement_testkit::fakes::KmsFixtureError::Io { .. })
    ));
    let readme = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures/kms/README.md");
    assert!(matches!(
        load_fixture(&readme),
        Err(casement_testkit::fakes::KmsFixtureError::Parse { .. })
    ));
}
