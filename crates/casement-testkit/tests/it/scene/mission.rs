//! Mission Control open, as a picture: three framed windows spread to their slots with the
//! app icons under them, over the dimmed desktop, at 1x and 2x (goldens
//! `mission-control-1x` and `mission-control-2x`). Reduce Motion changes how the windows travel,
//! not where they rest, so its settled picture is the same one.

use super::desk::{Desk, Dpi, OUTPUT};
use super::golden_matches;
use casement_core::OutputName;
use casement_core::policy::ConnectionRole;
use casement_testclient::Pattern;
use casement_testclient::quire::shell::{Motion, OverviewMode, ShellHandle, Toggle};
use casement_testkit::{Fixture, FixtureSpec, FrameCount, ShotTarget};
use std::time::Duration;

/// How long Mission Control is left to open and settle.
const SETTLE: Duration = Duration::from_millis(1500);

/// The 1x and 2x outputs the pictures are shot on.
const OUTPUTS: [(&str, &str, Dpi); 2] = [
    ("1x", "HEADLESS-1:1920x1080@60000:120:0,0", Dpi::One),
    ("2x", "HEADLESS-1:3840x2160@60000:240:0,0", Dpi::Two),
];

/// Three framed windows on an output of `output` pixels at `dpi`, Mission Control opened by the
/// shell (under `motion`, when it says one) and settled, and the frame shot.
fn open(output: &str, dpi: Dpi, motion: Option<Motion>) -> image::RgbaImage {
    let spec = FixtureSpec::desktop().with_outputs(&[output]);
    let mut fixture = Fixture::start(spec).expect("fixture");
    let mut desk = Desk::at(&mut fixture);
    let windows = [
        ("org.example.files", "Files", Pattern::Gradient),
        ("org.example.notes", "Notes", Pattern::Checker { cell: 20 }),
        ("org.example.mail", "Mail", Pattern::Solid(0xff2a_3fd0)),
    ];
    for (app, title, pattern) in windows {
        let top = desk.framed_window(app, title, (640, 400), pattern, dpi);
        desk.kept.keep(top);
        desk.fixture.advance(Duration::from_millis(300));
    }
    let id = desk.fixture.connect(ConnectionRole::Shell);
    let shell = ShellHandle::bind(desk.fixture.client(id), 7);
    if let Some(motion) = motion {
        shell.set_motion(motion);
    }
    desk.fixture.pump();
    shell.overview(OverviewMode::MissionControl, Toggle::On);
    desk.fixture.pump();
    desk.fixture.advance(SETTLE);
    desk.kept.keep(shell);
    let name = OutputName::parse(OUTPUT).expect("a name");
    desk.fixture.frames(&name, FrameCount(1));
    desk.fixture.shot(ShotTarget::Output(name))
}

/// Mission Control open with three windows matches its golden `mission-control-<scale>`.
#[test]
fn mission_control_with_three_windows_matches_its_golden() {
    for (scale, output, dpi) in OUTPUTS {
        let shot = open(output, dpi, None);
        golden_matches(&format!("mission-control-{scale}"), &shot);
    }
}

/// Under Reduce Motion the windows snap to their slots instead of travelling there, and rest in
/// the same places: the settled picture is the standard one, byte for byte.
#[test]
fn reduce_motion_settles_to_the_same_picture() {
    let (_, output, dpi) = OUTPUTS[0];
    let standard = open(output, dpi, Some(Motion::Standard));
    let reduced = open(output, dpi, Some(Motion::Reduced));
    assert!(
        standard.as_raw() == reduced.as_raw(),
        "the settled picture does not depend on the motion level"
    );
}
