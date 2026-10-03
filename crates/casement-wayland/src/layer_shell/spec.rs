//! What a layer surface asked for, in the model's words: smithay's flags and integers become
//! [`LayerSpec`] here, and the rectangle a layer occupies comes from its anchors and margins.

use casement_core::OutputKey;
use casement_core::geometry::{LogicalPoint, LogicalRect, LogicalSize};
use casement_core::layer::{
    Anchor, Anchors, ExclusiveZone, KeyboardInteractivity, LayerLevel, LayerSpec, Margins,
    ShellRoleTag,
};
use smithay::wayland::shell::wlr_layer::{
    Anchor as WireAnchor, ExclusiveZone as WireZone, KeyboardInteractivity as WireKeyboard,
    Layer as WireLayer, LayerSurfaceCachedState, Margins as WireMargins,
};

/// The model's level for a wire layer.
pub fn level_of(layer: WireLayer) -> LayerLevel {
    match layer {
        WireLayer::Background => LayerLevel::Background,
        WireLayer::Bottom => LayerLevel::Bottom,
        WireLayer::Top => LayerLevel::Top,
        WireLayer::Overlay => LayerLevel::Overlay,
    }
}

fn held(anchor: WireAnchor, edge: WireAnchor) -> Anchor {
    if anchor.contains(edge) {
        Anchor::Held
    } else {
        Anchor::Free
    }
}

/// The edges a surface is held to.
pub fn anchors_of(anchor: WireAnchor) -> Anchors {
    Anchors {
        top: held(anchor, WireAnchor::TOP),
        bottom: held(anchor, WireAnchor::BOTTOM),
        left: held(anchor, WireAnchor::LEFT),
        right: held(anchor, WireAnchor::RIGHT),
    }
}

/// How the surface treats the space beside its edge.
pub fn exclusive_of(zone: WireZone) -> ExclusiveZone {
    match zone {
        WireZone::Exclusive(depth) => ExclusiveZone::Reserve(depth),
        WireZone::Neutral => ExclusiveZone::Respect,
        WireZone::DontCare => ExclusiveZone::Ignore,
    }
}

/// The surface's distance from each anchored edge.
pub fn margins_of(margin: WireMargins) -> Margins {
    Margins {
        top: margin.top,
        right: margin.right,
        bottom: margin.bottom,
        left: margin.left,
    }
}

/// Whether the surface takes the keyboard.
pub fn keyboard_of(keyboard: WireKeyboard) -> KeyboardInteractivity {
    match keyboard {
        WireKeyboard::None => KeyboardInteractivity::None,
        WireKeyboard::OnDemand => KeyboardInteractivity::OnDemand,
        WireKeyboard::Exclusive => KeyboardInteractivity::Exclusive,
    }
}

/// The spec a surface's committed state describes, on `output`.
pub fn spec_of(output: &OutputKey, cached: &LayerSurfaceCachedState, namespace: &str) -> LayerSpec {
    LayerSpec {
        output: output.clone(),
        level: level_of(cached.layer),
        anchor: anchors_of(cached.anchor),
        exclusive: exclusive_of(cached.exclusive_zone),
        margin: margins_of(cached.margin),
        keyboard: keyboard_of(cached.keyboard_interactivity),
        namespace: namespace.to_owned(),
        role: ShellRoleTag::Unassigned,
    }
}

/// Where one axis of a surface sits: its start and length inside `[origin, origin + room)`.
/// Held at both ends it fills the room less the margins (a zero committed length means "as
/// much as there is"); held at one end it sits against it; otherwise it is centred.
fn along(
    room: (i32, u32),
    ends: (Anchor, Anchor),
    margins: (i32, i32),
    committed: u32,
) -> (i32, u32) {
    let (origin, space) = room;
    let (near, far) = margins;
    let end = |value: i64| i32::try_from(value).unwrap_or(i32::MAX);
    let length = |value: i64| u32::try_from(value.max(0)).unwrap_or(u32::MAX);
    match ends {
        (Anchor::Held, Anchor::Held) => {
            let span = i64::from(space) - i64::from(near) - i64::from(far);
            let own = if committed == 0 {
                length(span)
            } else {
                committed
            };
            let slack = (span - i64::from(own)).max(0);
            (end(i64::from(origin) + i64::from(near) + slack / 2), own)
        }
        (Anchor::Held, Anchor::Free) => (end(i64::from(origin) + i64::from(near)), committed),
        (Anchor::Free, Anchor::Held) => (
            end(i64::from(origin) + i64::from(space) - i64::from(far) - i64::from(committed)),
            committed,
        ),
        (Anchor::Free, Anchor::Free) => (
            end(i64::from(origin) + (i64::from(space) - i64::from(committed)) / 2),
            committed,
        ),
    }
}

