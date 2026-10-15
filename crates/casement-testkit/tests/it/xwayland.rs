//! X11 clients through xwayland-satellite (P1-X1), with the fixture's `FakeSpawner`: the
//! sockets, the on-demand start, the restart. The real satellite runs in
//! `dev/accept/xwayland-*.sh`.

use casement::spawn::{SpawnError, SpawnSpec, Spawned, Spawner};
use casement::xwayland::{
    MAX_STARTS, Permit, Restarts, SatelliteRendering, ServeStep, WINDOW, WaylandSocket, X11Dir,
    XwaylandConfig, XwaylandError, XwaylandState,
};
use casement_core::time::Mono;
use casement_testkit::fakes::FakeSpawner;
use rustix::net::{self, AddressFamily, SocketAddrUnix, SocketType};
use std::os::unix::net::UnixStream;
use std::time::Duration;
use tempfile::TempDir;

const SATELLITE: &str = "/opt/satellite";

/// The session clock `s` seconds in.
fn at(s: u64) -> Mono {
    Mono(Duration::from_secs(s))
}

/// `MAX_STARTS` as a count of seconds.
fn max_starts() -> u64 {
    u64::try_from(MAX_STARTS).expect("small")
}

fn wayland() -> WaylandSocket {
    WaylandSocket("wayland-7".to_owned())
}

/// A state listening on a fresh directory, and the directory (kept alive by the caller).
fn listening() -> (XwaylandState, XwaylandConfig, TempDir) {
    let dir = TempDir::new().expect("tempdir");
    let config = XwaylandConfig::satellite(SATELLITE);
    let state = XwaylandState::bind(&config, &X11Dir(dir.path().to_owned())).expect("bound");
    (state, config, dir)
}

/// The display of a listening state (`:N`).
fn display_of(state: &XwaylandState) -> String {
    let env = state.env();
    assert_eq!(env.len(), 1, "one variable: {env:?}");
    assert_eq!(env[0].name, "DISPLAY");
    env[0].value.clone()
}

/// The path socket of `display` in `dir`.
fn path_of(dir: &TempDir, display: &str) -> std::path::PathBuf {
    dir.path()
        .join(format!("X{}", display.trim_start_matches(':')))
}

/// A spawner that cannot start anything.
#[derive(Debug)]
struct Refusing;

impl Spawner for Refusing {
    fn spawn(&mut self, _spec: SpawnSpec) -> Result<Spawned, SpawnError> {
        Err(SpawnError::Empty)
    }
}

/// Binding exports `DISPLAY` at once and starts nothing; no client, no satellite.
#[test]
fn sockets_are_held_and_display_is_exported_before_satellite_runs() {
    let (mut state, config, dir) = listening();
    let display = display_of(&state);
    assert!(display.starts_with(':'));
    assert!(path_of(&dir, &display).exists(), "the path socket is bound");
    let mut spawner = FakeSpawner::new();
    let step = state
        .serve(&config, &mut spawner, &wayland(), at(0))
        .expect("served");
    assert_eq!(step, ServeStep::Idle);
    assert!(spawner.started().is_empty(), "no client has connected");
}

/// The first connection starts satellite once: argv is the configured path, the display and a
/// `-listenfd` per socket, and it inherits those descriptors and the Wayland socket's name.
#[test]
fn satellite_starts_with_its_listen_sockets() {
    let (mut state, config, dir) = listening();
    let display = display_of(&state);
    let _client = UnixStream::connect(path_of(&dir, &display)).expect("connected");
    let mut spawner = FakeSpawner::new();
    assert_eq!(
        state.satellite_pid(),
        None,
        "not running yet: nothing to spare"
    );
    let step = state
        .serve(&config, &mut spawner, &wayland(), at(0))
        .expect("served");
    let ServeStep::Started(process) = step else {
        panic!("satellite should have started, got {step:?}");
    };
    assert_eq!(
        state.satellite_pid().map(i64::from),
        Some(i64::from(process.pid.0)),
        "force quit spares the running satellite's process"
    );
    let started = spawner.started();
    assert_eq!(started.len(), 1);
    let argv = &started[0].argv.0;
    assert_eq!(argv[0], SATELLITE);
    assert_eq!(argv[1], display);
    let flags: Vec<&str> = argv[2..].iter().step_by(2).map(String::as_str).collect();
    assert_eq!(
        flags,
        ["-listenfd", "-listenfd"],
        "path and abstract socket"
    );
    assert!(
        argv[3..]
            .iter()
            .step_by(2)
            .all(|n| n.parse::<i32>().is_ok())
    );
    assert_eq!(started[0].passed_fds, 2);
    assert!(
        started[0]
            .env
            .iter()
            .any(|v| v.name == "WAYLAND_DISPLAY" && v.value == "wayland-7")
    );
    assert_eq!(display_of(&state), display, "DISPLAY stays while it runs");
}

