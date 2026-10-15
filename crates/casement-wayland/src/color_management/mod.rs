//! `wp_color_manager_v1` (version 2): clients learn what colours an output shows and tell the
//! compositor how their surfaces are encoded.
//!
//! What it serves, and what it does not:
//! - Render intent: perceptual only.
//! - Features: `parametric` and `set_primaries`. No ICC profiles, no luminances, no mastering
//!   volume, no `set_tf_power` and no Windows scRGB/BT.2100 creators (a request for one is
//!   `unsupported_feature`). HDR is out of scope.
//! - Transfer functions: sRGB (`compound_power_2_4`; `srgb` for a version 1 client), gamma 2.2 and
//!   `ext_linear`.
//! - Primaries: named sRGB and Display P3, or any set the client gives. The white point is taken to
//!   be D65 whatever the client says.
//! - An output's image description is its EDID primaries with the gamma 2.2 curve (sRGB when the
//!   panel is not wider than sRGB or its gamut is unknown). A surface's preferred description is
//!   always sRGB: the compositor converts what is tagged otherwise.
//!
//! The tag a surface commits is double buffered (`SurfaceColor`) and is read by the renderer with
//! [`surface_tag`]. The output gamut table is filled by whoever learns the EDID ([`set_gamut`]).

mod creator;
mod manager;
mod named;
mod surface;

pub use creator::{CreatorData, DescriptionData, IccCreatorData, InfoData};
pub use manager::{ManagerData, ManagerGlobal, OutputData};
pub use surface::{FeedbackData, SurfaceData, surface_tag};

use crate::states::GlobalFilter;
use casement_core::OutputKey;
use casement_core::output::{ColorTag, Gamut, TransferCurve};
use smithay::reexports::wayland_protocols::wp::color_management::v1::server::wp_color_manager_v1::WpColorManagerV1;
use smithay::reexports::wayland_server::backend::GlobalId;
use smithay::reexports::wayland_server::protocol::wl_output::WlOutput;
use smithay::reexports::wayland_server::{DisplayHandle, GlobalDispatch};
use std::collections::BTreeMap;

/// The version casement serves.
pub const VERSION: u32 = 2;

/// `wp_color_manager_v1` and what it knows about outputs.
#[derive(Debug, Default)]
pub struct ColorPart {
    /// Our global (none until [`ColorPart::new`]).
    pub global: Option<GlobalId>,
    gamuts: BTreeMap<OutputKey, Gamut>,
    identities: BTreeMap<ColorTag, u64>,
}

impl ColorPart {
    /// Creates the global (version 2), behind the I7 filter.
    pub fn new<D>(display: &DisplayHandle, role_filter: GlobalFilter) -> ColorPart
    where
        D: GlobalDispatch<WpColorManagerV1, ManagerGlobal> + 'static,
    {
        ColorPart {
            global: Some(
                display.create_global::<D, WpColorManagerV1, _>(
                    VERSION,
                    ManagerGlobal::new(role_filter),
                ),
            ),
            ..ColorPart::default()
        }
    }

    /// The gamut of `output` (sRGB until [`set_gamut`] says otherwise).
    pub fn gamut_of(&self, output: &OutputKey) -> Gamut {
        self.gamuts.get(output).copied().unwrap_or(Gamut::Srgb)
    }

    /// The tag that describes what `output` shows.
    pub fn output_tag(&self, output: &OutputKey) -> ColorTag {
        match self.gamut_of(output) {
            Gamut::Srgb => ColorTag::SRGB,
            Gamut::Wide(primaries) => ColorTag {
                primaries,
                transfer: TransferCurve::Gamma22,
            },
        }
    }

    /// The identity number of `tag`: the same for equal tags, never reused, never zero.
    fn identity_of(&mut self, tag: ColorTag) -> u64 {
        let next = u64::try_from(self.identities.len())
            .unwrap_or(u64::MAX)
            .saturating_add(1);
        *self.identities.entry(tag).or_insert(next)
    }
}

/// What the composition root provides to this protocol's dispatch.
pub trait ColorHandler: Sized + 'static {
    /// The part.
    fn color_part(&mut self) -> &mut ColorPart;
    /// The output a client's `wl_output` names.
    fn color_output(&self, output: &WlOutput) -> Option<OutputKey>;
}

/// `output` has this gamut (from its EDID, when it is plugged).
///
/// ```
/// use casement_core::output::{DISPLAY_P3, EdidFacts, EdidSerial, Gamut};
/// use casement_core::{OutputKey, OutputName};
/// use casement_wayland::color_management::{ColorPart, set_gamut};
///
/// let edid = EdidFacts {
///     make: "ACM".to_owned(),
///     model: "Panel".to_owned(),
///     serial: EdidSerial::Unknown,
/// };
/// let mut part = ColorPart::default();
/// let key = OutputKey::of(&edid, &OutputName::parse("DP-1").unwrap());
/// set_gamut(&mut part, key.clone(), Gamut::Wide(DISPLAY_P3));
/// assert_eq!(part.gamut_of(&key), Gamut::Wide(DISPLAY_P3));
/// ```
pub fn set_gamut(part: &mut ColorPart, output: OutputKey, gamut: Gamut) {
    part.gamuts.insert(output, gamut);
}

/// `output` is gone: its gamut is forgotten.
pub fn forget(part: &mut ColorPart, output: &OutputKey) {
    part.gamuts.remove(output);
}

#[cfg(test)]
mod tests {
    use super::*;
    use casement_core::OutputName;
    use casement_core::output::{DISPLAY_P3, EdidFacts, EdidSerial, SRGB};

    fn key(name: &str) -> OutputKey {
        let edid = EdidFacts {
            make: "ACM".to_owned(),
            model: "Panel".to_owned(),
            serial: EdidSerial::Unknown,
        };
        OutputKey::of(&edid, &OutputName::parse(name).expect("name"))
    }

    #[test]
    fn an_unknown_output_is_srgb_and_a_wide_one_is_gamma_22() {
        let mut part = ColorPart::default();
        assert_eq!(part.output_tag(&key("A-1")), ColorTag::SRGB);
        set_gamut(&mut part, key("A-1"), Gamut::Wide(DISPLAY_P3));
        let tag = part.output_tag(&key("A-1"));
        assert_eq!(tag.primaries, DISPLAY_P3);
        assert_eq!(tag.transfer, TransferCurve::Gamma22);
        forget(&mut part, &key("A-1"));
        assert_eq!(part.gamut_of(&key("A-1")), Gamut::Srgb);
    }

    #[test]
    fn identities_are_stable_distinct_and_not_zero() {
        let mut part = ColorPart::default();
        let wide = ColorTag {
            primaries: DISPLAY_P3,
            transfer: TransferCurve::Gamma22,
        };
        let (a, b, again) = (
            part.identity_of(ColorTag::SRGB),
            part.identity_of(wide),
            part.identity_of(ColorTag::SRGB),
        );
        assert_eq!((a, again), (1, 1));
        assert_eq!(b, 2);
        assert_eq!(SRGB, ColorTag::SRGB.primaries);
    }
}
