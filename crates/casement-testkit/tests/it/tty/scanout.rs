//! The scheduler against `FakeScanout` and the virtual clock (P1-D3).

use casement_backend::RenderCostModel;
use casement_backend::headless::FrameSource;
use casement_backend::tty::{
    FlipTracker, PresentOutcome, Presented, QueueOutcome, QueuedFrame, Scanout, ScanoutSet,
    ScanoutSurface,
};
use casement_core::OutputName;
use casement_core::frame::{CursorPlaneCaps, CursorPlaneChoice, FrameOutcome};
use casement_core::geometry::PhysicalSize;
use casement_core::time::{Millihertz, Mono, VblankClock, VblankSeq};
use casement_render::CasementElement;
use casement_testkit::fakes::FakeScanout;
use ds_core::geometry::scale::Scale;
use smithay::backend::renderer::gles::GlesRenderer;
use smithay::backend::renderer::pixman::PixmanRenderer;
use smithay::backend::renderer::test::DummyRenderer;
use std::cell::RefCell;
use std::rc::Rc;
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

/// The fake, shared so the test can feed its script while the `Scanout` owns the handle.
#[derive(Debug, Clone)]
struct Shared(Rc<RefCell<FakeScanout>>);

impl ScanoutSurface for Shared {
    fn queue(&mut self, frame: QueuedFrame<'_>) -> QueueOutcome {
        self.0.borrow_mut().queue(frame)
    }
    fn presented(&mut self, now: Mono) -> Option<Presented> {
        self.0.borrow_mut().presented(now)
    }
}

fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}

fn clock() -> VblankClock {
    VblankClock::new(Millihertz(60_000), Mono(Duration::ZERO))
}

fn output() -> OutputName {
    OutputName::parse("DP-1").expect("a name")
}

/// Runs `frames` back-to-back animation frames at 60 Hz on the virtual clock: each starts at
/// the scheduler's `render_at`, costs what `cost` says and lands on the first vblank at or
/// after it ends. The set's tally afterwards.
fn run(frames: u64, cost: &RenderCostModel) -> (u64, u64) {
    let clock = clock();
    let fake = Shared(Rc::new(RefCell::new(FakeScanout::scripted(Vec::new()))));
    let mut set = ScanoutSet::empty();
    set.attach(output(), Scanout::new(Box::new(fake.clone()), clock));
    let mut now = Mono(Duration::ZERO);
    for n in 0..frames {
        let scanout = set.get_mut(&output()).expect("lit");
        let deadline = scanout.deadline(now, ms(1));
        let wanted = clock.seq_at(deadline.vblank).expect("on the grid");
        let spent = cost.cost_of(n).unwrap_or(Duration::ZERO);
        let done = deadline.render_at.after(spent);
        let (shown_at, shown) =
            clock.next_after(Mono(done.0.saturating_sub(Duration::from_nanos(1))));
        {
            let mut fake = fake.0.borrow_mut();
            fake.vblank = wanted;
            fake.script.push_back(Presented::new(shown, shown_at));
        }
        scanout.record_cost(spent);
        let queued = set.present(&output(), &Empty, now).expect("present");
        assert_eq!(queued, PresentOutcome::Flipped(wanted), "frame {n}");
        now = shown_at;
    }
    let tally = set.get_mut(&output()).map(|scanout| {
        let _ = scanout.settle(now);
        scanout.tally()
    });
    let tally = tally.expect("lit");
    (tally.presented, tally.missed)
}

#[test]
fn thirty_six_thousand_idle_frames_miss_nothing() {
    let (presented, missed) = run(36_000, &RenderCostModel::Fixed(ms(1)));
    assert_eq!((presented, missed), (36_000, 0));
}