/// A client on the abstract socket (what libxcb tries first) starts it too, and a second
/// connection while satellite runs starts nothing more.
#[test]
fn the_abstract_socket_starts_it_and_a_running_satellite_is_left_alone() {
    let (mut state, config, dir) = listening();
    let display = display_of(&state);
    let path = path_of(&dir, &display);
    let fd = net::socket(AddressFamily::UNIX, SocketType::STREAM, None).expect("socket");
    let address = SocketAddrUnix::new_abstract_name(path.to_str().expect("utf-8").as_bytes())
        .expect("address");
    net::connect(&fd, &address).expect("connected to the abstract socket");
    let mut spawner = FakeSpawner::new();
    assert!(matches!(
        state.serve(&config, &mut spawner, &wayland(), at(0)),
        Ok(ServeStep::Started(_))
    ));
    let _second = UnixStream::connect(&path).expect("connected");
    assert_eq!(
        state
            .serve(&config, &mut spawner, &wayland(), at(0))
            .expect("served"),
        ServeStep::Idle
    );
    assert_eq!(spawner.started().len(), 1);
}

/// Off starts nothing, binds nothing and exports no `DISPLAY`.
#[test]
fn off_starts_nothing() {
    let dir = TempDir::new().expect("tempdir");
    let config = XwaylandConfig::off();
    let mut state = XwaylandState::bind(&config, &X11Dir(dir.path().to_owned())).expect("off");
    let mut spawner = FakeSpawner::new();
    let step = state
        .serve(&config, &mut spawner, &wayland(), at(0))
        .expect("served");
    assert_eq!(step, ServeStep::Idle);
    assert!(spawner.started().is_empty());
    assert!(state.env().is_empty());
    assert_eq!(std::fs::read_dir(dir.path()).expect("dir").count(), 0);
}

/// When satellite exits the sockets stay, and the next client starts it again.
#[test]
fn a_client_after_satellite_exited_starts_it_again() {
    let (mut state, config, dir) = listening();
    let display = display_of(&state);
    let path = path_of(&dir, &display);
    let mut spawner = FakeSpawner::new();
    let _first = UnixStream::connect(&path).expect("connected");
    state
        .serve(&config, &mut spawner, &wayland(), at(0))
        .expect("served");
    state.satellite_exited();
    assert_eq!(display_of(&state), display, "the display survives the exit");
    assert!(path.exists());
    // The first connection was never accepted (the fake started nothing): it still waits, so a
    // restart is due at once.
    assert!(matches!(
        state.serve(&config, &mut spawner, &wayland(), at(0)),
        Ok(ServeStep::Started(_))
    ));
    assert_eq!(spawner.started().len(), 2);
}

/// A failed start reports why, keeps the sockets, and a later turn starts it.
#[test]
fn a_failed_start_keeps_the_sockets_and_is_retried() {
    let (mut state, config, dir) = listening();
    let display = display_of(&state);
    let _client = UnixStream::connect(path_of(&dir, &display)).expect("connected");
    let error = state
        .serve(&config, &mut Refusing, &wayland(), at(0))
        .expect_err("the start fails");
    assert!(matches!(error, XwaylandError::Spawn(_)), "{error}");
    assert_eq!(display_of(&state), display);
    let mut spawner = FakeSpawner::new();
    assert!(matches!(
        state.serve(&config, &mut spawner, &wayland(), at(0)),
        Ok(ServeStep::Started(_))
    ));
}

