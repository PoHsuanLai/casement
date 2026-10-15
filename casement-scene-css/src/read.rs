//! Reading the laid-out document back into an [`Arrangement`]: each element's final box in
//! logical pixels.
//!
//! The boxes are all the CSS gives. What a box means to the compositor is derived here, from the
//! box and the spec, by the same definitions `RustScene` states: a window is configured when its
//! size is not the one it asked for (a full-screen or Split View window always), and a window
//! smaller than its minimum is reported. Neither is layout, and neither has a CSS form.

use crate::document::{SceneDocument, SpaceBody, SpaceNodes, SplitNodes, WindowNode};
use blitz_dom::{BaseDocument, NodeId};
use casement_core::geometry::{LogicalPoint, LogicalRect, LogicalSize};
use casement_layout::{
    Arrangement, BackdropTarget, ConfigureNeed, DividerTarget, LayoutReport, OutputSpec, SceneSpec,
    SpaceSpec, SpaceTarget, WindowPlacement, WindowSpec, WindowTarget,
};

/// The arrangement the laid-out `document` describes for `spec`.
///
/// Mission Control's slots (`Arrangement::overview`) and its `SpreadOverflow` report are not
/// produced: the spread is an iterative search CSS cannot express (FINDINGS.md, "the
/// exclusions"). Everything else `RustScene` lays out today is.
pub fn arrangement(document: &SceneDocument, spec: &SceneSpec) -> Arrangement {
    let mut into = Arrangement::default();
    for (output, nodes) in spec.outputs.iter().zip(&document.outputs) {
        for (space, nodes) in output.spaces.iter().zip(&nodes.spaces) {
            read_space(&document.dom, output, space, nodes, &mut into);
        }
    }
    into
}

fn read_space(
    dom: &BaseDocument,
    output: &OutputSpec,
    space: &SpaceSpec,
    nodes: &SpaceNodes,
    into: &mut Arrangement,
) {
    if let Some(at) = box_of(dom, nodes.origin) {
        into.spaces.push(SpaceTarget {
            space: space.space.clone(),
            output: output.output.clone(),
            origin: at.origin,
        });
    }
    match &nodes.body {
        SpaceBody::Desktop(windows) => {
            for window in windows {
                read_desktop_window(dom, output, space, *window, into);
            }
        }
        SpaceBody::Split(Some(split)) => read_split(dom, output, space, split, into),
        SpaceBody::Split(None) => {}
    }
}

fn read_desktop_window(
    dom: &BaseDocument,
    output: &OutputSpec,
    space: &SpaceSpec,
    window: WindowNode,
    into: &mut Arrangement,
) {
    let (Some(spec), Some(rect)) = (space.windows.get(window.index), box_of(dom, window.node))
    else {
        return;
    };
    let size = match spec.placement {
        WindowPlacement::FullScreen => ConfigureNeed::Resize(rect.size),
        _ => need_of(asked(spec), rect.size),
    };
    let unmet = match spec.placement {
        WindowPlacement::FullScreen => None,
        _ => min_unmet(spec, rect.size),
    };
    into.windows.push(WindowTarget {
        window: spec.window,
        output: output.output.clone(),
        rect,
        size,
    });
    into.reports.extend(unmet);
}

fn read_split(
    dom: &BaseDocument,
    output: &OutputSpec,
    space: &SpaceSpec,
    split: &SplitNodes,
    into: &mut Arrangement,
) {
    let (Some(leading), Some(trailing), Some(strip)) = (
        space.windows.get(split.leading.index),
        space.windows.get(split.trailing.index),
        box_of(dom, split.strip),
    ) else {
        return;
    };
    into.dividers.push(DividerTarget {
        space: space.space.clone(),
        output: output.output.clone(),
        leading: leading.window,
        trailing: trailing.window,
        strip,
        span: output.full,
        min_leading: leading.limits.min.w,
        min_trailing: trailing.limits.min.w,
    });
    for (spec, node) in [(leading, split.leading), (trailing, split.trailing)] {
        let Some(rect) = box_of(dom, node.node) else {
            continue;
        };
        into.windows.push(WindowTarget {
            window: spec.window,
            output: output.output.clone(),
            rect,
            size: ConfigureNeed::Resize(rect.size),
        });
        into.reports.extend(min_unmet(spec, rect.size));
    }
    into.backdrops.push(BackdropTarget {
        space: space.space.clone(),
        output: output.output.clone(),
    });
}

/// The size the model asked `spec`'s window for (a filled window asks for none).
fn asked(spec: &WindowSpec) -> LogicalSize {
    match spec.placement {
        WindowPlacement::Floating(rect) | WindowPlacement::Placed(rect) => rect.size,
        WindowPlacement::New(size) => size,
        _ => LogicalSize { w: 0, h: 0 },
    }
}

fn need_of(asked: LogicalSize, got: LogicalSize) -> ConfigureNeed {
    match got == asked {
        true => ConfigureNeed::Keep,
        false => ConfigureNeed::Resize(got),
    }
}

/// The report for a window laid out smaller than its minimum.
fn min_unmet(spec: &WindowSpec, got: LogicalSize) -> Option<LayoutReport> {
    let min = spec.limits.min;
    (got.w < min.w || got.h < min.h).then_some(LayoutReport::MinUnmet {
        window: spec.window,
        min,
        got,
    })
}

/// `node`'s border box in global logical pixels, each edge rounded to the nearest pixel (the
/// custom properties are whole pixels, so layout hands back whole pixels; the rounding is
/// for f32 noise only).
fn box_of(dom: &BaseDocument, node: NodeId) -> Option<LogicalRect> {
    let node = dom.get_node(node)?;
    let at = node.absolute_position(0.0, 0.0);
    let size = node.final_layout().size;
    Some(LogicalRect::new(
        LogicalPoint {
            x: whole(at.x),
            y: whole(at.y),
        },
        LogicalSize {
            w: extent(size.width),
            h: extent(size.height),
        },
    ))
}

/// A coordinate: the nearest whole pixel.
fn whole(value: f32) -> i32 {
    value.round() as i32
}

/// An extent: the nearest whole pixel, never negative.
fn extent(value: f32) -> u32 {
    value.round().max(0.0) as u32
}
