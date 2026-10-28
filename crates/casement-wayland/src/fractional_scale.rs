//! `wp_fractional_scale_v1`: the exact scale a surface should draw at, sent before its first
//! configure (shell-host FINDINGS 167).

use crate::states::{GlobalFilter, WaylandFault};
use casement_core::Event;
use casement_core::OutputBook;
use ds_core::geometry::scale::Scale;
use smithay::reexports::wayland_protocols::wp::fractional_scale::v1::server::{
    wp_fractional_scale_manager_v1::WpFractionalScaleManagerV1,
    wp_fractional_scale_v1::WpFractionalScaleV1,
};
use smithay::reexports::wayland_server::Resource;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::reexports::wayland_server::{Dispatch, DisplayHandle, GlobalDispatch};
use smithay::utils::Transform;
use smithay::wayland::GlobalData;
use smithay::wayland::compositor::{send_surface_state, with_states};
use smithay::wayland::fractional_scale::{
    FractionalScaleData, FractionalScaleHandler, FractionalScaleManagerState, with_fractional_scale,
};

/// `wp_fractional_scale_manager_v1`.
#[derive(Debug)]
pub struct FractionalScalePart {
    /// smithay's state.
    pub state: FractionalScaleManagerState,
}

impl FractionalScalePart {
    /// Creates the global (version 1). Every connection sees it.
    pub fn new<D>(display: &DisplayHandle, role_filter: GlobalFilter) -> FractionalScalePart
    where
        D: GlobalDispatch<WpFractionalScaleManagerV1, GlobalData>
            + Dispatch<WpFractionalScaleManagerV1, GlobalData>
            + Dispatch<WpFractionalScaleV1, FractionalScaleData>
            + FractionalScaleHandler
            + 'static,
    {
        let _ = role_filter;
        FractionalScalePart {
            state: FractionalScaleManagerState::new::<D>(display),
        }
    }
}

/// The scale a new surface is told to draw at, before it has a window and so before anyone
/// can say which output it will appear on: the primary output's, which is where every new
/// window opens (the output at the top-left of the global space, ties by key). `None` while no
/// output is plugged: the client then assumes 1x until [`apply_scale`] says otherwise.
pub fn home_scale(outputs: &OutputBook) -> Option<Scale> {
    outputs
        .0
        .iter()
        .min_by_key(|(key, state)| (state.config.position.x, state.config.position.y, *key))
        .map(|(_, state)| state.config.scale)
}

/// A surface asked for its preferred scale (or was created): nothing reaches the model; the
/// handler answers with [`apply_scale`] for [`home_scale`], before the surface's first
/// configure (the surface already carries the scale when its `wp_fractional_scale_v1` is
/// created, so the event is the first thing the object sees).
pub fn on_preferred(part: &mut FractionalScalePart, surface: &WlSurface) -> Vec<Event> {
    let _ = (part, surface);
    Vec::new()
}

/// Sends `scale` as `surface`'s preferred scale (in 120ths on the wire) and, for a `wl_surface`
/// of version 6 or later, the matching integer buffer scale (the ceiling). smithay sends each
/// only when it changes.
pub fn apply_scale(
    part: &mut FractionalScalePart,
    surface: &WlSurface,
    scale: Scale,
) -> Result<(), WaylandFault> {
    let _ = part;
    if !surface.is_alive() {
        return Err(WaylandFault::SurfaceGone);
    }
    with_states(surface, |states| {
        with_fractional_scale(states, |state| state.set_preferred_scale(scale.as_f64()));
        send_surface_state(
            surface,
            states,
            i32::try_from(scale.numerator().div_ceil(Scale::DENOMINATOR)).unwrap_or(i32::MAX),
            Transform::Normal,
        );
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use casement_core::OutputState;
    use casement_core::geometry::{LogicalPoint, PhysicalSize};
    use casement_core::output::{
        EdidFacts, EdidSerial, Mode, ModePreference, OutputConfig, OutputFacts, OutputPower,
        PhysicalMm, VrrCapability,
    };
    use casement_core::time::Millihertz;
    use casement_core::{OutputKey, OutputName};
    use casement_protocols::vocab::Transform as Turn;

    fn output(name: &str, at: (i32, i32), scale: u32) -> (OutputKey, OutputState) {
        let name = OutputName::parse(name).expect("a connector name");
        let edid = EdidFacts {
            make: "Test".to_owned(),
            model: "Panel".to_owned(),
            serial: EdidSerial::Unknown,
        };
        let key = OutputKey::of(&edid, &name);
        let mode = Mode {
            size: PhysicalSize { w: 1920, h: 1080 },
            refresh: Millihertz(60_000),
            preference: ModePreference::Preferred,
        };
        let facts = OutputFacts {
            name,
            key: key.clone(),
            edid,
            modes: vec![mode],
            physical: PhysicalMm { w: 0, h: 0 },
            vrr: VrrCapability::Incapable,
        };
        let config = OutputConfig {
            mode,
            scale: Scale(scale),
            transform: Turn::Normal,
            position: LogicalPoint { x: at.0, y: at.1 },
            power: OutputPower::On,
        };
        (key, OutputState { facts, config })
    }

    #[test]
    fn new_surfaces_take_the_top_left_outputs_scale() {
        assert_eq!(home_scale(&OutputBook::default()), None);
        let book = OutputBook(
            [
                output("HEADLESS-2", (1920, 0), 240),
                output("HEADLESS-1", (0, 0), 180),
                output("HEADLESS-3", (0, 1080), 120),
            ]
            .into_iter()
            .collect(),
        );
        assert_eq!(home_scale(&book), Some(Scale(180)));
    }

    #[test]
    fn fractional_scales_are_exact_on_the_wire() {
        for numerator in [120_u32, 150, 180, 210, 240, 360] {
            let wire = (Scale(numerator).as_f64() * 120.0).round();
            assert_eq!(wire, f64::from(numerator));
        }
    }
}
