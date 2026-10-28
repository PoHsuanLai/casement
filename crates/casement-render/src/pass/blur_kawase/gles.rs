//! The dual-Kawase passes on GLES, every call through smithay's safe API.
//!
//! [`capture`] runs when smithay says the framebuffer under the blur changed. It blits the
//! rectangle under the element, half the size on each side with linear filtering, into the first
//! level of a chain of textures kept in the element's cache; then draws each smaller level from
//! the one above (the downsample shader) and back up to the first (the upsample shader), each
//! draw in a frame of its own on that level's texture, nested inside the output's frame through
//! the frame guard (which restores the output's target, viewport and scissor when it drops).
//! [`finish`] draws the first level stretched over the output through the upsample shader again,
//! with the saturation, tint and grain, so a frame the blur is damaged in but nothing under it
//! changed costs one draw, and a still scene costs none.

use super::plan::{KawasePlan, level_size};
use crate::element::Rgba8;
use smithay::backend::allocator::Fourcc;
use smithay::backend::renderer::gles::{
    GlesError, GlesFrame, GlesRenderer, GlesTexProgram, GlesTexture, Uniform, UniformName,
    UniformType,
};
use smithay::backend::renderer::{
    Bind, BlitFrame, Color32F, Frame, FrameContext, Offscreen, Renderer, Texture, TextureFilter,
};
use smithay::utils::user_data::UserDataMap;
use smithay::utils::{Buffer as BufferCoords, Physical, Rectangle, Size, Transform};
use std::cell::RefCell;

/// The downsample shader (`src/shaders/kawase_down.frag`).
const DOWN: &str = include_str!("../../shaders/kawase_down.frag");

/// The upsample and finish shader (`src/shaders/kawase_up.frag`).
const UP: &str = include_str!("../../shaders/kawase_up.frag");

/// The two compiled shaders, kept in the EGL context's user data, one pair per renderer.
#[derive(Debug, Default)]
struct Compiled(RefCell<Option<Programs>>);

/// The compiled shaders.
#[derive(Debug, Clone)]
struct Programs {
    /// Halves a level.
    down: GlesTexProgram,
    /// Doubles a level, and finishes.
    up: GlesTexProgram,
}

/// What one blur element keeps between frames, in its damage-tracker cache.
#[derive(Debug, Default)]
struct Chain {
    /// The blurred copy, once there is one.
    shown: Option<Shown>,
}

/// A blurred copy of the framebuffer under an element.
#[derive(Debug, Clone)]
struct Shown {
    /// The levels, the first (half size) holding the blur.
    levels: Vec<GlesTexture>,
    /// The output rectangle the copy shows (the element's rectangle cut to the output).
    from: Rectangle<i32, Physical>,
}

/// The saturation, tint and grain of a pass: the neutral ones for the passes in between.
#[derive(Debug, Clone, Copy)]
struct Finish {
    saturation: f32,
    wash: Rgba8,
    grain: f32,
}

impl Finish {
    /// Leaves the colour alone.
    const NEUTRAL: Finish = Finish {
        saturation: 1.0,
        wash: Rgba8 {
            r: 0,
            g: 0,
            b: 0,
            a: 0,
        },
        grain: 0.0,
    };

    /// What `plan` asks of the last pass.
    fn of(plan: &KawasePlan) -> Finish {
        Finish {
            saturation: plan.saturation,
            wash: plan.tint,
            grain: plan.noise,
        }
    }
}

