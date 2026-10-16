//! A tty backend that gets itself going again: a mode the display refuses, two causes that lend
//! the seat, a device that cannot take master back, a flip that never lands, frames that keep
//! failing and a screen that stays dead (the watchdog is then not fed).

use casement_backend::BackendError;
use casement_backend::SessionFault;
use casement_backend::headless::FrameSource;
use casement_backend::tty::{
    CommitVerdict, DevicePath, Health, Kms, KmsCommit, KmsError, KmsOpener, KmsRefusal,
    KmsResources, Lighting, NoGpus, PresentOutcome, Presented, QueueOutcome, QueuedFrame, Scanout,
    ScanoutSet, ScanoutSurface, SeatState, SessionNote, TtyBackend, TtyNote, TtySeams, TtySession,
    VtNumber,
};
use casement_core::OutputName;
use casement_core::geometry::{LogicalPoint, Transform};
use casement_core::gpu::GpuChoice;
use casement_core::output::{GammaRamp, Mode, OutputConfig, OutputPower};
use casement_core::time::{Millihertz, Mono, VblankClock, VblankSeq};
use casement_render::CasementElement;
use casement_testkit::fakes::{FakeKms, FakeScanout, FakeUdev, SyntheticTopology};
use ds_core::geometry::scale::Scale;
use smithay::backend::renderer::gles::GlesRenderer;
use smithay::backend::renderer::pixman::PixmanRenderer;
use smithay::backend::renderer::test::DummyRenderer;
use std::cell::{Cell, RefCell};
use std::os::fd::OwnedFd;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;

/// A scene with nothing in it: the fakes never draw.
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

fn at(ms: u64) -> Mono {
    Mono(Duration::from_millis(ms))
}

fn card0() -> DevicePath {
    DevicePath(PathBuf::from("/dev/dri/card0"))
}

/// A session that reports what the test pushes: the seat changing hands, and the machine
/// sleeping, apart as the real session keeps them.
#[derive(Debug, Clone, Default)]
struct Script(Rc<RefCell<(Vec<SessionNote>, Vec<SessionNote>)>>);

impl Script {
    fn seat(&self, note: SessionNote) {
        self.0.borrow_mut().0.push(note);
    }

    fn sleep(&self, note: SessionNote) {
        self.0.borrow_mut().1.push(note);
    }
}

impl TtySession for Script {
    fn open_device(&mut self, _path: &DevicePath) -> Result<OwnedFd, SessionFault> {
        Err(SessionFault::Inactive)
    }

    fn switch_vt(&mut self, _vt: VtNumber) -> Result<(), SessionFault> {
        Ok(())
    }

    fn notes(&mut self) -> Vec<SessionNote> {
        std::mem::take(&mut self.0.borrow_mut().0)
    }

    fn sleep_notes(&mut self) -> Vec<SessionNote> {
        std::mem::take(&mut self.0.borrow_mut().1)
    }
}

/// A device that refuses some modes and cannot take master back a number of times.
#[derive(Debug)]
struct Picky {
    inner: FakeKms,
    refuse: Vec<Mode>,
    resume_fails: Rc<Cell<u32>>,
}

impl Kms for Picky {
    fn resources(&self) -> KmsResources {
        self.inner.resources()
    }

    fn test(&mut self, commit: &KmsCommit) -> CommitVerdict {
        self.inner.test(commit)
    }

    fn commit(&mut self, commit: KmsCommit) -> Result<(), KmsError> {
        let refused = commit.outputs.iter().any(|output| {
            self.refuse
                .iter()
                .any(|mode| mode.size == output.mode.size && mode.refresh == output.mode.refresh)
        });
        match refused {
            true => Err(KmsError::Refused(KmsRefusal::Bandwidth)),
            false => self.inner.commit(commit),
        }
    }

    fn set_power(
        &mut self,
        connector: casement_backend::tty::ConnectorId,
        power: OutputPower,
    ) -> Result<(), KmsError> {
        self.inner.set_power(connector, power)
    }

