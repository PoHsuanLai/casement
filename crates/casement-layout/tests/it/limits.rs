//! RustScene under random limits and Spaces: minimums honoured or reported, maximums kept,
//! sizes and positions inside the area, the answer a fixed point of layout, and the same on
//! every call.

use crate::support;

use casement_core::WindowId;
use casement_core::geometry::{LogicalRect, LogicalSize};
use casement_core::window::{MaxSize, SizeLimits};
use casement_layout::{
    Arrangement, ConfigureNeed, LayoutReport, OutputSpec, RustScene, SceneLayout, SceneMode,
    SceneSpec, SpaceArrangement, SpaceSpec, WindowPlacement, WindowSpec, WindowTarget,
};
use proptest::prelude::*;
use support::scene::{app, knobs, output_key, rect, size, space_id};

/// What the generator draws for one window.
#[derive(Debug, Clone)]
struct Draw {
    placement: WindowPlacement,
    limits: SizeLimits,
}

fn limits() -> impl Strategy<Value = SizeLimits> {
    (
        0u32..1600,
        0u32..1000,
        proptest::option::of((0u32..2600, 0u32..1700)),
    )
        .prop_map(|(min_w, min_h, max)| SizeLimits {
            min: size(min_w, min_h),
            max: max.map_or(MaxSize::Unbounded, |(w, h)| {
                MaxSize::Bounded(size(w.max(min_w), h.max(min_h)))
            }),
        })
}

fn placement() -> impl Strategy<Value = WindowPlacement> {
    prop_oneof![
        3 => (-800i32..3000, -800i32..2000, 0u32..2600, 0u32..1700)
            .prop_map(|(x, y, w, h)| WindowPlacement::Floating(rect(x, y, w, h))),
        2 => (0u32..2600, 0u32..1700).prop_map(|(w, h)| WindowPlacement::New(size(w, h))),
        1 => Just(WindowPlacement::Minimised),
    ]
}

fn draw() -> impl Strategy<Value = Draw> {
    (placement(), limits()).prop_map(|(placement, limits)| Draw { placement, limits })
}

/// A scene of `spaces` Spaces on one output, the `shown`th shown, windows dealt round the Spaces.
fn scene(usable: LogicalRect, draws: &[Draw], spaces: usize, shown: usize) -> SceneSpec {
    let id = |n: usize| space_id(&format!("s-{}", n + 1));
    let mut held: Vec<Vec<WindowSpec>> = vec![Vec::new(); spaces];
    for (n, d) in draws.iter().enumerate() {
        held[n % spaces].push(WindowSpec {
            window: WindowId(n as u64 + 1),
            app: app("test"),
            placement: d.placement.clone(),
            limits: d.limits,
            stack: casement_layout::StackIndex(n as u32),
            reserve_top: 0,
        });
    }
    SceneSpec {
        outputs: vec![OutputSpec {
            output: output_key("HEADLESS-1"),
            full: usable,
            usable,
            scale: ds_core::geometry::scale::Scale::ONE,
            mode: SceneMode::Desktop {
                shown: id(shown % spaces),
            },
            spaces: held
                .into_iter()
                .enumerate()
                .map(|(n, windows)| SpaceSpec {
                    space: id(n),
                    kind: SpaceArrangement::Desktop,
                    windows,
                })
                .collect(),
        }],
    }
}

fn target(arrangement: &Arrangement, window: WindowId) -> Option<&WindowTarget> {
    arrangement.windows.iter().find(|t| t.window == window)
}

fn reported(arrangement: &Arrangement, window: WindowId) -> Option<(LogicalSize, LogicalSize)> {
    arrangement.reports.iter().find_map(|r| match r {
        LayoutReport::MinUnmet {
            window: w,
            min,
            got,
        } if *w == window => Some((*min, *got)),
        _ => None,
    })
}

fn asked(placement: &WindowPlacement) -> Option<LogicalSize> {
    match placement {
        WindowPlacement::Floating(r) => Some(r.size),
        WindowPlacement::New(s) => Some(*s),
        _ => None,
    }
}

proptest! {
    #[test]
    fn limits_area_and_reports_agree(
        draws in proptest::collection::vec(draw(), 0..40),
        (x, y, w, h) in (0i32..60, 0i32..60, 320u32..3840, 240u32..2160),
        spaces in 1usize..4,
        shown in 0usize..4,
    ) {
        let usable = rect(x, y, w, h);
        let spec = scene(usable, &draws, spaces, shown);
        let got = RustScene.arrange(&spec, &knobs());
        let in_order = |n: usize| WindowId(n as u64 + 1);
        for (n, d) in draws.iter().enumerate() {
            let window = in_order(n);
            let Some(want) = asked(&d.placement) else {
                prop_assert!(target(&got, window).is_none(), "minimised windows have no target");
                continue;
            };
            let t = target(&got, window).expect("a floating or new window has a target");
            prop_assert_eq!(t.rect.intersection(&usable), Some(t.rect), "inside the area");
            let min = d.limits.min;
            let fits = min.w <= usable.size.w && min.h <= usable.size.h;
            if fits {
                prop_assert!(t.rect.size.w >= min.w && t.rect.size.h >= min.h, "minimum honoured");
                prop_assert_eq!(reported(&got, window), None, "nothing to report");
            } else {
                prop_assert_eq!(reported(&got, window), Some((min, t.rect.size)), "reported with what it got");
            }
            if let MaxSize::Bounded(max) = d.limits.max
                && fits
            {
                prop_assert!(t.rect.size.w <= max.w && t.rect.size.h <= max.h, "maximum honoured");
            }
            let need = if t.rect.size == want { ConfigureNeed::Keep } else { ConfigureNeed::Resize(t.rect.size) };
            prop_assert_eq!(t.size, need, "a resize exactly when the size changed");
        }
        prop_assert_eq!(got.spaces.len(), spaces);
        let shown_space = &got.spaces[shown % spaces];
        prop_assert_eq!(shown_space.origin.x, usable.origin.x);
        prop_assert_eq!(shown_space.origin.y, usable.origin.y);
        prop_assert_eq!(RustScene.arrange(&spec, &knobs()), got);
    }

    #[test]
    fn what_layout_answers_it_would_keep(
        draws in proptest::collection::vec(draw(), 0..40),
        (w, h) in (320u32..3840, 240u32..2160),
    ) {
        let usable = rect(0, 24, w, h);
        let first = RustScene.arrange(&scene(usable, &draws, 1, 0), &knobs());
        let settled = draws
            .iter()
            .enumerate()
            .filter_map(|(n, d)| {
                let t = target(&first, WindowId(n as u64 + 1))?;
                Some(Draw { placement: WindowPlacement::Floating(t.rect), limits: d.limits })
            })
            .collect::<Vec<_>>();
        let again = RustScene.arrange(&scene(usable, &settled, 1, 0), &knobs());
        for (t, u) in first.windows.iter().zip(&again.windows) {
            prop_assert_eq!(t.rect, u.rect, "a placed window stays");
            prop_assert_eq!(u.size, ConfigureNeed::Keep, "and needs no configure");
        }
    }
}
