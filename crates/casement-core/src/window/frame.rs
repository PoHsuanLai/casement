//! The frame casement draws round a window whose client asked for a server-side one: where the
//! titlebar, the three lights and the resize zones are, and which of them a point is on.
//!
//! The lengths are an argument ([`FrameMetrics`]): `casement-layout` fills them from quire's
//! chrome tokens, so this crate stays free of the style crates and a test can use round
//! numbers. Everything is in global logical pixels. The titlebar sits just above the client's
//! content rectangle (the window's place in the model), so the content keeps its size.

use crate::event::ResizeEdges;
use crate::geometry::{LogicalPoint, LogicalRect, LogicalSize};

/// The lengths of a frame, in logical pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct FrameMetrics {
    /// The titlebar's height.
    pub titlebar: u32,
    /// A light's diameter.
    pub light: u32,
    /// The space between two lights.
    pub gap: u32,
    /// The first light's distance from the frame's left edge.
    pub inset: u32,
    /// The thickness of a side's resize zone, outside the frame.
    pub edge: u32,
    /// The side of a corner's square resize zone.
    pub corner: u32,
}

impl FrameMetrics {
    /// Metrics from their lengths.
    pub fn new(
        titlebar: u32,
        light: u32,
        gap: u32,
        inset: u32,
        edge: u32,
        corner: u32,
    ) -> FrameMetrics {
        FrameMetrics {
            titlebar,
            light,
            gap,
            inset,
            edge,
            corner,
        }
    }
}

/// One of the three lights.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Light {
    /// Closes the window (the leftmost).
    Close,
    /// Minimises it.
    Minimise,
    /// Zooms it.
    Zoom,
}

impl Light {
    /// The lights, left to right.
    pub const ALL: [Light; 3] = [Light::Close, Light::Minimise, Light::Zoom];

    /// Its place from the left.
    fn index(self) -> u32 {
        match self {
            Light::Close => 0,
            Light::Minimise => 1,
            Light::Zoom => 2,
        }
    }
}

/// What a point is on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum FrameHit {
    /// Not on the frame or its zones.
    Outside,
    /// The client's content.
    Content,
    /// The titlebar, off the lights.
    Titlebar,
    /// A light.
    Light(Light),
    /// A resize zone, with the edges it moves.
    Edge(ResizeEdges),
}

/// A frame placed round a content rectangle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameGeometry {
    metrics: FrameMetrics,
    content: LogicalRect,
}

impl FrameGeometry {
    /// The frame of a window whose content is `content`.
    ///
    /// ```
    /// use casement_core::geometry::{LogicalPoint, LogicalRect, LogicalSize};
    /// use casement_core::window::{FrameGeometry, FrameHit, FrameMetrics};
    ///
    /// let content = LogicalRect::new(LogicalPoint { x: 100, y: 100 }, LogicalSize { w: 400, h: 300 });
    /// let frame = FrameGeometry::new(FrameMetrics::new(28, 12, 8, 13, 4, 12), content);
    /// assert_eq!(frame.hit(LogicalPoint { x: 300, y: 80 }), FrameHit::Titlebar);
    /// ```
    pub fn new(metrics: FrameMetrics, content: LogicalRect) -> FrameGeometry {
        FrameGeometry { metrics, content }
    }

    /// The content rectangle.
    pub fn content(&self) -> LogicalRect {
        self.content
    }

    /// The titlebar's rectangle: as wide as the content, just above it.
    pub fn titlebar(&self) -> LogicalRect {
        LogicalRect::new(
            LogicalPoint {
                x: self.content.origin.x,
                y: self
                    .content
                    .origin
                    .y
                    .saturating_sub(to_i32(self.metrics.titlebar)),
            },
            LogicalSize {
                w: self.content.size.w,
                h: self.metrics.titlebar,
            },
        )
    }

