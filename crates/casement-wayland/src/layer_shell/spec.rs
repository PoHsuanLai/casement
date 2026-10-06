//! What a layer surface asked for, in the model's words: smithay's flags and integers become
//! [`LayerSpec`] here (where a layer sits is `casement_core::layer::Arranged`, read by everyone).

use casement_core::OutputKey;
use casement_core::geometry::LogicalSize;
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
        size: LogicalSize {
            w: u32::try_from(cached.size.w).unwrap_or(0),
            h: u32::try_from(cached.size.h).unwrap_or(0),
        },
        exclusive: exclusive_of(cached.exclusive_zone),
        margin: margins_of(cached.margin),
        keyboard: keyboard_of(cached.keyboard_interactivity),
        namespace: namespace.to_owned(),
        role: ShellRoleTag::Unassigned,
    }
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
}