    fn set_gamma(
        &mut self,
        connector: casement_backend::tty::ConnectorId,
        ramp: &GammaRamp,
    ) -> Result<(), KmsError> {
        self.inner.set_gamma(connector, ramp)
    }

    fn events(&mut self) -> Vec<casement_backend::headless::ConnectorEvent> {
        self.inner.events()
    }

    fn scanout(
        &mut self,
        connector: casement_backend::tty::ConnectorId,
        config: &OutputConfig,
    ) -> Result<Box<dyn ScanoutSurface>, KmsError> {
        self.inner.scanout(connector, config)
    }

    fn pause(&mut self) {
        self.inner.pause();
    }

    fn resume(&mut self) -> Result<(), KmsError> {
        match self.resume_fails.get() {
            0 => self.inner.resume(),
            left => {
                self.resume_fails.set(left - 1);
                Err(KmsError::DeviceGone(card0().0))
            }
        }
    }
}

/// Hands out the one device once.
#[derive(Debug)]
struct Opens(Option<Picky>);

impl KmsOpener for Opens {
    fn open(&mut self, path: &DevicePath) -> Result<Box<dyn Kms>, KmsError> {
        match self.0.take() {
            Some(device) => Ok(Box::new(device)),
            None => Err(KmsError::DeviceGone(path.0.clone())),
        }
    }
}

struct Rig {
    backend: TtyBackend,
    session: Script,
    kms: FakeKms,
    resume_fails: Rc<Cell<u32>>,
    name: OutputName,
    modes: Vec<Mode>,
}

/// A backend over a laptop panel whose device refuses the modes of `refuse` (by index in the
/// panel's list: 2880x1800 at 120 Hz preferred, 2880x1800 at 60 Hz, 1920x1200 at 60 Hz).
fn rig(refuse: &[usize]) -> Rig {
    let kms = FakeKms::synthetic(SyntheticTopology::LaptopPanel);
    let panel = SyntheticTopology::LaptopPanel.resources().connectors[0].clone();
    let resume_fails = Rc::new(Cell::new(0));
    let device = Picky {
        inner: kms.clone(),
        refuse: refuse.iter().map(|at| panel.modes[*at]).collect(),
        resume_fails: Rc::clone(&resume_fails),
    };
    let session = Script::default();
    let seams = TtySeams::new(
        Box::new(session.clone()),
        Box::new(FakeUdev::default().with_present(vec![card0()])),
        Box::new(Opens(Some(device))),
        Box::new(NoGpus),
        GpuChoice::Auto,
    );
    let mut backend = TtyBackend::start(seams).expect("start");
    assert_eq!(backend.announce().len(), 1, "the panel is announced");
    Rig {
        backend,
        session,
        kms,
        resume_fails,
        name: panel.name,
        modes: panel.modes,
    }
}

