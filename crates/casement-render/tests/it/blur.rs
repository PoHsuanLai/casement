//! The dual-Kawase blur's numbers and its element's geometry: pure, no renderer. The pixels are
//! the goldens in `scenes.rs`.

use casement_core::geometry::PhysicalRect;
use casement_render::pass::{BlurLook, KawasePlan, level_size};
use casement_render::{BlurV0Element, Rgba8};
use smithay::backend::renderer::element::{Element, Id};
use smithay::utils::Scale;

const STAND_IN: Rgba8 = Rgba8 {
    r: 208,
    g: 208,
    b: 208,
    a: 217,
};

fn look(sigma: f32) -> BlurLook {
    BlurLook {
        sigma,
        ..BlurLook::MATERIAL
    }
}

/// What `plan` must hold for `sigma` (physical pixels, at scale 1): the sigma delivered, and no
/// more levels at half the scale.
fn assert_delivers(sigma: f32) -> KawasePlan {
    let plan = look(sigma).plan(1.0);
    assert!(
        (plan.sigma() - sigma).abs() < sigma * 1e-3,
        "sigma {sigma}: got {}",
        plan.sigma()
    );
    let one_fewer = look(sigma).plan(0.5);
    assert!(
        one_fewer.passes <= plan.passes,
        "a smaller output needs no more levels"
    );
    plan
}

#[test]
fn the_fewest_levels_that_keep_the_taps_close_deliver_the_sigma() {
    for sigma in [4.0_f32, 10.0, 22.0, 44.0, 90.0] {
        let plan = assert_delivers(sigma);
        assert!(plan.offset <= 2.5, "sigma {sigma}: offset {}", plan.offset);
    }
    assert_eq!(look(5.0).plan(1.0).passes, 2);
    assert_eq!(look(22.0).plan(2.0).passes, 5);
}

/// The deepest level may open the taps past 2.5 (to 4) before the sigma is capped: sigma 180 is
/// beyond what six levels deliver at 2.5, and is still delivered.
#[test]
fn the_deepest_level_opens_its_taps_for_a_sigma_it_would_otherwise_fall_short_of() {
    let plan = assert_delivers(180.0);
    assert_eq!(plan.passes, 6);
    assert!(
        plan.offset > 2.5 && plan.offset <= 4.0,
        "offset {}",
        plan.offset
    );
}

#[test]
fn a_sigma_beyond_the_deepest_level_is_capped() {
    let plan = look(5000.0).plan(1.0);
    assert_eq!(plan.passes, 6);
    assert_eq!(plan.offset, 4.0);
}

#[test]
fn no_blur_is_one_level_with_no_spread() {
    let plan = look(0.0).plan(1.0);
    assert_eq!((plan.passes, plan.offset), (1, 0.0));
    assert_eq!(plan.reach(), 1);
}

#[test]
fn a_look_keeps_its_saturation_and_clamps_its_grain() {
    let plan = BlurLook {
        noise: 3.0,
        ..BlurLook::MATERIAL
    }
    .plan(1.0);
    assert_eq!(plan.saturation, 1.8);
    assert_eq!(plan.noise, 1.0);
}

#[test]
fn levels_halve_and_round_up_to_at_least_a_pixel() {
    assert_eq!(level_size((100, 51), 1), (50, 26));
    assert_eq!(level_size((100, 51), 2), (25, 13));
    assert_eq!(level_size((3, 1), 4), (1, 1));
    assert_eq!(level_size((0, 0), 1), (1, 1));
}

#[test]
fn damage_is_expanded_by_the_reach_on_every_side() {
    let plan = look(10.0).plan(1.0);
    assert_eq!(plan.reach(), 31);
    let grown = plan.expand(smithay::utils::Rectangle::new(
        (50, 60).into(),
        (100, 40).into(),
    ));
    assert_eq!(
        (grown.loc.x, grown.loc.y, grown.size.w, grown.size.h),
        (19, 29, 162, 102)
    );
}

#[test]
fn the_element_covers_its_region_and_the_reach_but_draws_only_the_region() {
    let rect = |x, y, w, h| PhysicalRect { x, y, w, h };
    let blur = BlurV0Element::new(
        Id::new(),
        vec![rect(100, 100, 40, 20), rect(100, 140, 40, 20)],
        Rgba8 {
            r: 255,
            g: 255,
            b: 255,
            a: 255,
        },
    )
    .with_look(look(10.0), 1.0);
    let reach = blur.plan().reach();
    let geometry = blur.geometry(Scale::from(1.0));
    assert_eq!((geometry.loc.x, geometry.loc.y), (100 - reach, 100 - reach));
    assert_eq!(
        (geometry.size.w, geometry.size.h),
        (40 + 2 * reach, 60 + 2 * reach)
    );
    // Only the region is opaque (the stand-in here is), and all of it.
    let opaque = blur.opaque_regions(Scale::from(1.0));
    assert!(opaque.iter().all(|r| r.loc.x >= reach && r.loc.y >= reach));
    assert_eq!(
        opaque.iter().map(|r| r.size.w * r.size.h).sum::<i32>(),
        40 * 20 * 2
    );
}

#[test]
fn a_new_look_is_a_new_commit() {
    let blur = BlurV0Element::new(
        Id::new(),
        vec![PhysicalRect {
            x: 0,
            y: 0,
            w: 10,
            h: 10,
        }],
        STAND_IN,
    );
    let softer = blur.clone().with_look(look(40.0), 1.0);
    assert!(blur.current_commit() != softer.current_commit());
}

#[test]
fn the_shaders_declare_what_the_passes_set() {
    let down = include_str!("../../src/shaders/kawase_down.frag");
    let up = include_str!("../../src/shaders/kawase_up.frag");
    for source in [down, up] {
        assert!(source.contains("//_DEFINES_"), "smithay's defines marker");
        assert!(source.contains("varying vec2 v_coords;"));
        assert!(source.contains("uniform float alpha;"));
    }
    assert!(down.contains("uniform vec2 texel;") && down.contains("uniform float offset;"));
    for declaration in [
        "uniform vec2 texel;",
        "uniform float offset;",
        "uniform float saturation;",
        "uniform vec4 wash;",
        "uniform float grain;",
    ] {
        assert!(up.contains(declaration), "{declaration}");
    }
}

/// llvmpipe's `sin` returns NaN past an argument of 65536, which the classic
/// `sin(dot(gl_FragCoord.xy, k))` grain hash reaches at full-HD coordinates; the NaN turned the
/// bottom right of a 1920 by 1080 blur black even at a grain of 0.
#[test]
fn the_finish_shader_has_no_sin_of_the_pixel_position() {
    let up = include_str!("../../src/shaders/kawase_up.frag");
    assert!(!up.contains("sin("), "no sin() in the finish shader");
    assert!(up.contains("if (grain > 0.0)"), "grain only when asked");
}
