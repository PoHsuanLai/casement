//! The vkms test mode's seams over fakes (P1-V2): the direct session paused and activated by
//! the edge's link (what SIGUSR1 and SIGUSR2 do), the KMS report, and a writeback frame read
//! through `FakeWriteback` and written as the raw file and its meta line (what SIGHUP does).
//! No signal is raised and no device is opened.

use casement::cli::DirectTty;
use casement::test_mode::{Hooks, meta_path};
use casement_backend::headless::FrameSource;
use casement_backend::tty::{
    DeviceEvent, DevicePath, DeviceSource, DirectSession, FLIPS_TO_WAIT, FixedNode, NoGpus,
    POLL_EVERY, SeatState, SessionNote, SleepLink, TtyBackend, TtySeams, TtySession,
    WritebackError,
};
use casement_core::OutputName;
use casement_core::geometry::{LogicalPoint, PhysicalSize, Transform};
use casement_core::gpu::GpuChoice;
use casement_core::output::{Mode, OutputConfig, OutputPower};
use casement_core::time::Mono;
use casement_render::CasementElement;
use casement_testkit::fakes::{FakeKms, FakeKmsOpener, FakeWriteback, SyntheticTopology};
use ds_core::geometry::scale::Scale;
use smithay::backend::renderer::gles::GlesRenderer;
use smithay::backend::renderer::pixman::PixmanRenderer;
use smithay::backend::renderer::test::DummyRenderer;
use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::time::Duration;

/// A scene with nothing in it: the fake never draws.
#[derive(Debug)]
struct Empty;

impl FrameSource for Empty {
    fn pixman(&self, _: &mut PixmanRenderer) -> Vec<CasementElement<PixmanRenderer>> {
        Vec::new()
    }
    fn gles(&self, _: &mut GlesRenderer) -> Vec<CasementElement<GlesRenderer>> {
        Vec::new()
    }
    fn dummy(&self, _: &mut DummyRenderer) -> Vec<CasementElement<DummyRenderer>> {
        Vec::new()
    }
}

fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}

fn at(n: u64) -> Mono {
    Mono(ms(n))
}

/// A backend over the direct seams and a one-panel fake device, with one lit output.
struct Rig {
    backend: TtyBackend,
    link: SleepLink,
    kms: FakeKms,
    writeback: FakeWriteback,
    name: OutputName,
    mode: Mode,
    tick: u64,
}

fn rig_with(writeback: Option<FakeWriteback>) -> Rig {
    let kms = FakeKms::synthetic(SyntheticTopology::LaptopPanel);
    let card = DevicePath(PathBuf::from("/dev/dri/card1"));
    let mut opener = FakeKmsOpener::default();
    opener.devices.insert(card.clone(), kms.clone());
    let mut session = DirectSession::new(|| {});
    let link = session.sleep_link().expect("the edge's end");
    let fake = writeback.clone().unwrap_or_default();
    let seams = TtySeams::new(
        Box::new(session),
        Box::new(FixedNode::new(card)),
        Box::new(opener),
        Box::new(NoGpus),
        GpuChoice::Auto,
    );
    let seams = match writeback {
        Some(_) => seams.with_writeback(Box::new(fake.clone())),
        None => seams,
    };
    let mut backend = TtyBackend::start(seams).expect("start");
    let panel = kms.resources().connectors[0].clone();
    let mode = panel.modes[0];
    assert_eq!(backend.announce().len(), 1, "the one panel is announced");
    Rig {
        backend,
        link,
        kms,
        writeback: fake,
        name: panel.name,
        mode,
        tick: 0,
    }
}

fn rig() -> Rig {
    rig_with(Some(FakeWriteback::default()))
}

impl Rig {
    fn config(&self, power: OutputPower) -> OutputConfig {
        OutputConfig {
            mode: self.mode,
            scale: Scale::ONE,
            transform: Transform::Normal,
            position: LogicalPoint { x: 0, y: 0 },
            power,
        }
    }

    fn light(&mut self) {
        let config = self.config(OutputPower::On);
        self.backend
            .apply(&self.name, &config, at(0))
            .expect("the panel is lit");
    }