/// Blurs what lies under `dst` (the element on the output) as `plan` says, into the cache.
pub(crate) fn capture(
    frame: &mut GlesFrame<'_, '_>,
    dst: Rectangle<i32, Physical>,
    plan: &KawasePlan,
    cache: &UserDataMap,
) -> Result<(), GlesError> {
    cache.insert_if_missing(|| RefCell::new(Chain::default()));
    let Some(cell) = cache.get::<RefCell<Chain>>() else {
        return Ok(());
    };
    let Some(from) = dst.intersection(Rectangle::from_size(frame.output_size())) else {
        cell.borrow_mut().shown = None;
        return Ok(());
    };
    let sizes: Vec<Size<i32, BufferCoords>> = (1..=plan.passes.max(1))
        .map(|level| level_size((from.size.w, from.size.h), level).into())
        .collect();
    let kept = cell
        .borrow()
        .shown
        .as_ref()
        .map(|shown| shown.levels.clone())
        .filter(|levels| levels.iter().map(Texture::size).eq(sizes.iter().copied()));
    let mut levels = match kept {
        Some(levels) => levels,
        None => on_renderer(frame, |renderer| {
            sizes
                .iter()
                .map(|size| {
                    Offscreen::<GlesTexture>::create_buffer(renderer, Fourcc::Abgr8888, *size)
                })
                .collect()
        })?,
    };
    let programs = programs(frame)?;
    blit_first(frame, from, &mut levels)?;
    on_renderer(frame, |renderer| {
        run_levels(renderer, &mut levels, &programs, plan)
    })?;
    cell.borrow_mut().shown = Some(Shown { levels, from });
    Ok(())
}

/// Draws the blurred copy over the output at `dst` where `damage` (relative to `dst`, already cut
/// to the region) asks, at `alpha`. True when the blurred copy was drawn; false when nothing was
/// captured yet and the caller draws the stand-in.
pub(crate) fn finish(
    frame: &mut GlesFrame<'_, '_>,
    plan: &KawasePlan,
    alpha: f32,
    (dst, damage): (Rectangle<i32, Physical>, &[Rectangle<i32, Physical>]),
    cache: Option<&UserDataMap>,
) -> Result<bool, GlesError> {
    let shown = cache
        .and_then(|cache| cache.get::<RefCell<Chain>>())
        .and_then(|cell| cell.borrow().shown.clone());
    let Some(Shown { levels, from }) = shown else {
        return Ok(false);
    };
    let Some(first) = levels.first() else {
        return Ok(false);
    };
    let program = programs(frame)?.up;
    // The damage, moved from the element's origin to the copy's.
    let shift = dst.loc - from.loc;
    let local: Vec<_> = damage
        .iter()
        .filter_map(|rect| {
            Rectangle::new(rect.loc + shift, rect.size)
                .intersection(Rectangle::from_size(from.size))
        })
        .collect();
    if local.is_empty() {
        return Ok(true);
    }
    let size = first.size();
    frame.render_texture_from_to(
        first,
        Rectangle::from_size(size).to_f64(),
        from,
        &local,
        &[],
        Transform::Normal,
        alpha,
        Some(&program),
        &up_uniforms(size, plan.offset, Finish::of(plan)),
    )?;
    Ok(true)
}

/// Copies `from` (output pixels) of the frame into the first level, linearly filtered.
fn blit_first(
    frame: &mut GlesFrame<'_, '_>,
    from: Rectangle<i32, Physical>,
    levels: &mut [GlesTexture],
) -> Result<(), GlesError> {
    let Some(first) = levels.first_mut() else {
        return Ok(());
    };
    let size = first.size();
    let mut target = on_renderer(frame, |renderer| renderer.bind(first))?;
    let to = Rectangle::from_size((size.w, size.h).into());
    let copied = frame.blit_to(&mut target, from, to, TextureFilter::Linear)?;
    frame.wait(&copied)
}

/// Down through the levels, then back up to the first; the first level ends up blurred.
fn run_levels(
    renderer: &mut GlesRenderer,
    levels: &mut [GlesTexture],
    programs: &Programs,
    plan: &KawasePlan,
) -> Result<(), GlesError> {
    for at in 1..levels.len() {
        let source = levels[at - 1].clone();
        let uniforms = [
            Uniform::new("texel", texel(source.size())),
            Uniform::new("offset", plan.offset),
        ];
        pass(
            renderer,
            &source,
            &mut levels[at],
            &programs.down,
            &uniforms,
        )?;
    }
    for at in (0..levels.len().saturating_sub(1)).rev() {
        let source = levels[at + 1].clone();
        let uniforms = up_uniforms(source.size(), plan.offset, Finish::NEUTRAL);
        pass(renderer, &source, &mut levels[at], &programs.up, &uniforms)?;
    }
    Ok(())
}

