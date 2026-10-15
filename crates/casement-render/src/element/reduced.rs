//! Reduce transparency (quire's `appearance.transparency`): the frame's blurs drawn as their
//! solid stand-ins.

use super::{CasementElement, Rgba8};
use crate::compositor::Compositor;

/// How the frame's blurs are finished.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum BlurFinish {
    /// A surface that asked for blur behind it gets the blur.
    #[default]
    Frosted,
    /// No blur: the surface gets this mostly opaque colour (premultiplied) in its place.
    Solid(Rgba8),
}

/// `elements` with every blur finished as `finish`: unchanged when [`BlurFinish::Frosted`]; when
/// [`BlurFinish::Solid`] each blur becomes that colour (on any renderer, and with no framebuffer
/// capture), so the setting takes effect on the next frame.
pub fn under_finish<R: Compositor>(
    elements: Vec<CasementElement<R>>,
    finish: BlurFinish,
) -> Vec<CasementElement<R>> {
    match finish {
        BlurFinish::Frosted => elements,
        BlurFinish::Solid(_) => elements
            .into_iter()
            .map(|element| match element {
                CasementElement::Blur(blur) => CasementElement::Blur(blur.with_finish(finish)),
                other => other,
            })
            .collect(),
    }
}
