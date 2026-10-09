//! `output` (P0-H1).

use casement_backend::VirtualOutputSpec;
use casement_core::OutputName;
use casement_testclient::TestClient;
use casement_testclient::surface::subsurface::bind_outputs;

use casement_core::policy::ConnectionRole;
use casement_testkit::{ClientId, Fixture, FixtureSpec};

const THREE: [&str; 3] = [
    "HEADLESS-1:1920x1080@60000:120:0,0",
    "HEADLESS-2:2880x1620@60000:180:1920,0",
    "HEADLESS-3:3840x2160@60000:240:3840,0",
];

fn boot_with(outputs: &[&str]) -> (Fixture, ClientId) {
    let mut fixture =
        Fixture::start(FixtureSpec::desktop().with_outputs(outputs)).expect("fixture");
    let id = fixture.connect(ConnectionRole::Public);
    fixture.pump();
    (fixture, id)
}

/// The registry names of the `wl_output` globals the client sees.
fn output_globals(client: &TestClient) -> Vec<u32> {
    client
        .globals()
        .contents()
        .clone_list()
        .into_iter()
        .filter(|global| global.interface == "wl_output")
        .map(|global| global.name)
        .collect()
}

/// Three outputs at scales 1, 1.5 and 2 each advertise wl_output with name, mode and the integer scale ceiling; xdg-output gives logical size.
#[test]
fn each_plugged_output_is_advertised_with_its_name_and_scale() {
    let (mut fixture, id) = boot_with(&THREE);
    // Every version of wl_output the global offers (1 to 4) is bound once per output.
    for version in 1..=4_u32 {
        let outputs =
            bind_outputs(fixture.client(id), version, version == 4).expect("outputs bind");
        fixture.pump();
        assert_eq!(outputs.len(), 3, "version {version}");
        if version < 4 {
            continue;
        }
        // (name, mode, wl_output scale: the ceiling, xdg-output logical size)
        let expected = [
            ("HEADLESS-1", (1920, 1080, 60_000), 1, (1920, 1080)),
            ("HEADLESS-2", (2880, 1620, 60_000), 2, (1920, 1080)),
            ("HEADLESS-3", (3840, 2160, 60_000), 2, (1920, 1080)),
        ];
        for (name, mode, scale, logical) in expected {
            let facts = outputs
                .iter()
                .map(|output| output.facts(fixture.client(id)))
                .find(|facts| facts.name.as_deref() == Some(name))
                .unwrap_or_else(|| panic!("{name} is advertised"));
            assert_eq!(facts.mode, Some(mode), "{name}");
            assert_eq!(facts.scale, Some(scale), "{name}");
            assert_eq!(facts.logical_size, Some(logical), "{name}");
            assert!(facts.done >= 1, "{name}");
        }
    }
    assert!(fixture.client(id).protocol_error().is_none());
}

/// A plugged output is advertised to a client that is already connected.
#[test]
fn a_hotplugged_output_appears_in_the_registry() {
    let (mut fixture, id) = boot_with(&THREE[..1]);
    assert_eq!(output_globals(fixture.client(id)).len(), 1);
    fixture.plug(VirtualOutputSpec::parse(THREE[1]).expect("a spec"));
    fixture.pump();
    assert_eq!(output_globals(fixture.client(id)).len(), 2);
}

/// Unplugging removes the global; a client bound to it sees it gone.
#[test]
fn an_unplugged_output_global_is_withdrawn() {
    let (mut fixture, id) = boot_with(&THREE);
    let bound = bind_outputs(fixture.client(id), 4, true).expect("outputs bind");
    fixture.pump();
    assert_eq!(bound.len(), 3);
    let doomed = bound
        .iter()
        .find(|output| output.facts(fixture.client(id)).name.as_deref() == Some("HEADLESS-2"))
        .expect("HEADLESS-2 is bound")
        .name;
    fixture.unplug(&OutputName::parse("HEADLESS-2").expect("a name"));
    fixture.pump();
    let client = fixture.client(id);
    let remaining = output_globals(client);
    assert_eq!(remaining.len(), 2);
    assert!(!remaining.contains(&doomed));
    assert!(client.protocol_error().is_none());
    assert!(client.transcript().0.iter().any(|line| {
        line.object.starts_with("wl_registry#")
            && line.event == "global_remove"
            && line.args == [doomed.to_string()]
    }));
}
