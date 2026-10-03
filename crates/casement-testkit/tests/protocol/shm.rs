//! `shm` (P0-H1).

use casement_core::geometry::PhysicalSize;
use casement_testclient::surface::subsurface::{Compositor, H1Data, RawPool, seen};
use casement_testclient::{ClientError, Pattern, shm_buffer};
use wayland_client::protocol::wl_shm::{Format, WlShm};

use casement_core::policy::ConnectionRole;
use casement_testclient::TestClient;
use casement_testkit::{ClientId, Env, Fixture, FixtureSpec};

fn boot() -> (Fixture, ClientId) {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let id = fixture.connect(ConnectionRole::Public);
    fixture.pump();
    (fixture, id)
}

/// The (interface, code) of the protocol error the compositor killed `id` with.
fn fault(fixture: &mut Fixture, id: ClientId) -> (String, u32) {
    let error = fixture
        .client(id)
        .protocol_error()
        .expect("the compositor posted an error");
    (error.interface, error.code)
}

/// Compares what `client` received, without the events of the objects of `skip` and without
/// instants, with `tests/golden/transcripts/<name>`; `CASEMENT_BLESS=1` replaces a differing
/// golden (CONVENTIONS addendum 12).
fn check(name: &str, client: &TestClient, skip: &[&str]) {
    let actual: String = client
        .transcript()
        .0
        .iter()
        .filter(|line| {
            !skip
                .iter()
                .any(|iface| line.object.starts_with(&format!("{iface}#")))
        })
        .map(|line| format!("{}.{}({})\n", line.object, line.event, line.args.join(", ")))
        .collect();
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden/transcripts")
        .join(name);
    let golden = std::fs::read_to_string(&path).unwrap_or_default();
    if golden == actual {
        return;
    }
    if Env::of_process().bless == casement_render::golden::Bless::Yes {
        std::fs::write(&path, &actual).expect("write the blessed golden");
        return;
    }
    panic!(
        "{} differs\n--- golden\n{golden}--- actual\n{actual}",
        path.display()
    );
}

fn formats(fixture: &mut Fixture, id: ClientId, version: u32) -> Vec<String> {
    let shm: WlShm = fixture
        .client(id)
        .bind(version..=version, H1Data)
        .expect("wl_shm binds");
    fixture.pump();
    seen(fixture.client(id), &shm)
        .into_iter()
        .filter(|(event, _)| event == "format")
        .filter_map(|(_, args)| args.into_iter().next())
        .collect()
}

/// Wl_shm advertises argb8888 and xrgb8888.
#[test]
#[ignore = "P0-B: needs Fixture::start (P0-B-headless), the loop (P0-B-loop) and the client wiring of Fixture::connect"]
fn argb_and_xrgb_are_advertised() {
    let (mut fixture, id) = boot();
    for version in [1, 2] {
        // wl_shm.format codes: argb8888 is 0, xrgb8888 is 1.
        assert_eq!(
            formats(&mut fixture, id, version),
            ["0", "1"],
            "version {version}"
        );
    }
    assert!(matches!(
        fixture.client(id).bind::<WlShm, _>(3..=3, H1Data),
        Err(ClientError::MissingGlobal(_))
    ));
}

/// The canonical flow's transcript: the two formats, once per bind.
#[test]
#[ignore = "P0-B: needs Fixture::start (P0-B-headless), the loop (P0-B-loop) and the client wiring of Fixture::connect"]
fn the_format_flow_transcript_matches_its_golden() {
    let (mut fixture, id) = boot();
    let _buffer = shm_buffer(
        fixture.client(id),
        PhysicalSize { w: 8, h: 8 },
        Pattern::Gradient,
    );
    fixture.pump();
    check(
        "shm-formats.txt",
        fixture.client(id),
        &["wl_registry", "wl_display"],
    );
}

/// A buffer past its pool's end is invalid_stride/invalid_fd and disconnects only that client.
#[test]
#[ignore = "P0-B: needs Fixture::start (P0-B-headless), the loop (P0-B-loop) and the client wiring of Fixture::connect"]
fn a_pool_too_small_for_its_buffer_is_a_protocol_error() {
    let (mut fixture, bad) = boot();
    let good = fixture.connect(ConnectionRole::Public);
    fixture.pump();
    let client = fixture.client(bad);
    let pool = RawPool::new(client, 2, 64);
    let _buffer = pool.buffer(client, 0, (100, 100), 400, Format::Argb8888);
    fixture.pump();
    // wl_shm.error.invalid_stride, posted on the pool.
    assert_eq!(fault(&mut fixture, bad), ("wl_shm_pool".to_owned(), 1));
    assert!(fixture.client(good).is_connected());
}

/// A format the compositor did not advertise is wl_shm.invalid_format (0).
#[test]
#[ignore = "P0-B: needs Fixture::start (P0-B-headless), the loop (P0-B-loop) and the client wiring of Fixture::connect"]
fn an_unadvertised_format_is_a_protocol_error() {
    let (mut fixture, id) = boot();
    let client = fixture.client(id);
    let pool = RawPool::new(client, 2, 4096);
    let _buffer = pool.buffer(client, 0, (8, 8), 16, Format::Rgb565);
    fixture.pump();
    assert_eq!(fault(&mut fixture, id), ("wl_shm_pool".to_owned(), 0));
}

/// An empty pool is wl_shm.invalid_stride (1), posted on the wl_shm.
#[test]
#[ignore = "P0-B: needs Fixture::start (P0-B-headless), the loop (P0-B-loop) and the client wiring of Fixture::connect"]
fn an_empty_pool_is_a_protocol_error() {
    let (mut fixture, id) = boot();
    let _pool = RawPool::new(fixture.client(id), 2, 0);
    fixture.pump();
    assert_eq!(fault(&mut fixture, id), ("wl_shm".to_owned(), 1));
}

/// Destroying a pool leaves its buffers valid (the mapping lives while a buffer does).
#[test]
#[ignore = "P0-B: needs Fixture::start (P0-B-headless), the loop (P0-B-loop) and the client wiring of Fixture::connect"]
fn a_buffer_outlives_its_pool() {
    let (mut fixture, id) = boot();
    let client = fixture.client(id);
    let size = PhysicalSize { w: 64, h: 32 };
    let pool = RawPool::new(client, 2, 64 * 32 * 4);
    pool.write(0, &Pattern::Gradient.bytes(size));
    let buffer = pool.buffer(client, 0, (64, 32), 256, Format::Argb8888);
    pool.pool.destroy();
    let surface = Compositor::bind(client, 5).surface(client);
    surface.attach_raw(&buffer);
    surface.commit();
    fixture.pump();
    assert!(fixture.client(id).protocol_error().is_none());
}
