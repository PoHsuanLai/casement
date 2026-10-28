//! Spec builders shared by the layout tests.

use casement_core::app::AppKey;
use casement_core::geometry::{LogicalPoint, LogicalRect, LogicalSize};
use casement_core::output::{EdidFacts, EdidSerial};
use casement_core::window::SizeLimits;
use casement_core::{OutputKey, OutputName, WindowId};
use casement_layout::{
    OutputSpec, SceneMode, SceneSpec, SpaceArrangement, SpaceSpec, StackIndex, WindowPlacement,
    WindowSpec,
};
use ds_core::geometry::scale::Scale;
use ds_core::vocab::Fraction;
use porter_core::{ClaimedId, SpaceId};

pub fn rect(x: i32, y: i32, w: u32, h: u32) -> LogicalRect {
    LogicalRect::new(LogicalPoint { x, y }, LogicalSize { w, h })
}

pub fn size(w: u32, h: u32) -> LogicalSize {
    LogicalSize { w, h }
}

pub fn output_key(name: &str) -> OutputKey {
    let edid = EdidFacts {
        make: String::new(),
        model: String::new(),
        serial: EdidSerial::Unknown,
    };
    OutputKey::of(&edid, &OutputName::parse(name).expect("name"))
}

pub fn space_id(text: &str) -> SpaceId {
    SpaceId::parse(text).expect("space id")
}

pub const ANY_SIZE: SizeLimits = SizeLimits {
    min: LogicalSize { w: 0, h: 0 },
    max: None,
};

/// An app known by the claimed id `name`.
pub fn app(name: &str) -> AppKey {
    AppKey::Claimed(ClaimedId::parse(name).expect("claimed id"))
}

/// Window `n` of the app `test`.
pub fn window(n: u64, placement: WindowPlacement, limits: SizeLimits) -> WindowSpec {
    WindowSpec {
        window: WindowId(n),
        app: app("test"),
        placement,
        limits,
        stack: StackIndex(u32::try_from(n).expect("small")),
        reserve_top: 0,
    }
}

/// One output `HEADLESS-1` with `usable`, at 1x, showing one desktop Space `s-1` holding
/// `windows`.
pub fn one_desktop(usable: LogicalRect, windows: Vec<WindowSpec>) -> SceneSpec {
    SceneSpec {
        outputs: vec![OutputSpec {
            output: output_key("HEADLESS-1"),
            full: usable,
            covered: usable,
            usable,
            scale: Scale::ONE,
            mode: SceneMode::Desktop {
                shown: space_id("s-1"),
            },
            spaces: vec![SpaceSpec {
                space: space_id("s-1"),
                kind: SpaceArrangement::Desktop,
                windows,
            }],
        }],
    }
}

pub fn knobs() -> casement_layout::LayoutKnobs {
    casement_layout::LayoutKnobs {
        cascade_step: LogicalPoint { x: 28, y: 28 },
        tile_margin: 8,
        spread_gap: 24,
        spread_floor: Fraction(300),
        grip: LogicalSize { w: 112, h: 28 },
    }
}
