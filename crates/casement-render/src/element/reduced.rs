//! Reduce transparency (`accessibility.reduce_transparency`): the frame's blurs drawn as their
//! solid stand-ins.

use super::CasementElement;
use crate::compositor::Compositor;
use casement_core::settings::Transparency;

/// `elements` with every blur under `transparency`: unchanged at [`Transparency::Full`]; at
/// [`Transparency::Reduced`] each blur becomes its mostly opaque stand-in (on any renderer, and
/// with no framebuffer capture), so the setting takes effect on the next frame.
pub fn under_transparency<R: Compositor>(
    elements: Vec<CasementElement<R>>,
    transparency: Transparency,
) -> Vec<CasementElement<R>> {
    match transparency {
        Transparency::Full => elements,
        Transparency::Reduced => elements
            .into_iter()
            .map(|element| match element {
                CasementElement::Blur(blur) => {
                    CasementElement::Blur(blur.with_transparency(transparency))
                }
                other => other,
            })
            .collect(),
    }
}
