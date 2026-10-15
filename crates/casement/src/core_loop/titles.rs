//! The titles of the server-side frames: which windows have one to draw, with what ink, and the
//! pixels `casement-chrome` makes for them. The raster is the titlebar's size at the output's
//! native scale; `casement-render` lays it over the titlebar.

use super::frames::activity_of;
use crate::state::{RenderState, State};
use casement_chrome::{Margins, TitleKey, TitleRaster};
use casement_core::WindowId;
use casement_core::window::{FrameMetrics, FrameStyle};
use casement_layout::frame_metrics;
use casement_motion::OutputFrame;
use casement_render::{OutputView, TitleDraw, window_drawn};
use std::collections::BTreeMap;

/// What the title keeps clear in a titlebar of `metrics`: the lights and one gap after them on
/// the left, the light inset on the right (so the title stays centred when it fits and keeps off
/// the lights when it does not).
fn margins(metrics: &FrameMetrics) -> Margins {
    let lights = 3 * metrics.light + 2 * metrics.gap;
    Margins {
        left: metrics.inset + lights + metrics.gap,
        right: metrics.inset,
    }
}

/// The title pixels for each framed window on `frame` (the windows in `styles`). A window whose title cannot be drawn (empty, no room) has none.
pub(super) fn of(
    state: &mut State,
    frame: &OutputFrame,
    view: &OutputView,
    styles: &BTreeMap<WindowId, FrameStyle>,
) -> BTreeMap<WindowId, TitleDraw> {
    if styles.is_empty() {
        return BTreeMap::new();
    }
    let RenderState {
        titles,
        textures,
        scheme,
        ..
    } = &mut state.render;
    let raster = titles.get_or_insert_with(|| TitleRaster::system(margins(&frame_metrics())));
    let model = &state.core.model;
    let effect = model.settings().windows.minimise_effect;
    frame
        .windows
        .iter()
        .filter_map(|placed| {
            let style = styles.get(&placed.window)?;
            let window = model.windows().get(&placed.window)?;
            let entry = textures.window(&placed.window)?;
            let bar = style
                .around(window_drawn(placed, &entry.root, effect))
                .titlebar();
            let key = TitleKey::new(
                &window.title.0,
                bar.size.w,
                bar.size.h,
                view.scale,
                *scheme,
                activity_of(window),
            );
            let pixels = raster.render(&key).ok()?;
            Some((
                placed.window,
                TitleDraw {
                    id: pixels.id,
                    width: pixels.width,
                    height: pixels.height,
                    rgba: pixels.rgba,
                },
            ))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_title_keeps_off_the_lights_and_the_edge() {
        let metrics = frame_metrics();
        let got = margins(&metrics);
        // 13 inset, three 12 px lights, two 8 px gaps and one more gap: the title starts at 73.
        assert_eq!((got.left, got.right), (73, 13));
        assert!(got.left > metrics.inset + 3 * metrics.light + 2 * metrics.gap);
    }
}