fn config(mode: Mode) -> OutputConfig {
    OutputConfig {
        mode,
        scale: Scale::ONE,
        transform: Transform::Normal,
        position: LogicalPoint { x: 0, y: 0 },
        power: OutputPower::On,
    }
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
fn a_refused_mode_falls_back_to_the_one_the_output_was_showing() {
    let mut rig = rig(&[0]);
    let (preferred, sixty) = (rig.modes[0], rig.modes[1]);
    assert_eq!(
        rig.backend
            .apply(&rig.name, &config(sixty), at(0))
            .expect("lit"),
        Lighting::Lit
    );
    let got = rig
        .backend
        .apply(&rig.name, &config(preferred), at(10))
        .expect("falls back");
    assert_eq!(got, Lighting::Fallback(sixty));
    assert_eq!(rig.backend.lit[&rig.name].config.mode, sixty);
    assert!(
        rig.backend.scanouts.get(&rig.name).is_some(),
        "still on the glass"
    );
}

#[test]
fn a_refused_first_mode_falls_back_to_the_preferred_then_the_smallest() {
    // name, refused indexes, asked for, lit at
    let cases = [
        ("the asked mode is not the preferred one", vec![1], 1, 0),
        ("the preferred one is refused too", vec![0, 1], 1, 2),
        ("the preferred one is asked for and refused", vec![0], 0, 2),
    ];
    for (name, refuse, asked, lit) in cases {
        let mut rig = rig(&refuse);
        let want = rig.modes[lit];
        let got = rig
            .backend
            .apply(&rig.name, &config(rig.modes[asked]), at(0))
            .expect(name);
        assert_eq!(got, Lighting::Fallback(want), "{name}");
        assert_eq!(rig.backend.lit[&rig.name].config.mode, want, "{name}");
    }
}

#[test]
fn an_output_no_mode_lights_is_an_error_and_not_a_panic() {
    let mut rig = rig(&[0, 1, 2]);
    let asked = config(rig.modes[0]);
    assert!(rig.backend.apply(&rig.name, &asked, at(0)).is_err());
    assert!(rig.backend.lit.is_empty());
}

#[test]
fn a_seat_another_session_holds_stays_lent_when_the_machine_wakes() {
    let mut rig = rig(&[]);
    rig.session.seat(SessionNote::Paused);
    assert_eq!(
        session_notes(&rig.backend.dispatch()),
        [SessionNote::Paused]
    );
    rig.session.sleep(SessionNote::Paused);
    assert!(session_notes(&rig.backend.dispatch()).is_empty());
    rig.session.sleep(SessionNote::Activated);
    assert!(session_notes(&rig.backend.dispatch()).is_empty());
    assert_eq!(
        rig.backend.seat,
        SeatState::Lent,
        "the other session has it"
    );
    rig.session.seat(SessionNote::Activated);
    assert_eq!(
        session_notes(&rig.backend.dispatch()),
        [SessionNote::Activated]
    );
    assert_eq!(rig.backend.seat, SeatState::Held);
    assert_eq!(rig.kms.seat_changes(), (1, 1), "given up once, taken once");
}

#[test]
fn a_session_that_returns_during_sleep_gets_the_seat_when_the_machine_wakes() {
    let mut rig = rig(&[]);
    rig.session.sleep(SessionNote::Paused);
    assert_eq!(
        session_notes(&rig.backend.dispatch()),
        [SessionNote::Paused]
    );
    rig.session.seat(SessionNote::Paused);
    rig.session.seat(SessionNote::Activated);
    assert!(session_notes(&rig.backend.dispatch()).is_empty());
    assert_eq!(rig.backend.seat, SeatState::Lent, "the machine is asleep");
    rig.session.sleep(SessionNote::Activated);
    assert_eq!(
        session_notes(&rig.backend.dispatch()),
        [SessionNote::Activated]
    );
    assert_eq!(rig.backend.seat, SeatState::Held);
}

#[test]
fn a_device_that_cannot_take_master_back_is_asked_again_and_draws_nothing_meanwhile() {
    let mut rig = rig(&[]);
    let lit = rig.modes[1];
    rig.backend
        .apply(&rig.name, &config(lit), at(0))
        .expect("lit");
    rig.session.seat(SessionNote::Paused);
    rig.backend.dispatch();
    // Fails when the seat returns and once more at the first retry; the second retry works.
    rig.resume_fails.set(2);
    rig.session.seat(SessionNote::Activated);
    assert_eq!(
        session_notes(&rig.backend.dispatch()),
        [SessionNote::Activated]
    );
    let frame = |rig: &mut Rig, ms| rig.backend.present(&rig.name.clone(), &Empty, at(ms));
    assert_eq!(frame(&mut rig, 0).expect("waits"), PresentOutcome::Waiting);
    assert_eq!(
        frame(&mut rig, 500).expect("waits"),
        PresentOutcome::Waiting
    );
    assert!(matches!(
        frame(&mut rig, 1_000).expect("draws"),
        PresentOutcome::Flipped(_)
    ));
    // The loop is told the seat is back, so it applies every output again.
    assert_eq!(
        session_notes(&rig.backend.dispatch()),
        [SessionNote::Activated]
    );
}

/// The vblank counter and reset count of a surface a test keeps hold of.
#[derive(Debug, Clone, Default)]
struct Probe {
    resets: Rc<Cell<usize>>,
    next: Rc<Cell<u64>>,
}

/// A surface that takes every frame and never reports one landing.
#[derive(Debug)]
struct Silent(Probe);

impl ScanoutSurface for Silent {
    fn queue(&mut self, _frame: QueuedFrame<'_>) -> QueueOutcome {
        let n = self.0.next.get();
        self.0.next.set(n + 1);
        QueueOutcome::Queued(VblankSeq(n))
    }

    fn presented(&mut self, _now: Mono) -> Option<Presented> {
        None
    }

    fn reset(&mut self) {
        self.0.resets.set(self.0.resets.get() + 1);
    }
}

/// A surface that fails every frame with an I/O error of this kind.
#[derive(Debug)]
struct Refusing(std::io::ErrorKind);

impl ScanoutSurface for Refusing {
    fn queue(&mut self, _frame: QueuedFrame<'_>) -> QueueOutcome {
        QueueOutcome::Failed(KmsError::Io(std::io::Error::from(self.0)))
    }

    fn presented(&mut self, _now: Mono) -> Option<Presented> {
        None
    }
}

fn output() -> OutputName {
    OutputName::parse("DP-1").expect("a name")
}

fn set_over(surface: Box<dyn ScanoutSurface>) -> ScanoutSet {
    let mut set = ScanoutSet::empty();
    let clock = VblankClock::new(Millihertz(60_000), at(0));
    set.attach(output(), Scanout::new(surface, clock));
    set
}

fn present(set: &mut ScanoutSet, ms: u64) -> Result<PresentOutcome, BackendError> {
    set.present(&output(), &Empty, at(ms))
}

#[test]
fn a_flip_that_never_lands_is_reset_and_then_the_output_is_set_up_again() {
    let probe = Probe::default();
    let mut set = set_over(Box::new(Silent(probe.clone())));
    let steps = [
        (0, PresentOutcome::Flipped(VblankSeq(0))),
        (400, PresentOutcome::Waiting),
        (600, PresentOutcome::Recovered),
        (700, PresentOutcome::Flipped(VblankSeq(1))),
        (1_300, PresentOutcome::Stalled),
    ];
    for (ms, want) in steps {
        assert_eq!(present(&mut set, ms).expect("present"), want, "at {ms} ms");
    }
    assert_eq!(probe.resets.get(), 2);
}

#[test]
fn a_display_that_shows_nothing_for_ten_seconds_is_frozen_and_a_landing_flip_clears_it() {
    let mut dead = set_over(Box::new(Silent(Probe::default())));
    present(&mut dead, 0).expect("queued");
    assert_eq!(dead.health(at(10_000)), Health::Alive);
    assert_eq!(dead.health(at(10_001)), Health::Frozen);
    // Recoveries in between do not start the count over.
    present(&mut dead, 600).expect("recovered");
    assert_eq!(dead.health(at(10_001)), Health::Frozen);

    let mut live = set_over(Box::new(FakeScanout::following()));
    present(&mut live, 0).expect("queued");
    present(&mut live, 16).expect("landed, queued again");
    // The flip queued at 16 ms lands at the next look, a minute on: frames flow.
    present(&mut live, 60_000).expect("landed, queued again");
    assert_eq!(live.health(at(60_001)), Health::Alive);
    assert_eq!(ScanoutSet::empty().health(at(60_000)), Health::Alive);
}

#[test]
fn three_failed_frames_in_a_row_recover_the_output() {
    let mut set = set_over(Box::new(Refusing(std::io::ErrorKind::Other)));
    assert!(present(&mut set, 0).is_err());
    assert!(present(&mut set, 1).is_err());
    assert_eq!(
        present(&mut set, 2).expect("recovered"),
        PresentOutcome::Recovered
    );
}

#[test]
fn a_frame_refused_with_permission_denied_is_a_lost_master() {
    let mut set = set_over(Box::new(Refusing(std::io::ErrorKind::PermissionDenied)));
    assert_eq!(
        present(&mut set, 0).expect("not an error"),
        PresentOutcome::MasterLost
    );
}
