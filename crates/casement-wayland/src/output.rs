//! `wl_output` and `zxdg_output_manager_v1`: one global per plugged output, created and
//! removed as the backend reports connectors (`ConnectorEvent`), the same path for headless
//! and KMS. (Not in the plan's module list; `GlobalKind::Output` needs a home: FINDINGS.)

use crate::states::{GlobalFilter, WaylandFault};
use casement_core::geometry::LogicalPoint;
use casement_core::output::{EdidFacts, EdidSerial, Mode, ModePreference, PhysicalMm};
use casement_core::{OutputKey, OutputName};
use casement_protocols::vocab::Transform;
use ds_core::geometry::scale::Scale;
use smithay::output::{Mode as WireMode, Output, PhysicalProperties, Scale as WireScale, Subpixel};
use smithay::reexports::wayland_protocols::xdg::xdg_output::zv1::server::zxdg_output_manager_v1::ZxdgOutputManagerV1;
use smithay::reexports::wayland_server::backend::GlobalId;
use smithay::reexports::wayland_server::protocol::wl_output::WlOutput;
use smithay::reexports::wayland_server::{DisplayHandle, GlobalDispatch};
use smithay::utils::Transform as WireTransform;
use smithay::wayland::GlobalData;
use smithay::wayland::output::{OutputManagerState, WlOutputData};
use std::collections::BTreeMap;

/// `wl_output` globals and the xdg-output manager.
#[derive(Debug)]
pub struct OutputPart {
    /// smithay's state.
    pub state: OutputManagerState,
    /// The advertised outputs.
    pub outputs: BTreeMap<OutputKey, Output>,
    /// How to withdraw each output's global (it needs the state type the global was created
    /// with, which only `apply_plug` knows).
    removers: BTreeMap<OutputKey, Remover>,
}

/// Withdraws one `wl_output` global.
struct Remover(Box<dyn FnOnce(&DisplayHandle)>);

impl std::fmt::Debug for Remover {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Remover")
    }
}

impl OutputPart {
    /// Creates the `zxdg_output_manager_v1` global (version 3); `wl_output` globals come with
    /// [`apply_plug`]. Every connection sees them.
    pub fn new<D>(display: &DisplayHandle, role_filter: GlobalFilter) -> OutputPart
    where
        D: GlobalDispatch<WlOutput, WlOutputData>
            + GlobalDispatch<ZxdgOutputManagerV1, GlobalData>
            + 'static,
    {
        let _ = role_filter;
        OutputPart {
            state: OutputManagerState::new_with_xdg_output::<D>(display),
            outputs: BTreeMap::new(),
            removers: BTreeMap::new(),
        }
    }
}

/// What a `wl_output` advertises.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputWire {
    /// The display's stable key.
    pub key: OutputKey,
    /// The connector.
    pub name: OutputName,
    /// The EDID identity (make and model go on the wire).
    pub edid: EdidFacts,
    /// The current mode.
    pub mode: Mode,
    /// Its scale (the integer `wl_output.scale` is its ceiling; fractional clients get the
    /// exact value through `wp_fractional_scale_v1`).
    pub scale: Scale,
    /// Its transform.
    pub transform: Transform,
    /// Its top-left in the global logical space.
    pub position: LogicalPoint,
    /// Its physical size.
    pub physical: PhysicalMm,
}

/// Our transform as smithay's.
fn transform_to(transform: Transform) -> WireTransform {
    match transform {
        Transform::Normal => WireTransform::Normal,
        Transform::R90 => WireTransform::_90,
        Transform::R180 => WireTransform::_180,
        Transform::R270 => WireTransform::_270,
        Transform::Flipped => WireTransform::Flipped,
        Transform::Flipped90 => WireTransform::Flipped90,
        Transform::Flipped180 => WireTransform::Flipped180,
        Transform::Flipped270 => WireTransform::Flipped270,
    }
}

/// Our mode as smithay's (a refresh beyond `i32::MAX` mHz saturates).
fn mode_to(mode: &Mode) -> WireMode {
    let side = |pixels: u32| i32::try_from(pixels).unwrap_or(i32::MAX);
    WireMode {
        size: (side(mode.size.w), side(mode.size.h)).into(),
        refresh: i32::try_from(mode.refresh.0).unwrap_or(i32::MAX),
    }
}

/// The scale `wl_output` and `xdg_output` advertise: the exact fraction (the integer
/// `wl_output.scale` is its ceiling; fractional clients take the exact value from
/// `wp_fractional_scale_v1`).
fn scale_to(scale: Scale) -> WireScale {
    WireScale::Fractional(scale.as_f64())
}

/// A smithay `Output` carrying everything `wire` says.
fn build(wire: &OutputWire) -> Output {
    let side = |mm: u32| i32::try_from(mm).unwrap_or(i32::MAX);
    let output = Output::new(
        wire.name.as_str().to_owned(),
        PhysicalProperties {
            size: (side(wire.physical.w), side(wire.physical.h)).into(),
            subpixel: Subpixel::Unknown,
            make: wire.edid.make.clone(),
            model: wire.edid.model.clone(),
            serial_number: match &wire.edid.serial {
                EdidSerial::Known(serial) => serial.clone(),
                EdidSerial::Unknown => "Unknown".to_owned(),
            },
        },
    );
    let mode = mode_to(&wire.mode);
    match wire.mode.preference {
        ModePreference::Preferred => output.set_preferred(mode),
        ModePreference::Other => output.add_mode(mode),
    }
    apply_state(&output, wire);
    output
}

