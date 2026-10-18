//! The blur's numbers: what a material asks for and the pass count and offset that deliver it.
//! Pure; nothing here touches a renderer.

use crate::element::Rgba8;
use casement_core::layer::BlurStrength;
use smithay::utils::{Physical, Rectangle};

/// The most halving levels (a 4K output is 60 pixels across at level six).
pub const MAX_PASSES: u8 = 6;

/// The tap offset past which a level's taps leave gaps: more levels are used instead.
const MAX_OFFSET: f32 = 2.5;

/// The offset a plan with [`MAX_PASSES`] may use when even that many levels fall short.
const LAST_RESORT_OFFSET: f32 = 4.0;

/// How far one unit of `offset * 2^passes` spreads, as a Gaussian's sigma. UNMEASURED: set from
/// the filter's variance by hand; the golden at 1x and 2x pins what it gives, and design/23 M26
/// (10 to 90 percent over 56 logical pixels for sigma 22) is the figure to calibrate against.
const SIGMA_PER_STEP: f32 = 0.8;

/// How many sigmas of reach the blur reads beyond its region.
const REACH_SIGMAS: f32 = 3.0;

/// What a material asks of the blur behind it, in logical pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BlurLook {
    /// The sigma of the Gaussian the blur should look like, in logical pixels.
    pub sigma: f32,
    /// The saturation of what shows through (1 leaves it, 1.8 is the widget card's, design/23 M29).
    pub saturation: f32,
    /// A premultiplied colour mixed over the blur (alpha 0: none).
    pub tint: Rgba8,
    /// The amplitude of grain added over the blur, 0 to 1 (0: none).
    pub noise: f32,
}

impl BlurLook {
    /// The compositor's one material when a client asks for blur and says no more: sigma 22
    /// logical pixels and saturation 1.8 (design/23 M26 and M29), no tint, no grain.
    pub const MATERIAL: BlurLook = BlurLook {
        sigma: 22.0,
        saturation: 1.8,
        tint: Rgba8 {
            r: 0,
            g: 0,
            b: 0,
            a: 0,
        },
        noise: 0.0,
    };

    /// The look of a small surface that should keep the picture behind it readable: sigma 12,
    /// saturation 1.6. UNMEASURED: a judgement beside [`BlurLook::MATERIAL`].
    pub const LIGHT: BlurLook = BlurLook {
        sigma: 12.0,
        saturation: 1.6,
        ..BlurLook::MATERIAL
    };

    /// The look of a large panel that should stand clear of a busy picture: sigma 32, saturation
    /// 1.8. UNMEASURED: a judgement beside [`BlurLook::MATERIAL`].
    pub const STRONG: BlurLook = BlurLook {
        sigma: 32.0,
        ..BlurLook::MATERIAL
    };

    /// The look a surface of this `strength` asks for; `Regular` is [`BlurLook::MATERIAL`].
    ///
    /// ```
    /// use casement_core::layer::BlurStrength;
    /// use casement_render::pass::BlurLook;
    ///
    /// assert_eq!(BlurLook::of(BlurStrength::Regular), BlurLook::MATERIAL);
    /// ```
    pub fn of(strength: BlurStrength) -> BlurLook {
        match strength {
            BlurStrength::Light => BlurLook::LIGHT,
            BlurStrength::Regular => BlurLook::MATERIAL,
            BlurStrength::Strong => BlurLook::STRONG,
        }
    }

    /// The passes this look needs on an output at `scale` (physical pixels per logical pixel).
    ///
    /// ```
    /// use casement_render::pass::BlurLook;
    ///
    /// let plan = BlurLook::MATERIAL.plan(2.0);
    /// assert_eq!(plan.passes, 5);
    /// assert!(plan.offset > 0.0 && plan.offset <= 2.5);
    /// ```
    pub fn plan(self, scale: f64) -> KawasePlan {
        let sigma = self.sigma.max(0.0) * scale_f32(scale);
        let (passes, offset) = passes_and_offset(sigma);
        KawasePlan {
            passes,
            offset,
            saturation: self.saturation.max(0.0),
            tint: self.tint,
            noise: self.noise.clamp(0.0, 1.0),
        }
    }
}

/// A blur in physical pixels: the levels, the tap offset and the finish.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KawasePlan {
    /// Halving levels, 1 to [`MAX_PASSES`]. Level one is the framebuffer copy itself.
    pub passes: u8,
    /// The tap offset in the source level's texels.
    pub offset: f32,
    /// The saturation of what shows through.
    pub saturation: f32,
    /// The tint over the blur, premultiplied.
    pub tint: Rgba8,
    /// The grain amplitude.
    pub noise: f32,
}

impl KawasePlan {
    /// The sigma (physical pixels) this plan blurs by.
    pub fn sigma(&self) -> f32 {
        SIGMA_PER_STEP * self.offset * level_scale(self.passes)
    }

    /// How far beyond its region the blur reads, in physical pixels: what the element's
    /// geometry grows by, so damage that close to the region captures the blur again.
    ///
    /// ```
    /// use casement_render::pass::BlurLook;
    ///
    /// let look = BlurLook { sigma: 10.0, ..BlurLook::MATERIAL };
    /// assert_eq!(look.plan(1.0).reach(), 31);
    /// ```
    pub fn reach(&self) -> i32 {
        let sigmas = (REACH_SIGMAS * self.sigma()).round();
        // The cast is exact: the value is a small whole number (sigma is bounded by the plan).
        sigmas as i32 + 1
    }

    /// `rect` grown by [`KawasePlan::reach`] on every side.
    pub fn expand(&self, rect: Rectangle<i32, Physical>) -> Rectangle<i32, Physical> {
        let reach = self.reach();
        Rectangle::new(
            (
                rect.loc.x.saturating_sub(reach),
                rect.loc.y.saturating_sub(reach),
            )
                .into(),
            (
                rect.size.w.saturating_add(reach.saturating_mul(2)),
                rect.size.h.saturating_add(reach.saturating_mul(2)),
            )
                .into(),
        )
    }
}

/// The fewest levels, and their tap offset, that blur by `sigma` physical pixels.
pub(crate) fn passes_and_offset(sigma: f32) -> (u8, f32) {
    let offset_at = |passes: u8| sigma / (SIGMA_PER_STEP * level_scale(passes));
    (1..=MAX_PASSES)
        .map(|passes| (passes, offset_at(passes)))
        .find(|(_, offset)| *offset <= MAX_OFFSET)
        .unwrap_or_else(|| (MAX_PASSES, offset_at(MAX_PASSES).min(LAST_RESORT_OFFSET)))
}

/// A level's size: `size` halved `level` times, rounded up, at least one pixel.
///
/// ```
/// # use casement_render::pass::level_size;
/// assert_eq!(level_size((100, 51), 2), (25, 13));
/// assert_eq!(level_size((3, 1), 4), (1, 1));
/// ```
pub fn level_size((w, h): (i32, i32), level: u8) -> (i32, i32) {
    let side = |n: i32| {
        n.max(1)
            .saturating_add((1_i32 << level) - 1)
            .div_euclid(1_i32 << level)
            .max(1)
    };
    (side(w), side(h))
}

/// `2^passes` as a float.
fn level_scale(passes: u8) -> f32 {
    f32::from(1_u16 << passes.min(MAX_PASSES))
}

/// An output scale as a float.
fn scale_f32(scale: f64) -> f32 {
    // The cast narrows a scale (between 1 and 8 in practice) to the shader's precision.
    scale as f32
}