    /// `n` frames, one per millisecond of the virtual clock.
    fn frames(&mut self, n: u64) {
        for _ in 0..n {
            self.tick += 1;
            self.backend
                .present(&self.name, &Empty, at(self.tick))
                .expect("a frame");
        }
    }
}

fn direct(report: Option<PathBuf>, raw: Option<PathBuf>) -> DirectTty {
    let mut direct = DirectTty::new(PathBuf::from("/dev/dri/card1"));
    direct.report = report;
    direct.writeback_raw = raw;
    direct
}

#[test]
fn the_edge_pauses_and_activates_the_session_like_a_vt_switch() {
    let mut rig = rig();
    rig.light();
    rig.frames(4);
    let flips = rig.backend.report().flips;
    assert!(flips > 0);
    // SIGUSR1: the link says paused (nobody waits for the acknowledgement here).
    let _ = rig.link.paused(Duration::ZERO);
    let notes = rig.backend.dispatch();
    assert!(matches!(
        notes.as_slice(),
        [casement_backend::tty::TtyNote::Session(SessionNote::Paused)]
    ));
    assert_eq!(rig.backend.seat, SeatState::Lent);
    assert_eq!(rig.kms.seat_changes(), (1, 0));
    // No frame is queued while the seat is lent.
    rig.frames(4);
    assert_eq!(rig.backend.report().flips, flips);
    assert_eq!(rig.backend.report().seat, SeatState::Lent);
    // SIGUSR2: activated; the device takes master again and frames flow.
    rig.link.activated();
    let notes = rig.backend.dispatch();
    assert!(matches!(
        notes.as_slice(),
        [casement_backend::tty::TtyNote::Session(
            SessionNote::Activated
        )]
    ));
    assert_eq!(rig.backend.seat, SeatState::Held);
    assert_eq!(rig.kms.seat_changes(), (1, 1));
}

#[test]
fn the_fixed_node_reports_itself_changed_once_in_a_while_for_hotplug() {
    let card = DevicePath(PathBuf::from("/dev/dri/card1"));
    let mut node = FixedNode::new(card.clone());
    assert_eq!(node.present(), [card.clone()]);
    let period = usize::from(POLL_EVERY) + 1;
    let mut changed = 0;
    for turn in 1..=period * 3 {
        // One backend turn: ask until the source says none.
        let mut reported = 0;
        while let Some(event) = node.next_event() {
            assert_eq!(event, DeviceEvent::Changed(card.clone()));
            reported += 1;
        }
        changed += reported;
        assert_eq!(reported, usize::from(turn % period == 0), "turn {turn}");
    }
    assert_eq!(changed, 3);
}

#[test]
fn the_direct_session_opens_no_device_and_switches_no_vt() {
    let mut session = DirectSession::new(|| {});
    let node = DevicePath(PathBuf::from("/dev/dri/card1"));
    assert!(session.open_device(&node).is_err());
    assert!(
        session
            .switch_vt(casement_backend::tty::VtNumber(3))
            .is_err()
    );
    assert!(session.notes().is_empty());
}