#[test]
fn scripted_slow_frames_miss_exactly_as_scripted() {
    // One 25 ms frame in every 40 (frames 10, 50, ...), on a 16.7 ms period: each is
    // predicted cheap, so each misses once (by two vblanks); the frames after it are planned
    // for the 25 ms and make their vblanks.
    let mut script = vec![ms(1); 40];
    script[10] = ms(25);
    let (presented, missed) = run(400, &RenderCostModel::Script(script));
    assert_eq!((presented, missed), (400, 10));

    // Two slow frames in a row: the second is already predicted slow, so only one misses.
    let mut script = vec![ms(1); 40];
    script[10] = ms(25);
    script[11] = ms(25);
    let (_, missed) = run(40, &RenderCostModel::Script(script));
    assert_eq!(missed, 1);
}

#[test]
fn a_late_flip_is_judged_by_how_many_vblanks_it_missed() {
    let tracker = FlipTracker::new().queued(VblankSeq(4));
    let at = Presented::new(VblankSeq(6), Mono(ms(100)));
    assert_eq!(tracker.presented(at).1, FrameOutcome::Missed { by: 2 });
}

#[test]
fn a_busy_surface_defers_the_frame_to_the_next_vblank() {
    let mut fake = FakeScanout::scripted(vec![Presented::new(VblankSeq(0), Mono(ms(17)))]);
    let frame = || QueuedFrame::Rendered(casement_backend::headless::FramePixels::Dummy);
    assert!(matches!(
        fake.queue(frame()),
        QueueOutcome::Queued(VblankSeq(0))
    ));
    assert!(matches!(fake.queue(frame()), QueueOutcome::Busy));
    assert_eq!(fake.queued, 1);
    assert!(fake.presented(Mono(ms(17))).is_some());
    fake.vblank = VblankSeq(1);
    assert!(matches!(
        fake.queue(frame()),
        QueueOutcome::Queued(VblankSeq(1))
    ));

    // Through the set: a second present while the flip is pending queues nothing.
    let shared = Shared(Rc::new(RefCell::new(FakeScanout::scripted(vec![
        Presented::new(VblankSeq(0), Mono(ms(17))),
    ]))));
    let mut set = ScanoutSet::empty();
    set.attach(output(), Scanout::new(Box::new(shared.clone()), clock()));
    assert_eq!(
        set.present(&output(), &Empty, Mono(ms(0))).expect("first"),
        PresentOutcome::Flipped(VblankSeq(0))
    );
    shared.0.borrow_mut().script.clear();
    assert_eq!(
        set.present(&output(), &Empty, Mono(ms(1))).expect("second"),
        PresentOutcome::Waiting
    );
    assert_eq!(shared.0.borrow().queued, 1);
}

#[test]
fn the_cursor_plane_decision_follows_the_caps() {
    let planeless = FakeScanout::scripted(Vec::new());
    let small = PhysicalSize { w: 32, h: 32 };
    let large = PhysicalSize { w: 128, h: 128 };
    assert_eq!(
        planeless.cursor_choice(small, Scale::ONE),
        CursorPlaneChoice::Composited
    );
    let planed = planeless.with_cursor(CursorPlaneCaps::new(Some(PhysicalSize { w: 64, h: 64 })));
    assert_eq!(
        planed.cursor_choice(small, Scale::ONE),
        CursorPlaneChoice::Hardware
    );
    assert_eq!(
        planed.cursor_choice(large, Scale::ONE),
        CursorPlaneChoice::Composited
    );
}

/// A surface whose every frame changes nothing on the glass.
#[derive(Debug)]
struct Idle;

impl ScanoutSurface for Idle {
    fn queue(&mut self, _frame: QueuedFrame<'_>) -> QueueOutcome {
        QueueOutcome::Unchanged
    }
    fn presented(&mut self, _now: Mono) -> Option<Presented> {
        None
    }
}

#[test]
fn a_frame_that_changes_nothing_is_unchanged_not_waiting_and_not_an_error() {
    let mut set = ScanoutSet::empty();
    set.attach(output(), Scanout::new(Box::new(Idle), clock()));
    assert_eq!(
        set.present(&output(), &Empty, Mono(ms(0)))
            .expect("present"),
        PresentOutcome::Unchanged
    );
    let other = OutputName::parse("DP-2").expect("a name");
    assert_eq!(
        set.present(&other, &Empty, Mono(ms(1))).expect("present"),
        PresentOutcome::Waiting,
        "an unlit output waits"
    );
}