/// Draws all of `source` over all of `target` through `program`, replacing what was there.
fn pass(
    renderer: &mut GlesRenderer,
    source: &GlesTexture,
    target: &mut GlesTexture,
    program: &GlesTexProgram,
    uniforms: &[Uniform<'_>],
) -> Result<(), GlesError> {
    let size = target.size();
    let area = Rectangle::<i32, Physical>::from_size((size.w, size.h).into());
    let mut bound = renderer.bind(target)?;
    let mut nested = renderer.render(&mut bound, area.size, Transform::Normal)?;
    nested.clear(Color32F::TRANSPARENT, &[area])?;
    nested.render_texture_from_to(
        source,
        Rectangle::from_size(source.size()).to_f64(),
        area,
        &[area],
        &[],
        Transform::Normal,
        1.0,
        Some(program),
        uniforms,
    )?;
    // One GL context draws every level and then samples it, and GL keeps that order, so the
    // level's fence need not be waited on here.
    let _ordered_by_the_context = nested.finish()?;
    Ok(())
}

/// The upsample shader's uniforms for a source of `size`.
fn up_uniforms(
    size: Size<i32, BufferCoords>,
    offset: f32,
    finish: Finish,
) -> [Uniform<'static>; 5] {
    let unit = |n: u8| f32::from(n) / 255.0;
    let wash = finish.wash;
    [
        Uniform::new("texel", texel(size)),
        Uniform::new("offset", offset),
        Uniform::new("saturation", finish.saturation),
        Uniform::new(
            "wash",
            (unit(wash.r), unit(wash.g), unit(wash.b), unit(wash.a)),
        ),
        Uniform::new("grain", finish.grain),
    ]
}

/// One texel of a texture of `size`, in texture coordinates.
fn texel(size: Size<i32, BufferCoords>) -> (f32, f32) {
    let side = |n: i32| 1.0 / f32::from(u16::try_from(n.max(1)).unwrap_or(u16::MAX));
    (side(size.w), side(size.h))
}

/// Runs `run` on the renderer behind `frame`; the output's target is restored afterwards.
fn on_renderer<T>(
    frame: &mut GlesFrame<'_, '_>,
    run: impl FnOnce(&mut GlesRenderer) -> Result<T, GlesError>,
) -> Result<T, GlesError> {
    let mut guard = frame.renderer();
    let renderer: &mut GlesRenderer = guard.as_mut();
    run(renderer)
}

/// The shaders, compiled on first use for this renderer's context.
fn programs(frame: &mut GlesFrame<'_, '_>) -> Result<Programs, GlesError> {
    on_renderer(frame, |renderer| {
        renderer
            .egl_context()
            .user_data()
            .insert_if_missing(Compiled::default);
        let known = renderer
            .egl_context()
            .user_data()
            .get::<Compiled>()
            .and_then(|compiled| compiled.0.borrow().clone());
        if let Some(programs) = known {
            return Ok(programs);
        }
        let texel = UniformName::new("texel", UniformType::_2f);
        let offset = UniformName::new("offset", UniformType::_1f);
        let programs = Programs {
            down: renderer.compile_custom_texture_shader(DOWN, &[texel.clone(), offset.clone()])?,
            up: renderer.compile_custom_texture_shader(
                UP,
                &[
                    texel,
                    offset,
                    UniformName::new("saturation", UniformType::_1f),
                    UniformName::new("wash", UniformType::_4f),
                    UniformName::new("grain", UniformType::_1f),
                ],
            )?,
        };
        if let Some(compiled) = renderer.egl_context().user_data().get::<Compiled>() {
            *compiled.0.borrow_mut() = Some(programs.clone());
        }
        Ok(programs)
    })
}
