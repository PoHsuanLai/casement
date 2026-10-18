//! Drawing with the CPU alone (CASB-07): `--safe` builds only software scanouts and never asks
//! for the GPU, a GPU surface that cannot be made falls back for that output only, and a
//! software surface flips and reports presentation like the GPU's.

use casement_backend::tty::{
    ConnectorId, DevicePath, Drawing, NoGpus, Pace, PresentOutcome, RenderMode, TtyBackend,
    TtySeams, Turn,
};
use casement_core::OutputName;
use casement_core::geometry::{LogicalPoint, Transform};
use casement_core::gpu::GpuChoice;
use casement_core::output::{OutputConfig, OutputPower};
use casement_core::time::{Mono, VblankSeq};
use casement_testkit::fakes::{
    EmptyScene, FakeKms, FakeKmsOpener, FakeSession, FakeUdev, SharedSession, SyntheticTopology,
};
use ds_core::geometry::scale::Scale;
use std::path::PathBuf;
use std::time::Duration;

fn card() -> DevicePath {
    DevicePath(PathBuf::from("/dev/dri/card0"))
}

fn at(n: u64) -> Mono {
    Mono(Duration::from_millis(n))
}

/// A backend over `kms` (drawing as `mode`) with every connector of the dock announced.
struct Rig {
    backend: TtyBackend,
    kms: FakeKms,
    outputs: Vec<(OutputName, OutputConfig)>,
}

fn rig(mode: RenderMode) -> Rig {
    let kms = FakeKms::synthetic(SyntheticTopology::Dock).drawing(mode);
    let mut opener = FakeKmsOpener::default();
    opener.devices.insert(card(), kms.clone());
    let seams = TtySeams::new(
        Box::new(SharedSession::new(FakeSession::default())),
        Box::new(FakeUdev::scripted(Vec::new()).with_present(vec![card()])),
        Box::new(opener),
        Box::new(NoGpus),
        GpuChoice::Auto,
    );
    let mut backend = TtyBackend::start(seams).expect("start");
    assert_eq!(backend.announce().len(), 3, "the dock's three monitors");
    let outputs = SyntheticTopology::Dock
        .resources()
        .connectors
        .into_iter()
        .map(|connector| {
            let config = OutputConfig {
                mode: connector.modes[0],
                scale: Scale::ONE,
                transform: Transform::Normal,
                position: LogicalPoint { x: 0, y: 0 },
                power: OutputPower::On,
            };
            (connector.name, config)
        })
        .collect();
    Rig {
        backend,
        kms,
        outputs,
    }
}

impl Rig {
    fn light(&mut self) {
        for (name, config) in &self.outputs {
            self.backend.apply(name, config, at(0)).expect("lit");
        }
    }

    fn drawn_by(&self, connector: u32) -> Option<Drawing> {
        self.kms.drawn_by(ConnectorId(connector))
    }
}

#[test]
fn a_safe_session_draws_every_output_with_the_cpu_and_never_asks_for_the_gpu() {
    let mut rig = rig(RenderMode::Software);
    rig.light();
    assert_eq!(rig.kms.gpu_asked(), 0, "no GPU surface was even asked for");
    for connector in [1, 2, 3] {
        assert_eq!(
            rig.drawn_by(connector),
            Some(Drawing::Pixman),
            "{connector}"
        );
    }
}

#[test]
fn a_session_with_a_gpu_draws_every_output_with_it() {
    let mut rig = rig(RenderMode::Gpu);
    rig.light();
    assert_eq!(rig.kms.gpu_asked(), 3);
    for connector in [1, 2, 3] {
        assert_eq!(rig.drawn_by(connector), Some(Drawing::Gpu), "{connector}");
    }
}

#[test]
fn an_output_whose_gpu_surface_fails_is_drawn_by_the_cpu_and_the_others_keep_the_gpu() {
    let mut rig = rig(RenderMode::Gpu);
    rig.kms.refuse_gpu_on(ConnectorId(2));
    rig.light();
    assert_eq!(rig.drawn_by(1), Some(Drawing::Gpu));
    assert_eq!(
        rig.drawn_by(2),
        Some(Drawing::Pixman),
        "the one that failed"
    );
    assert_eq!(rig.drawn_by(3), Some(Drawing::Gpu));
    assert_eq!(rig.kms.gpu_asked(), 3, "every output was tried once");

    // Lit again at another mode, the failed output goes straight to the CPU: the driver that
    // refused it once is not asked again.
    let (name, config) = rig.outputs[1].clone();
    let smaller = OutputConfig {
        mode: *SyntheticTopology::Dock.resources().connectors[1]
            .modes
            .last()
            .expect("a smaller mode"),
        ..config
    };
    assert_ne!(smaller.mode, config.mode);
    rig.backend
        .apply(&name, &smaller, at(1))
        .expect("lit again");
    assert_eq!(rig.drawn_by(2), Some(Drawing::Pixman));
    assert_eq!(
        rig.kms.gpu_asked(),
        3,
        "the GPU was not asked for the output again"
    );
}

#[test]
fn a_software_output_flips_and_reports_presentation_like_a_gpu_one() {
    for mode in [RenderMode::Gpu, RenderMode::Software] {
        let mut rig = rig(mode);
        rig.light();
        let (name, _) = rig.outputs[0].clone();
        for n in 0..4u64 {
            let shown = rig
                .backend
                .present(&name, &EmptyScene, at(n + 1))
                .expect("frame");
            assert_eq!(
                shown,
                PresentOutcome::Flipped(VblankSeq(n)),
                "{mode:?} frame {n}"
            );
        }
        // Each present first took in the flip before it: three reached the glass, none late.
        let report = rig.backend.report();
        assert_eq!((report.flips, report.missed), (3, 0), "{mode:?}");
    }
}

#[test]
fn a_flip_in_flight_makes_the_next_frame_wait_and_a_lost_flip_is_not_reported() {
    let mut pace = Pace::new();
    assert_eq!(pace.turn(), Turn::Free(VblankSeq(0)));
    pace.queued();
    assert_eq!(pace.turn(), Turn::Busy);
    assert_eq!(pace.pending(), Some(VblankSeq(0)));
    let shown = pace.landed(at(16)).expect("the flip landed");
    assert_eq!((shown.seq, shown.at), (VblankSeq(0), at(16)));
    assert_eq!(pace.turn(), Turn::Free(VblankSeq(1)));
    assert_eq!(pace.landed(at(33)), None, "nothing was in flight");

    // The output went dark with a frame in flight: it never lands, and the count stays put.
    pace.queued();
    pace.lost();
    assert_eq!(pace.turn(), Turn::Free(VblankSeq(1)));
    assert_eq!(pace.landed(at(50)), None);
}
