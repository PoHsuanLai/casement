//! `wl_output` and `zxdg_output_manager_v1`: one global per plugged output, created and
//! removed as the backend reports connectors (`ConnectorEvent`), the same path for headless
//! and KMS. (Not in the plan's module list; `GlobalKind::Output` needs a home: FINDINGS.)

use crate::states::{GlobalFilter, WaylandFault};
use casement_core::geometry::{LogicalPoint, Transform};
use casement_core::output::{EdidFacts, Mode, PhysicalMm};
use casement_core::{OutputKey, OutputName};
use ds_core::geometry::scale::Scale;
use smithay::output::Output;
use smithay::reexports::wayland_protocols::xdg::xdg_output::zv1::server::zxdg_output_manager_v1::ZxdgOutputManagerV1;
use smithay::reexports::wayland_server::protocol::wl_output::WlOutput;
use smithay::reexports::wayland_server::{DisplayHandle, GlobalDispatch};
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

/// Advertises a plugged output.
pub fn apply_plug<D>(
    part: &mut OutputPart,
    display: &DisplayHandle,
    wire: &OutputWire,
) -> Result<(), WaylandFault>
where
    D: GlobalDispatch<WlOutput, WlOutputData> + 'static,
{
    let _ = (part, display, wire);
    todo!("P0-H1: output::apply_plug")
}

/// Sends a changed mode, scale, transform or position to every bound `wl_output`.
pub fn apply_change(part: &mut OutputPart, wire: &OutputWire) -> Result<(), WaylandFault> {
    let _ = (part, wire);
    todo!("P0-H1: output::apply_change")
}

/// Withdraws an unplugged output's global.
pub fn apply_unplug(
    part: &mut OutputPart,
    display: &DisplayHandle,
    key: &OutputKey,
) -> Result<(), WaylandFault> {
    let _ = (part, display, key);
    todo!("P0-H1: output::apply_unplug")
}