#[test]
fn the_report_counts_flips_and_names_each_output() {
    let mut rig = rig();
    assert_eq!(rig.backend.report().flips, 0);
    assert!(rig.backend.report().outputs.is_empty());
    rig.light();
    rig.frames(6);
    let report = rig.backend.report();
    assert!(report.flips >= 5, "flips: {}", report.flips);
    assert_eq!(report.missed, 0);
    let line = report.json_line();
    let size = rig.mode.size;
    let want = format!(
        r#""outputs":[{{"name":"{}","mode":"{}x{}@{}","power":"on"}}]"#,
        rig.name.as_str(),
        size.w,
        size.h,
        rig.mode.refresh.0
    );
    assert!(line.contains(&want), "{line}");
    assert!(line.contains(r#""seat":"held""#), "{line}");
    // DPMS off keeps the output in the report, dark.
    let dark = rig.config(OutputPower::Off);
    rig.backend.apply(&rig.name, &dark, at(10)).expect("dark");
    assert!(
        rig.backend
            .report()
            .json_line()
            .contains(r#""power":"off""#)
    );
}

#[test]
fn a_dark_output_stops_the_flips_and_lighting_it_restarts_them() {
    let mut rig = rig();
    rig.light();
    rig.frames(4);
    let dark = rig.config(OutputPower::Off);
    rig.backend.apply(&rig.name, &dark, at(10)).expect("dark");
    let before = rig.backend.report().flips;
    rig.frames(4);
    assert_eq!(rig.backend.report().flips, before);
    rig.light();
    rig.frames(4);
    assert!(rig.backend.report().flips > before);
}

#[test]
fn the_hooks_append_one_report_line_a_second_of_session_time() {
    let dir = tempfile::tempdir().expect("scratch");
    let path = dir.path().join("report.jsonl");
    let (mut hooks, _asks) = Hooks::new(&direct(Some(path.clone()), None));
    let mut rig = rig();
    rig.light();
    for at_ms in [0, 400, 999, 1000, 1500, 2000] {
        hooks.turn(&mut rig.backend, at(at_ms));
    }
    let text = std::fs::read_to_string(&path).expect("the report");
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 3, "t = 0, 1 s and 2 s: {text}");
    for line in lines {
        assert!(
            line.starts_with(r#"{"flips":"#) && line.ends_with("]}"),
            "{line}"
        );
    }
}

#[test]
fn sighup_reads_a_frame_back_and_writes_the_raw_file_then_its_meta() {
    let dir = tempfile::tempdir().expect("scratch");
    let raw = dir.path().join("writeback.raw");
    let (mut hooks, asks): (Hooks, Sender<()>) = Hooks::new(&direct(None, Some(raw.clone())));
    let mut rig = rig();
    rig.light();
    rig.frames(2);
    // SIGHUP: the job is queued on the lit output's CRTC at its mode size.
    asks.send(()).expect("the signal");
    hooks.turn(&mut rig.backend, at(100));
    assert_eq!(rig.writeback.world().begun.len(), 1);
    assert_eq!(rig.writeback.world().begun[0].1, rig.mode.size);
    assert!(!raw.exists(), "nothing is written before the flips pass");
    // A second SIGHUP while the job waits is refused (logged), not queued.
    asks.send(()).expect("the signal");
    hooks.turn(&mut rig.backend, at(101));
    assert_eq!(rig.writeback.world().begun.len(), 1);
    // Enough flips later the frame is read and written.
    rig.frames(FLIPS_TO_WAIT + 2);
    hooks.turn(&mut rig.backend, at(200));
    let size: PhysicalSize = rig.mode.size;
    assert_eq!(
        std::fs::read(&raw).expect("the raw file"),
        FakeWriteback::pattern(size)
    );
    assert_eq!(
        std::fs::read_to_string(meta_path(&raw)).expect("the meta file"),
        format!("{} {} {} XR24\n", size.w, size.h, size.w * 4)
    );
    assert_eq!(rig.writeback.world().finished, 1);
    // The next SIGHUP starts another job.
    asks.send(()).expect("the signal");
    hooks.turn(&mut rig.backend, at(300));
    assert_eq!(rig.writeback.world().begun.len(), 2);
}

#[test]
fn a_writeback_job_is_refused_when_it_cannot_be_read() {
    // Without the seam (not in --drm-node mode with --writeback-raw).
    let mut off = rig_with(None);
    off.light();
    assert!(matches!(
        off.backend.request_writeback(),
        Err(WritebackError::NotEnabled)
    ));
    // Before any output is lit.
    let mut dark = rig();
    assert!(matches!(
        dark.backend.request_writeback(),
        Err(WritebackError::NoOutput)
    ));
    // While the seat is lent.
    let mut lent = rig();
    lent.light();
    let _ = lent.link.paused(Duration::ZERO);
    lent.backend.dispatch();
    assert!(matches!(
        lent.backend.request_writeback(),
        Err(WritebackError::SeatLent)
    ));
    // With a job already waiting.
    let mut busy = rig();
    busy.light();
    busy.backend.request_writeback().expect("first");
    assert!(matches!(
        busy.backend.request_writeback(),
        Err(WritebackError::Busy)
    ));
    assert!(busy.backend.take_writeback().is_none(), "not ready yet");
}
