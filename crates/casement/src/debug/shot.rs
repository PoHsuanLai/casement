//! `Shot`: the last frame an output presented, or a region of it, written as a PNG.

use super::png::encode_rgba;
use casement::State;
use casement_backend::{BackendKind, FramePixels};
use casement_core::geometry::LogicalRect;
use casement_ipc::ShotArea;
use casement_ipc::dbus::DebugReply;

/// The scale denominator: output scales are in 120ths.
const PER_UNIT: i64 = 120;

/// Writes the shot `area` names to `path`; answers its size, or why it could not be taken.
pub(super) fn shot(state: &State, area: &ShotArea, path: &str) -> DebugReply {
    let outputs = super::dump::outputs(state);
    let chosen = match area {
        ShotArea::Output(Some(name)) => outputs.iter().find(|(_, found, _)| found == name),
        ShotArea::Output(None) => outputs.first(),
        ShotArea::Region(rect) => outputs
            .iter()
            .find(|(_, _, config)| config.logical_rect().contains(rect.origin)),
    };
    let Some((_, name, config)) = chosen else {
        return DebugReply::Refused(format!("shot {area:?}: no such output"));
    };
    let BackendKind::Headless(backend) = &state.backend else {
        return DebugReply::Refused("shot works on the headless backend only".to_owned());
    };
    let Some(frame) = backend.last_frame(name) else {
        return DebugReply::Refused(format!(
            "{}: no frame has been presented yet (step the clock)",
            name.as_str()
        ));
    };
    let FramePixels::Rgba(pixels) = &frame.pixels else {
        return DebugReply::Refused("the dummy renderer draws nothing to shoot".to_owned());
    };
    let scale = i64::from(config.scale.0);
    let (x0, y0, w, h, rgba) = match area {
        ShotArea::Region(rect) => {
            let (x0, y0, w, h) = physical_region(rect, config.logical_rect().origin, scale);
            let cut = cut(&pixels.bytes, pixels.size.w as usize, (x0, y0, w, h));
            (x0, y0, w, h, cut)
        }
        ShotArea::Output(_) => (
            0,
            0,
            pixels.size.w as usize,
            pixels.size.h as usize,
            Some(pixels.bytes.clone()),
        ),
    };
    let Some(rgba) = rgba else {
        return DebugReply::Refused(format!(
            "the region lies outside {}'s {}x{} frame",
            name.as_str(),
            pixels.size.w,
            pixels.size.h
        ));
    };
    let png = encode_rgba(
        u32::try_from(w).unwrap_or(0),
        u32::try_from(h).unwrap_or(0),
        &rgba,
    );
    match std::fs::write(path, png) {
        Ok(()) => DebugReply::Json(format!(
            "{{\"output\":\"{}\",\"x\":{x0},\"y\":{y0},\"width\":{w},\"height\":{h},\"scale\":{scale}}}",
            name.as_str()
        )),
        Err(error) => DebugReply::Refused(format!("writing {path}: {error}")),
    }
}

/// `rect` (global logical) as physical pixels of an output at `origin` and `scale` (120ths):
/// x, y, width, height, edges rounded outwards so no pixel of the region is lost.
fn physical_region(
    rect: &LogicalRect,
    origin: casement_core::geometry::LogicalPoint,
    scale: i64,
) -> (usize, usize, usize, usize) {
    let down = |logical: i64| (logical * scale).div_euclid(PER_UNIT);
    let up = |logical: i64| (logical * scale + PER_UNIT - 1).div_euclid(PER_UNIT);
    let (x, y) = (
        i64::from(rect.origin.x) - i64::from(origin.x),
        i64::from(rect.origin.y) - i64::from(origin.y),
    );
    let (x0, y0) = (down(x).max(0), down(y).max(0));
    let (x1, y1) = (
        up(x + i64::from(rect.size.w)),
        up(y + i64::from(rect.size.h)),
    );
    let to = |n: i64| usize::try_from(n).unwrap_or(0);
    (to(x0), to(y0), to(x1 - x0), to(y1 - y0))
}

/// The rows of the `(x, y, w, h)` window of a `stride`-pixel-wide RGBA image, or `None` when
/// the window leaves the image or is empty.
fn cut(bytes: &[u8], stride: usize, (x, y, w, h): (usize, usize, usize, usize)) -> Option<Vec<u8>> {
    let rows = bytes.len() / 4 / stride.max(1);
    if w == 0 || h == 0 || x + w > stride || y + h > rows {
        return None;
    }
    Some(
        (y..y + h)
            .flat_map(|row| {
                let from = (row * stride + x) * 4;
                bytes[from..from + w * 4].iter().copied()
            })
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use casement_core::geometry::{LogicalPoint, LogicalSize};

    #[test]
    fn a_region_at_one_and_a_half_covers_whole_pixels() {
        let rect = LogicalRect::new(LogicalPoint { x: 100, y: 50 }, LogicalSize { w: 3, h: 3 });
        let origin = LogicalPoint { x: 0, y: 0 };
        assert_eq!(physical_region(&rect, origin, 180), (150, 75, 5, 5));
        assert_eq!(physical_region(&rect, origin, 120), (100, 50, 3, 3));
        // On the second output of a row, the region is relative to its origin.
        let right = LogicalPoint { x: 1920, y: 0 };
        let rect = LogicalRect::new(LogicalPoint { x: 1930, y: 0 }, LogicalSize { w: 4, h: 4 });
        assert_eq!(physical_region(&rect, right, 120), (10, 0, 4, 4));
    }

    #[test]
    fn a_cut_takes_the_rows_of_its_window() {
        // 3x2 image, pixel value = index.
        let bytes: Vec<u8> = (0..6_u8).flat_map(|n| [n, n, n, 255]).collect();
        let got = cut(&bytes, 3, (1, 0, 2, 2)).expect("inside");
        let firsts: Vec<u8> = got.chunks(4).map(|px| px[0]).collect();
        assert_eq!(firsts, [1, 2, 4, 5]);
        assert!(
            cut(&bytes, 3, (2, 0, 2, 1)).is_none(),
            "past the right edge"
        );
        assert!(cut(&bytes, 3, (0, 1, 1, 2)).is_none(), "past the bottom");
        assert!(cut(&bytes, 3, (0, 0, 0, 1)).is_none(), "empty");
    }
}