    /// The titlebar and the content together.
    pub fn outline(&self) -> LogicalRect {
        let bar = self.titlebar();
        LogicalRect::new(
            bar.origin,
            LogicalSize {
                w: self.content.size.w,
                h: self.content.size.h.saturating_add(self.metrics.titlebar),
            },
        )
    }

    /// `light`'s rectangle: a square, centred up and down in the titlebar.
    pub fn light(&self, light: Light) -> LogicalRect {
        let bar = self.titlebar();
        let step = self.metrics.light + self.metrics.gap;
        let left = i64::from(bar.origin.x)
            + i64::from(self.metrics.inset)
            + i64::from(step) * i64::from(light.index());
        let spare = self.metrics.titlebar.saturating_sub(self.metrics.light) / 2;
        LogicalRect::new(
            LogicalPoint {
                x: i32::try_from(left).unwrap_or(i32::MAX),
                y: bar.origin.y.saturating_add(to_i32(spare)),
            },
            LogicalSize {
                w: self.metrics.light,
                h: self.metrics.light,
            },
        )
    }

    /// The rectangle of the three lights together, for the group's hover.
    pub fn lights(&self) -> LogicalRect {
        let (first, last) = (self.light(Light::Close), self.light(Light::Zoom));
        LogicalRect::new(
            first.origin,
            LogicalSize {
                w: u32::try_from(
                    i64::from(last.origin.x) + i64::from(last.size.w) - i64::from(first.origin.x),
                )
                .unwrap_or(0),
                h: first.size.h,
            },
        )
    }

    /// What `at` is on. A corner's square wins over a side's strip, a light over the titlebar.
    pub fn hit(&self, at: LogicalPoint) -> FrameHit {
        if let Some(edges) = self.edges_at(at) {
            return FrameHit::Edge(edges);
        }
        if let Some(light) = Light::ALL
            .into_iter()
            .find(|light| self.light(*light).contains(at))
        {
            return FrameHit::Light(light);
        }
        if self.titlebar().contains(at) {
            return FrameHit::Titlebar;
        }
        if self.content.contains(at) {
            return FrameHit::Content;
        }
        FrameHit::Outside
    }

    /// The resize zone `at` is in: the strips lie outside the outline, the corner squares
    /// start at its outer corners and reach inward.
    fn edges_at(&self, at: LogicalPoint) -> Option<ResizeEdges> {
        let outline = self.outline();
        let edge = i64::from(self.metrics.edge);
        let corner = i64::from(self.metrics.corner);
        let (x, y) = (i64::from(at.x), i64::from(at.y));
        let left = i64::from(outline.origin.x) - edge;
        let top = i64::from(outline.origin.y) - edge;
        let right = i64::from(outline.origin.x) + i64::from(outline.size.w) + edge;
        let bottom = i64::from(outline.origin.y) + i64::from(outline.size.h) + edge;
        if x < left || x >= right || y < top || y >= bottom {
            return None;
        }
        let (near_left, near_right) = (x < left + corner, x >= right - corner);
        let (near_top, near_bottom) = (y < top + corner, y >= bottom - corner);
        match (near_left, near_right, near_top, near_bottom) {
            (true, _, true, _) => return Some(ResizeEdges::TopLeft),
            (_, true, true, _) => return Some(ResizeEdges::TopRight),
            (true, _, _, true) => return Some(ResizeEdges::BottomLeft),
            (_, true, _, true) => return Some(ResizeEdges::BottomRight),
            _ => {}
        }
        let strip = (
            x < left + edge,
            x >= right - edge,
            y < top + edge,
            y >= bottom - edge,
        );
        match strip {
            (true, _, _, _) => Some(ResizeEdges::Left),
            (_, true, _, _) => Some(ResizeEdges::Right),
            (_, _, true, _) => Some(ResizeEdges::Top),
            (_, _, _, true) => Some(ResizeEdges::Bottom),
            _ => None,
        }
    }
}