/// The rectangle (global logical) a layer surface of `committed` size occupies on an output
/// whose whole rectangle is `output`, from its anchors and margins.
pub fn layer_area(spec: &LayerSpec, output: LogicalRect, committed: LogicalSize) -> LogicalRect {
    let (x, w) = along(
        (output.origin.x, output.size.w),
        (spec.anchor.left, spec.anchor.right),
        (spec.margin.left, spec.margin.right),
        committed.w,
    );
    let (y, h) = along(
        (output.origin.y, output.size.h),
        (spec.anchor.top, spec.anchor.bottom),
        (spec.margin.top, spec.margin.bottom),
        committed.h,
    );
    LogicalRect::new(LogicalPoint { x, y }, LogicalSize { w, h })
}

#[cfg(test)]
mod tests {
    use super::*;
    use casement_core::OutputName;
    use casement_core::output::{EdidFacts, EdidSerial};

    fn output_key() -> OutputKey {
        let edid = EdidFacts {
            make: String::new(),
            model: String::new(),
            serial: EdidSerial::Unknown,
        };
        OutputKey::of(&edid, &OutputName::parse("HEADLESS-1").expect("name"))
    }

    fn spec(anchor: WireAnchor, margin: WireMargins) -> LayerSpec {
        let cached = LayerSurfaceCachedState {
            anchor,
            margin,
            ..LayerSurfaceCachedState::default()
        };
        spec_of(&output_key(), &cached, "bar")
    }

    const NO_MARGIN: WireMargins = WireMargins {
        top: 0,
        right: 0,
        bottom: 0,
        left: 0,
    };

    /// One placement case.
    struct Case {
        name: &'static str,
        anchor: WireAnchor,
        margin: WireMargins,
        committed: (u32, u32),
        want: LogicalRect,
    }

    fn rect(x: i32, y: i32, w: u32, h: u32) -> LogicalRect {
        LogicalRect::new(LogicalPoint { x, y }, LogicalSize { w, h })
    }

    #[test]
    fn wire_values_become_the_models() {
        assert_eq!(level_of(WireLayer::Overlay), LayerLevel::Overlay);
        assert_eq!(level_of(WireLayer::Background), LayerLevel::Background);
        assert_eq!(exclusive_of(WireZone::from(30)), ExclusiveZone::Reserve(30));
        assert_eq!(exclusive_of(WireZone::from(0)), ExclusiveZone::Respect);
        assert_eq!(exclusive_of(WireZone::from(-1)), ExclusiveZone::Ignore);
        assert_eq!(
            keyboard_of(WireKeyboard::Exclusive),
            KeyboardInteractivity::Exclusive
        );
        assert_eq!(
            keyboard_of(WireKeyboard::OnDemand),
            KeyboardInteractivity::OnDemand
        );
        let anchors = anchors_of(WireAnchor::TOP | WireAnchor::LEFT | WireAnchor::RIGHT);
        assert_eq!(
            (anchors.top, anchors.bottom, anchors.left, anchors.right),
            (Anchor::Held, Anchor::Free, Anchor::Held, Anchor::Held)
        );
        let s = spec(WireAnchor::TOP, NO_MARGIN);
        assert_eq!(s.namespace, "bar");
        assert_eq!(s.role, ShellRoleTag::Unassigned);
    }

    #[test]
    fn a_layer_sits_where_its_anchors_put_it() {
        let screen = rect(100, 50, 1920, 1080);
        let margin = WireMargins {
            top: 8,
            right: 10,
            bottom: 12,
            left: 6,
        };
        let bar_anchor = WireAnchor::TOP | WireAnchor::LEFT | WireAnchor::RIGHT;
        let corner = WireAnchor::BOTTOM | WireAnchor::RIGHT;
        let cases = [
            Case {
                name: "a bar spans the width at the top",
                anchor: bar_anchor,
                margin: NO_MARGIN,
                committed: (0, 30),
                want: rect(100, 50, 1920, 30),
            },
            Case {
                name: "margins shrink a spanning bar and push it off the edge",
                anchor: bar_anchor,
                margin,
                committed: (0, 30),
                want: rect(106, 58, 1904, 30),
            },
            Case {
                name: "a corner layer sits against both its edges",
                anchor: corner,
                margin,
                committed: (200, 100),
                want: rect(100 + 1920 - 10 - 200, 50 + 1080 - 12 - 100, 200, 100),
            },
            Case {
                name: "no anchor centres",
                anchor: WireAnchor::empty(),
                margin: NO_MARGIN,
                committed: (400, 200),
                want: rect(100 + 760, 50 + 440, 400, 200),
            },
            Case {
                name: "a full anchor with a committed size centres inside the margins",
                anchor: WireAnchor::all(),
                margin: NO_MARGIN,
                committed: (320, 240),
                want: rect(100 + 800, 50 + 420, 320, 240),
            },
            Case {
                name: "a full anchor with no size fills the output",
                anchor: WireAnchor::all(),
                margin: NO_MARGIN,
                committed: (0, 0),
                want: screen,
            },
        ];
        for case in cases {
            let (w, h) = case.committed;
            let got = layer_area(
                &spec(case.anchor, case.margin),
                screen,
                LogicalSize { w, h },
            );
            assert_eq!(got, case.want, "{}", case.name);
        }
    }
}
