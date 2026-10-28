//! `arrange` timing for 50 windows (the plan's budget: p95 at most 200 us), outside `crates/`
//! because boundary row B8 keeps `Instant::now` there. `layout [ITERATIONS] [--tsv]`.
use casement_core::app::AppKey;
use casement_core::geometry::{LogicalPoint, LogicalRect, LogicalSize};
use casement_core::output::{EdidFacts, EdidSerial};
use casement_core::window::SizeLimits;
use casement_core::{OutputKey, OutputName, WindowId};
use casement_layout::*;
use casement_perf::{Options, time_row};
use ds_core::geometry::scale::Scale;
use ds_core::vocab::Fraction;
use porter_core::{ClaimedId, SpaceId};

fn rect(x: i32, y: i32, w: u32, h: u32) -> LogicalRect {
    LogicalRect::new(LogicalPoint { x, y }, LogicalSize { w, h })
}

struct Lcg(u64);
impl Lcg {
    fn next(&mut self, lo: u32, hi: u32) -> u32 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        lo + ((self.0 >> 33) as u32) % (hi - lo + 1)
    }
}

fn window(n: u64, placement: WindowPlacement) -> WindowSpec {
    WindowSpec {
        window: WindowId(n),
        app: AppKey::Claimed(ClaimedId::parse("bench").unwrap()),
        placement,
        limits: SizeLimits {
            min: LogicalSize { w: 200, h: 120 },
            max: None,
        },
        stack: StackIndex(n as u32),
    }
}

fn scene(spaces: usize, per_space: usize, mix: bool) -> SceneSpec {
    let edid = EdidFacts {
        make: String::new(),
        model: String::new(),
        serial: EdidSerial::Unknown,
    };
    let key = OutputKey::of(&edid, &OutputName::parse("HEADLESS-1").unwrap());
    let mut rng = Lcg(7);
    let mut n = 0;
    let spaces_spec = (0..spaces)
        .map(|s| SpaceSpec {
            space: SpaceId::parse(&format!("s-{s}")).unwrap(),
            kind: SpaceArrangement::Desktop,
            windows: (0..per_space)
                .map(|i| {
                    n += 1;
                    let (w, h) = (rng.next(320, 1000), rng.next(220, 720));
                    let placement = match (mix, i % 5) {
                        (true, 3) => WindowPlacement::New(LogicalSize { w, h }),
                        (true, 4) => WindowPlacement::Minimised,
                        _ => WindowPlacement::Floating(rect(
                            rng.next(0, 1500) as i32,
                            rng.next(0, 900) as i32,
                            w,
                            h,
                        )),
                    };
                    window(n, placement)
                })
                .collect(),
        })
        .collect();
    SceneSpec {
        outputs: vec![OutputSpec {
            output: key,
            full: rect(0, 0, 1920, 1080),
            usable: rect(0, 24, 1920, 1056),
            scale: Scale::ONE,
            mode: SceneMode::Desktop {
                shown: SpaceId::parse("s-0").unwrap(),
            },
            spaces: spaces_spec,
        }],
    }
}

fn run(name: &str, spec: &SceneSpec, options: Options) {
    let knobs = LayoutKnobs {
        cascade_step: LogicalPoint { x: 28, y: 28 },
        tile_margin: 8,
        spread_gap: 20,
        spread_floor: Fraction(150),
    };
    let windows: usize = spec.outputs[0].spaces.iter().map(|s| s.windows.len()).sum();
    time_row(
        &format!("arrange: {name} ({windows} windows)"),
        options,
        500,
        |_| {
            std::hint::black_box(RustScene.arrange(std::hint::black_box(spec), &knobs));
        },
    );
}

fn main() {
    let options = Options::from_args();
    run("1 Space, 50 floating", &scene(1, 50, false), options);
    run("3 Spaces, 50 floating", &scene(3, 17, false), options);
    run("1 Space, 50 mixed", &scene(1, 50, true), options);
    run("5 Spaces, 50 mixed", &scene(5, 10, true), options);
}