/// `value` held to `i32`.
fn to_i32(value: u32) -> i32 {
    i32::try_from(value).unwrap_or(i32::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    const METRICS: FrameMetrics = FrameMetrics {
        titlebar: 28,
        light: 12,
        gap: 8,
        inset: 13,
        edge: 4,
        corner: 12,
    };

    /// Content at (100, 100), 400 x 300: the titlebar is y 72..100, the outline y 72..400.
    fn frame() -> FrameGeometry {
        FrameGeometry::new(
            METRICS,
            LogicalRect::new(
                LogicalPoint { x: 100, y: 100 },
                LogicalSize { w: 400, h: 300 },
            ),
        )
    }

    fn rect(x: i32, y: i32, w: u32, h: u32) -> LogicalRect {
        LogicalRect::new(LogicalPoint { x, y }, LogicalSize { w, h })
    }

    #[test]
    fn the_titlebar_sits_above_the_content() {
        assert_eq!(frame().titlebar(), rect(100, 72, 400, 28));
        assert_eq!(frame().outline(), rect(100, 72, 400, 328));
    }

    #[test]
    fn the_lights_are_twelve_square_twenty_apart_and_centred() {
        type Case = (&'static str, Light, (i32, i32, u32, u32));
        const CASES: &[Case] = &[
            ("close", Light::Close, (113, 80, 12, 12)),
            ("minimise", Light::Minimise, (133, 80, 12, 12)),
            ("zoom", Light::Zoom, (153, 80, 12, 12)),
        ];
        for (name, light, (x, y, w, h)) in CASES {
            assert_eq!(frame().light(*light), rect(*x, *y, *w, *h), "{name}");
        }
        assert_eq!(frame().lights(), rect(113, 80, 52, 12));
    }

    #[test]
    fn a_point_is_on_the_part_it_lies_in() {
        const CASES: &[(&str, (i32, i32), FrameHit)] = &[
            ("close light", (119, 86), FrameHit::Light(Light::Close)),
            (
                "minimise light",
                (139, 86),
                FrameHit::Light(Light::Minimise),
            ),
            ("zoom light", (159, 86), FrameHit::Light(Light::Zoom)),
            ("gap between lights", (126, 86), FrameHit::Titlebar),
            ("titlebar middle", (300, 86), FrameHit::Titlebar),
            ("titlebar above a light", (119, 74), FrameHit::Titlebar),
            ("last titlebar row", (300, 99), FrameHit::Titlebar),
            ("first content row", (300, 100), FrameHit::Content),
            ("content middle", (300, 250), FrameHit::Content),
            (
                "content short of the corner square",
                (480, 380),
                FrameHit::Content,
            ),
            ("top strip", (300, 70), FrameHit::Edge(ResizeEdges::Top)),
            ("left strip", (97, 250), FrameHit::Edge(ResizeEdges::Left)),
            (
                "right strip",
                (502, 250),
                FrameHit::Edge(ResizeEdges::Right),
            ),
            (
                "bottom strip",
                (300, 402),
                FrameHit::Edge(ResizeEdges::Bottom),
            ),
            (
                "top left corner",
                (97, 69),
                FrameHit::Edge(ResizeEdges::TopLeft),
            ),
            (
                "top left corner, inside",
                (105, 77),
                FrameHit::Edge(ResizeEdges::TopLeft),
            ),
            (
                "top right corner",
                (502, 69),
                FrameHit::Edge(ResizeEdges::TopRight),
            ),
            (
                "bottom left corner",
                (97, 402),
                FrameHit::Edge(ResizeEdges::BottomLeft),
            ),
            (
                "bottom right corner",
                (499, 399),
                FrameHit::Edge(ResizeEdges::BottomRight),
            ),
            ("beyond the strip, left", (95, 250), FrameHit::Outside),
            ("beyond the strip, below", (300, 404), FrameHit::Outside),
            ("elsewhere", (0, 0), FrameHit::Outside),
        ];
        for (name, (x, y), want) in CASES {
            assert_eq!(frame().hit(LogicalPoint { x: *x, y: *y }), *want, "{name}");
        }
    }
}
