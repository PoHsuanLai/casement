//! Mapped toplevels that are shot: the helper the surface adjuncts (viewporter, single pixel,
//! alpha modifier) use for their pixman goldens and pixel probes (P0-R-scene's `shoot`).

use casement_core::WindowId;
use casement_core::geometry::LogicalPoint;
use casement_core::input::DeviceClass;
use casement_core::policy::ConnectionRole;
use casement_render::golden::{Golden, GoldenOutcome, GoldenTier, compare};
use casement_testclient::TestClient;
use casement_testclient::surface::subsurface::Surface;
use casement_testclient::surface::toplevel::{Toplevel, WmBase};
use casement_testkit::{ClientId, Env, Fixture, FixtureSpec, ShotTarget, TestOrigin};
use std::time::Duration;

/// How long a window's opening motion takes to settle, with room to spare.
const SETTLE: Duration = Duration::from_secs(2);

/// A desktop with one client and its `xdg_wm_base`.
pub(crate) fn desk() -> (Fixture, ClientId, WmBase) {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    // The pointer starts at the output's centre, over the windows these tests shoot: the shots
    // are of the window, not of the cursor, so it goes to the corner.
    let device = fixture.input().plug(
        DeviceClass::Pointer,
        TestOrigin::Physical(DeviceClass::Pointer),
    );
    fixture
        .input()
        .pointer_to(device, LogicalPoint { x: 0, y: 0 });
    let id = fixture.connect(ConnectionRole::Public);
    fixture.pump();
    let base = WmBase::bind(fixture.client(id), 6);
    (fixture, id, base)
}

/// `top`'s surface as the adjunct helpers take it.
pub(crate) fn surface_of(top: &Toplevel) -> Surface {
    Surface {
        surface: top.surface.clone(),
    }
}

/// A toplevel that maps: created, committed bare, acked, then `attach` gives it a buffer (and
/// whatever adjunct objects the test is about; what it returns is kept for the test) and the
/// commit that maps it is made; the opening motion is run out.
pub(crate) fn map_with<K>(
    fixture: &mut Fixture,
    id: ClientId,
    base: &WmBase,
    attach: impl FnOnce(&mut TestClient, &Toplevel) -> K,
) -> (Toplevel, K) {
    let top = base.toplevel(fixture.client(id));
    top.set_app_id("org.example.shoot");
    top.commit_bare();
    fixture.pump();
    assert!(top.ack_latest().is_some(), "the initial configure arrives");
    let kept = attach(fixture.client(id), &top);
    top.surface.damage_buffer(0, 0, i32::MAX, i32::MAX);
    top.surface.commit();
    fixture.pump();
    fixture.advance(SETTLE);
    (top, kept)
}

/// The windows the model holds, oldest first.
pub(crate) fn windows(fixture: &Fixture) -> Vec<WindowId> {
    fixture.model().windows().keys().copied().collect()
}

/// A shot of `window` in the last frame.
pub(crate) fn shot_of(fixture: &mut Fixture, window: WindowId) -> image::RgbaImage {
    fixture.shot(ShotTarget::Window(window))
}

/// Compares `actual` with `tests/golden/scenes/<name>.png` byte for byte (pixman, tier P);
/// `CASEMENT_BLESS=1` writes it (CONVENTIONS addendum 12).
pub(crate) fn check_pixels(name: &str, actual: &image::RgbaImage) {
    let env = Env::of_process();
    let golden = Golden {
        scene: format!("tests/golden/scenes/{name}"),
        tier: GoldenTier::Exact,
    };
    let outcome = compare(&golden, actual, &env.out_dir, env.bless);
    assert!(
        matches!(outcome, GoldenOutcome::Match | GoldenOutcome::Blessed),
        "{name}: {outcome:?}"
    );
}
