//! The frame without the cursor: every output's frames drawn a second time with the cursor's
//! elements left out, into a canvas and a damage tracker of their own. A capture that does not
//! ask for the cursor (`overlay_cursor` 0, no `paint_cursors`) reads this one; the frame shown
//! (with the cursor) is the scanout's.

use super::output_set::tracker_of;
use super::renderer::{Canvas, Drawn, HeadlessRenderer};
use super::{FrameSource, VirtualOutputSpec};
use crate::backend::BackendError;
use casement_render::{CasementElement, Compositor};
use smithay::backend::renderer::damage::OutputDamageTracker;
use smithay::backend::renderer::element::{Element, Kind};
use smithay::backend::renderer::gles::GlesRenderer;
use smithay::backend::renderer::pixman::PixmanRenderer;
#[cfg(feature = "testing")]
use smithay::backend::renderer::test::DummyRenderer;

/// One output's cursor-less canvas and tracker.
#[derive(Debug)]
pub(super) struct BareLayer {
    canvas: Canvas,
    tracker: OutputDamageTracker,
}

impl BareLayer {
    /// A layer for `spec`'s mode, scale and transform; none when the renderer could not make
    /// a canvas (the output then has no cursor-less frames).
    pub(super) fn start(
        renderer: &mut dyn HeadlessRenderer,
        spec: &VirtualOutputSpec,
    ) -> Option<BareLayer> {
        match renderer.canvas(spec.mode.size) {
            Ok(canvas) => Some(BareLayer {
                canvas,
                tracker: tracker_of(spec),
            }),
            Err(error) => {
                tracing::warn!(output = spec.name.as_str(), %error, "no canvas for the bare frame");
                None
            }
        }
    }

    /// The image the last cursor-less frame was drawn into.
    pub(super) fn canvas(&self) -> &Canvas {
        &self.canvas
    }

    /// Draws `source` without its cursor; the bytes read back when something changed.
    pub(super) fn draw(
        &mut self,
        renderer: &mut dyn HeadlessRenderer,
        source: &dyn FrameSource,
    ) -> Result<Drawn, BackendError> {
        let age = self.canvas.age();
        let drawn = renderer.draw(
            &mut self.canvas,
            &mut self.tracker,
            &Cursorless(source),
            age,
        )?;
        self.canvas.mark_drawn();
        Ok(drawn)
    }
}

/// A source with the cursor's elements taken out.
pub(crate) struct Cursorless<'a>(pub(crate) &'a dyn FrameSource);

impl FrameSource for Cursorless<'_> {
    fn pixman(&self, renderer: &mut PixmanRenderer) -> Vec<CasementElement<PixmanRenderer>> {
        without_cursor(self.0.pixman(renderer))
    }

    fn gles(&self, renderer: &mut GlesRenderer) -> Vec<CasementElement<GlesRenderer>> {
        without_cursor(self.0.gles(renderer))
    }

    #[cfg(feature = "testing")]
    fn dummy(&self, renderer: &mut DummyRenderer) -> Vec<CasementElement<DummyRenderer>> {
        without_cursor(self.0.dummy(renderer))
    }
}

/// `elements` but the cursor's.
pub(crate) fn without_cursor<R: Compositor>(
    elements: Vec<CasementElement<R>>,
) -> Vec<CasementElement<R>> {
    elements
        .into_iter()
        .filter(|element| !matches!(element.kind(), Kind::Cursor))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use smithay::backend::renderer::Color32F;
    use smithay::backend::renderer::element::Id;
    use smithay::backend::renderer::element::solid::SolidColorRenderElement;
    use smithay::backend::renderer::utils::CommitCounter;
    use smithay::utils::{Physical, Rectangle};

    /// A solid element of `kind`, on pixman's element type (no renderer is needed to hold it).
    fn solid(kind: Kind) -> CasementElement<PixmanRenderer> {
        let rect = Rectangle::<i32, Physical>::new((0, 0).into(), (4, 4).into());
        CasementElement::Solid(SolidColorRenderElement::new(
            Id::new(),
            rect,
            CommitCounter::default(),
            Color32F::new(1.0, 0.0, 0.0, 1.0),
            kind,
        ))
    }

    /// Which of `elements` are the cursor, in order.
    fn cursors(elements: &[CasementElement<PixmanRenderer>]) -> Vec<bool> {
        elements
            .iter()
            .map(|element| matches!(element.kind(), Kind::Cursor))
            .collect()
    }

    #[test]
    fn a_frame_without_the_cursor_keeps_every_other_element_in_order() {
        // name, the element list, what is left (true = a cursor element)
        let table = [
            ("empty", Vec::new(), Vec::new()),
            (
                "no cursor",
                vec![solid(Kind::Unspecified), solid(Kind::Unspecified)],
                vec![false, false],
            ),
            ("only the cursor", vec![solid(Kind::Cursor)], Vec::new()),
            (
                "the cursor on top",
                vec![solid(Kind::Cursor), solid(Kind::Unspecified)],
                vec![false],
            ),
            (
                "two cursor elements between windows",
                vec![
                    solid(Kind::Unspecified),
                    solid(Kind::Cursor),
                    solid(Kind::Cursor),
                    solid(Kind::Unspecified),
                ],
                vec![false, false],
            ),
        ];
        for (name, elements, left) in table {
            let kept = without_cursor(elements);
            assert_eq!(cursors(&kept), left, "{name}");
        }
    }
}