/// Two instances in one directory take different displays, and dropping one frees its socket.
#[test]
fn displays_are_distinct_and_released_on_drop() {
    let dir = TempDir::new().expect("tempdir");
    let config = XwaylandConfig::satellite(SATELLITE);
    let x11 = X11Dir(dir.path().to_owned());
    let first = XwaylandState::bind(&config, &x11).expect("first");
    let second = XwaylandState::bind(&config, &x11).expect("second");
    let (a, b) = (display_of(&first), display_of(&second));
    assert_ne!(a, b);
    drop(first);
    assert!(!path_of(&dir, &a).exists());
    assert!(path_of(&dir, &b).exists());
}

/// Software rendering adds satellite's `-glamor none` (not `off`, which panics it); the default
/// adds nothing.
#[test]
fn software_rendering_passes_glamor_none() {
    let dir = TempDir::new().expect("tempdir");
    let x11 = X11Dir(dir.path().to_owned());
    let argv_of = |config: &XwaylandConfig| {
        let mut state = XwaylandState::bind(config, &x11).expect("bound");
        let _client = UnixStream::connect(path_of(&dir, &display_of(&state))).expect("connected");
        let mut spawner = FakeSpawner::new();
        state
            .serve(config, &mut spawner, &wayland(), at(0))
            .expect("served");
        spawner.started()[0].argv.0.clone()
    };
    let plain = argv_of(&XwaylandConfig::satellite(SATELLITE));
    assert!(!plain.iter().any(|a| a == "-glamor"), "{plain:?}");
    let soft =
        argv_of(&XwaylandConfig::satellite(SATELLITE).with_rendering(SatelliteRendering::Software));
    assert_eq!(soft[soft.len() - 2..], ["-glamor", "none"], "{soft:?}");
}

/// A satellite that keeps exiting with a client waiting is started `MAX_STARTS` times in a
/// window, then held until the oldest start leaves the window; `DISPLAY` stays throughout.
#[test]
fn a_crash_loop_is_held_and_released_after_the_window() {
    let (mut state, config, dir) = listening();
    let display = display_of(&state);
    let _client = UnixStream::connect(path_of(&dir, &display)).expect("connected");
    let mut spawner = FakeSpawner::new();
    for turn in 0..max_starts() {
        let step = state
            .serve(&config, &mut spawner, &wayland(), at(turn))
            .expect("served");
        assert!(matches!(step, ServeStep::Started(_)), "start {turn}");
        state.satellite_exited();
    }
    let held = state
        .serve(&config, &mut spawner, &wayland(), at(max_starts()))
        .expect("served");
    assert_eq!(
        held,
        ServeStep::Held {
            until: Mono(WINDOW)
        }
    );
    assert_eq!(spawner.started().len(), MAX_STARTS, "no start while held");
    assert_eq!(display_of(&state), display);
    let released = state
        .serve(&config, &mut spawner, &wayland(), Mono(WINDOW))
        .expect("served");
    assert!(matches!(released, ServeStep::Started(_)));
}

/// With no client waiting nothing is counted, however often the loop turns.
#[test]
fn idle_turns_do_not_use_up_starts() {
    let (mut state, config, _dir) = listening();
    let mut spawner = FakeSpawner::new();
    for turn in 0..(max_starts() * 3) {
        let step = state
            .serve(&config, &mut spawner, &wayland(), at(turn))
            .expect("served");
        assert_eq!(step, ServeStep::Idle);
    }
}

/// The ledger alone: starts older than the window are forgotten.
#[test]
fn restarts_forget_starts_older_than_the_window() {
    let mut restarts = Restarts::default();
    for _ in 0..MAX_STARTS {
        assert_eq!(restarts.permit(at(0)), Permit::Allowed);
    }
    assert_eq!(
        restarts.permit(at(1)),
        Permit::Held {
            until: Mono(WINDOW)
        }
    );
    assert_eq!(restarts.permit(Mono(WINDOW)), Permit::Allowed);
}

/// The acceptance scripts of the suite exist (they run in the jail, never in the gate): one
/// per row of the definition of done.
#[test]
fn the_acceptance_scripts_of_the_suite_exist() {
    let accept = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../dev/accept");
    for name in [
        "app-id",
        "input",
        "override-redirect",
        "clipboard",
        "scale",
        "activation",
    ] {
        let script = accept.join(format!("xwayland-{name}.sh"));
        assert!(script.is_file(), "{} is missing", script.display());
    }
}