/// Sends `output` the state `wire` says.
fn apply_state(output: &Output, wire: &OutputWire) {
    output.change_current_state(
        Some(mode_to(&wire.mode)),
        Some(transform_to(wire.transform)),
        Some(scale_to(wire.scale)),
        Some((wire.position.x, wire.position.y).into()),
    );
}

/// Advertises a plugged output: a `wl_output` global (and its xdg-output) with the name, the
/// make and model, the mode, the scale ceiling and the position. Plugging a key that is
/// already advertised replaces it.
pub fn apply_plug<D>(
    part: &mut OutputPart,
    display: &DisplayHandle,
    wire: &OutputWire,
) -> Result<(), WaylandFault>
where
    D: GlobalDispatch<WlOutput, WlOutputData> + 'static,
{
    if let Some(remove) = part.removers.remove(&wire.key) {
        (remove.0)(display);
    }
    let output = build(wire);
    let global: GlobalId = output.create_global::<D>(display);
    let withdraw = Remover(Box::new(move |display: &DisplayHandle| {
        display.remove_global::<D>(global);
    }));
    part.removers.insert(wire.key.clone(), withdraw);
    part.outputs.insert(wire.key.clone(), output);
    Ok(())
}

/// Sends a changed mode, scale, transform or position to every bound `wl_output` and
/// `xdg_output` (smithay sends each only if it changed, then `done`). An output that is not
/// advertised changes nothing: the connector event that follows its plug carries it.
pub fn apply_change(part: &mut OutputPart, wire: &OutputWire) -> Result<(), WaylandFault> {
    match part.outputs.get(&wire.key) {
        Some(output) => apply_state(output, wire),
        None => {
            tracing::warn!(output = %wire.name.as_str(), "change for an output that is not advertised")
        }
    }
    Ok(())
}

/// Withdraws an unplugged output's global: clients bound to it see `global_remove` and the
/// bound `wl_output` stays valid but inert.
pub fn apply_unplug(
    part: &mut OutputPart,
    display: &DisplayHandle,
    key: &OutputKey,
) -> Result<(), WaylandFault> {
    if let Some(remove) = part.removers.remove(key) {
        (remove.0)(display);
    }
    if let Some(output) = part.outputs.remove(key) {
        output.leave_all();
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use casement_core::geometry::PhysicalSize;
    use casement_core::time::Millihertz;

    fn wire(scale: u32, at: (i32, i32)) -> OutputWire {
        let name = casement_core::OutputName::parse("HEADLESS-1").expect("a name");
        let edid = EdidFacts {
            make: "Test".to_owned(),
            model: "Panel".to_owned(),
            serial: EdidSerial::Known("S1".to_owned()),
        };
        OutputWire {
            key: OutputKey::of(&edid, &name),
            name,
            edid,
            mode: Mode {
                size: PhysicalSize { w: 2880, h: 1620 },
                refresh: Millihertz(60_000),
                preference: ModePreference::Preferred,
            },
            scale: Scale(scale),
            transform: Transform::R90,
            position: LogicalPoint { x: at.0, y: at.1 },
            physical: PhysicalMm { w: 600, h: 340 },
        }
    }

    #[test]
    fn an_output_carries_its_name_mode_scale_and_place() {
        let output = build(&wire(180, (1920, 0)));
        assert_eq!(output.name(), "HEADLESS-1");
        let mode = output.current_mode().expect("a mode");
        assert_eq!(
            (mode.size.w, mode.size.h, mode.refresh),
            (2880, 1620, 60_000)
        );
        assert_eq!(output.preferred_mode(), Some(mode));
        // 1.5 reaches wl_output as its ceiling and everything else exactly.
        assert_eq!(output.current_scale().integer_scale(), 2);
        assert!((output.current_scale().fractional_scale() - 1.5).abs() < f64::EPSILON);
        assert_eq!(output.current_transform(), WireTransform::_90);
        assert_eq!(
            (output.current_location().x, output.current_location().y),
            (1920, 0)
        );
        assert_eq!(output.physical_properties().make, "Test");
    }

    #[test]
    fn whole_scales_advertise_themselves() {
        for (numerator, integer) in [(120, 1), (150, 2), (240, 2), (360, 3)] {
            let output = build(&wire(numerator, (0, 0)));
            assert_eq!(output.current_scale().integer_scale(), integer);
        }
    }

    #[test]
    fn a_change_replaces_state_on_the_same_output() {
        let mut part_outputs = build(&wire(120, (0, 0)));
        apply_state(&part_outputs, &wire(240, (100, 50)));
        assert_eq!(part_outputs.current_scale().integer_scale(), 2);
        assert_eq!(part_outputs.current_location().x, 100);
        part_outputs = build(&wire(120, (0, 0)));
        assert_eq!(part_outputs.current_scale().integer_scale(), 1);
    }

    #[test]
    fn every_transform_maps_to_its_own() {
        use Transform::*;
        let all = [
            Normal, R90, R180, R270, Flipped, Flipped90, Flipped180, Flipped270,
        ];
        let mapped: Vec<_> = all.iter().map(|t| transform_to(*t)).collect();
        for (i, a) in mapped.iter().enumerate() {
            for b in &mapped[i + 1..] {
                assert_ne!(a, b);
            }
        }
    }
}
