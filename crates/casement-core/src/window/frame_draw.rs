//! What the renderer needs to draw a frame, resolved: colours, marks and shadow as plain data.
//! `casement-layout` makes it from quire's tokens; `casement-render` draws it. Nothing here
//! knows a token or a renderer.

use super::frame::{FrameGeometry, Light};

/// A colour with straight (not premultiplied) alpha.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FrameColour {
    /// Red.
    pub r: u8,
    /// Green.
    pub g: u8,
    /// Blue.
    pub b: u8,
    /// Alpha, 255 opaque.
    pub a: u8,
}

/// What one light looks like.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LightDraw {
    /// The disc.
    pub fill: FrameColour,
    /// The hairline round it.
    pub ring: FrameColour,
    /// The ink of the mark (the cross, the bar, the arrows) when it shows.
    pub mark: Option<FrameColour>,
}

/// One layer of the window's shadow: a blurred copy of the outline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ShadowLayer {
    /// How far down it falls, in logical pixels.
    pub dy: i32,
    /// The blur radius, in logical pixels.
    pub blur: u32,
    /// How far it grows (negative: shrinks), in logical pixels.
    pub spread: i32,
    /// Its colour.
    pub colour: FrameColour,
}

/// A frame ready to draw.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameDraw {
    /// Where its parts are.
    pub geometry: FrameGeometry,
    /// The window's corner radius, in logical pixels.
    pub radius: u32,
    /// The titlebar's ground.
    pub titlebar: FrameColour,
    /// The hairline between the titlebar and the content.
    pub rule: FrameColour,
    /// The three lights, left to right.
    pub lights: [LightDraw; 3],
    /// The shadow, widest last.
    pub shadow: [ShadowLayer; 2],
}

impl FrameDraw {
    /// `light`'s look.
    pub fn light(&self, light: Light) -> LightDraw {
        match light {
            Light::Close => self.lights[0],
            Light::Minimise => self.lights[1],
            Light::Zoom => self.lights[2],
        }
    }
}
