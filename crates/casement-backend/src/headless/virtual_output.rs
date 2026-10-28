//! A virtual output's spec (C27): what `--outputs` and the test kit's `FixtureSpec` describe.
//!
//! Grammar, one output: `NAME:WxH@MILLIHERTZ:SCALE120:X,Y[:key=value...]`, e.g.
//! `DP-1:3840x2160@60000:180:0,0` (a 4K display at 60 Hz and scale 1.5 at the origin). Keys:
//! `transform=normal|90|180|270|flipped|flipped-90|flipped-180|flipped-270`, `gpu=N`,
//! `vrr=capable|incapable`, `edid=FIXTURE` (a recorded EDID fixture's name; the default is a
//! synthetic EDID whose serial is unknown, so the output's key is its connector).

use crate::backend::BackendError;
use casement_core::OutputName;
use casement_core::geometry::{LogicalPoint, PhysicalSize};
use casement_core::output::{EdidFacts, EdidSerial, Mode, ModePreference, VrrCapability};
use casement_core::time::Millihertz;
use casement_protocols::vocab::Transform;
use ds_core::geometry::scale::Scale;

/// One virtual output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VirtualOutputSpec {
    /// The connector name.
    pub name: OutputName,
    /// The one mode it offers (preferred).
    pub mode: Mode,
    /// Its scale.
    pub scale: Scale,
    /// Its transform.
    pub transform: Transform,
    /// Its top-left in the global logical space.
    pub position: LogicalPoint,
    /// The soft GPU it hangs off (the display GPU in a fake topology).
    pub gpu: GpuId,
    /// Whether it can vary its refresh rate.
    pub vrr: VrrCapability,
    /// Where its EDID comes from.
    pub edid: EdidSource,
}

/// Where a virtual output's EDID comes from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EdidSource {
    /// Made up from these facts.
    Synthetic(EdidFacts),
    /// A recorded fixture (from the user's optional hardware smoke run).
    Recorded(FixtureName),
}

/// A recorded fixture's name (`tests/fixtures/edid/<name>.bin`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FixtureName(pub String);

/// A soft GPU's number in a fake topology (`/sys/dev/char/226:(128+n)`, A3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct GpuId(pub u32);

/// What is wrong with an output spec.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum SpecError {
    /// The first field is not a connector name.
    #[error("the name is not a connector name")]
    Name,
    /// The second field is not `WIDTHxHEIGHT@MILLIHERTZ`.
    #[error("the mode is not WIDTHxHEIGHT@MILLIHERTZ")]
    Mode,
    /// The third field is not a positive scale in 120ths.
    #[error("the scale is not a positive number of 120ths")]
    Scale,
    /// The fourth field is not `X,Y`.
    #[error("the position is not X,Y")]
    Position,
    /// An option after the position is not one of transform, gpu, vrr or edid.
    #[error("the option `{0}` is not understood")]
    UnknownOption(String),
}

impl VirtualOutputSpec {
    /// The spec `text` describes (grammar in the module docs).
    pub fn parse(text: &str) -> Result<VirtualOutputSpec, BackendError> {
        let bad = |why: SpecError| BackendError::BadSpec {
            text: text.to_owned(),
            why,
        };
        let mut fields = text.split(':');
        let name = fields
            .next()
            .and_then(|n| OutputName::parse(n).ok())
            .ok_or_else(|| bad(SpecError::Name))?;
        let mode = fields
            .next()
            .and_then(parse_mode)
            .ok_or_else(|| bad(SpecError::Mode))?;
        let scale = fields
            .next()
            .and_then(|s| s.parse::<u32>().ok())
            .filter(|n| *n > 0)
            .map(Scale)
            .ok_or_else(|| bad(SpecError::Scale))?;
        let position = fields
            .next()
            .and_then(parse_point)
            .ok_or_else(|| bad(SpecError::Position))?;
        let base = VirtualOutputSpec {
            edid: EdidSource::Synthetic(EdidFacts {
                make: "Casement".to_owned(),
                model: "Virtual".to_owned(),
                serial: EdidSerial::Unknown,
            }),
            name,
            mode,
            scale,
            transform: Transform::Normal,
            position,
            gpu: GpuId(0),
            vrr: VrrCapability::Incapable,
        };
        fields.try_fold(base, |spec, field| {
            with_option(spec, field).ok_or_else(|| bad(SpecError::UnknownOption(field.to_owned())))
        })
    }

    /// The spec as text, in the grammar [`VirtualOutputSpec::parse`] reads.
    pub fn text(&self) -> String {
        let mut text = format!(
            "{}:{}x{}@{}:{}:{},{}",
            self.name.as_str(),
            self.mode.size.w,
            self.mode.size.h,
            self.mode.refresh.0,
            self.scale.0,
            self.position.x,
            self.position.y
        );
        if self.transform != Transform::Normal {
            text.push_str(&format!(":transform={}", transform_word(self.transform)));
        }
        if self.gpu != GpuId(0) {
            text.push_str(&format!(":gpu={}", self.gpu.0));
        }
        if self.vrr == VrrCapability::Capable {
            text.push_str(":vrr=capable");
        }
        if let EdidSource::Recorded(fixture) = &self.edid {
            text.push_str(&format!(":edid={}", fixture.0));
        }
        text
    }
}

fn parse_mode(text: &str) -> Option<Mode> {
    let (size, refresh) = text.split_once('@')?;
    let (w, h) = size.split_once('x')?;
    let w = w.parse::<u32>().ok().filter(|w| *w > 0)?;
    let h = h.parse::<u32>().ok().filter(|h| *h > 0)?;
    let refresh = refresh.parse::<u32>().ok().filter(|r| *r > 0)?;
    Some(Mode {
        size: PhysicalSize { w, h },
        refresh: Millihertz(refresh),
        preference: ModePreference::Preferred,
    })
}

fn parse_point(text: &str) -> Option<LogicalPoint> {
    let (x, y) = text.split_once(',')?;
    Some(LogicalPoint {
        x: x.parse().ok()?,
        y: y.parse().ok()?,
    })
}

fn with_option(spec: VirtualOutputSpec, field: &str) -> Option<VirtualOutputSpec> {
    let (key, value) = field.split_once('=')?;
    match key {
        "transform" => Some(VirtualOutputSpec {
            transform: transform_of(value)?,
            ..spec
        }),
        "gpu" => Some(VirtualOutputSpec {
            gpu: GpuId(value.parse().ok()?),
            ..spec
        }),
        "vrr" => Some(VirtualOutputSpec {
            vrr: match value {
                "capable" => VrrCapability::Capable,
                "incapable" => VrrCapability::Incapable,
                _ => return None,
            },
            ..spec
        }),
        "edid" if !value.is_empty() => Some(VirtualOutputSpec {
            edid: EdidSource::Recorded(FixtureName(value.to_owned())),
            ..spec
        }),
        _ => None,
    }
}

const TRANSFORMS: [(Transform, &str); 8] = [
    (Transform::Normal, "normal"),
    (Transform::R90, "90"),
    (Transform::R180, "180"),
    (Transform::R270, "270"),
    (Transform::Flipped, "flipped"),
    (Transform::Flipped90, "flipped-90"),
    (Transform::Flipped180, "flipped-180"),
    (Transform::Flipped270, "flipped-270"),
];

fn transform_of(word: &str) -> Option<Transform> {
    TRANSFORMS.iter().find(|(_, w)| *w == word).map(|(t, _)| *t)
}

fn transform_word(transform: Transform) -> &'static str {
    TRANSFORMS
        .iter()
        .find(|(t, _)| *t == transform)
        .map_or("normal", |(_, w)| w)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_plans_example_parses() {
        let spec = VirtualOutputSpec::parse("DP-1:3840x2160@60000:180:0,0").expect("parses");
        assert_eq!(spec.name.as_str(), "DP-1");
        assert_eq!(spec.mode.size, PhysicalSize { w: 3840, h: 2160 });
        assert_eq!(spec.mode.refresh, Millihertz(60_000));
        assert_eq!(spec.scale, Scale(180));
        assert_eq!(spec.position, LogicalPoint { x: 0, y: 0 });
        assert_eq!(spec.transform, Transform::Normal);
        assert_eq!(spec.gpu, GpuId(0));
        assert_eq!(spec.vrr, VrrCapability::Incapable);
        assert!(matches!(
            spec.edid,
            EdidSource::Synthetic(EdidFacts {
                serial: EdidSerial::Unknown,
                ..
            })
        ));
    }

    #[test]
    fn options_parse_and_every_spec_round_trips_through_its_text() {
        let texts = [
            "HEADLESS-1:1920x1080@60000:120:0,0",
            "HEADLESS-2:2560x1440@144000:150:-2560,0:transform=90:gpu=1:vrr=capable",
            "eDP-1:2880x1800@120000:240:0,1080:transform=flipped-270:edid=framework-13",
        ];
        for text in texts {
            let spec = VirtualOutputSpec::parse(text).expect(text);
            assert_eq!(spec.text(), text);
            assert_eq!(VirtualOutputSpec::parse(&spec.text()).expect("again"), spec);
        }
    }

    #[test]
    fn malformed_specs_are_refused() {
        let bad = [
            "",
            "DP-1",
            "DP-1:3840x2160@60000:180",
            "DP 1:3840x2160@60000:180:0,0",
            "DP-1:0x2160@60000:180:0,0",
            "DP-1:3840x2160@0:180:0,0",
            "DP-1:3840x2160@60000:0:0,0",
            "DP-1:3840x2160@60000:180:0;0",
            "DP-1:3840x2160@60000:180:0,0:transform=45",
            "DP-1:3840x2160@60000:180:0,0:colour=red",
            "DP-1:3840x2160@60000:180:0,0:edid=",
        ];
        for text in bad {
            assert!(
                matches!(
                    VirtualOutputSpec::parse(text),
                    Err(BackendError::BadSpec { .. })
                ),
                "{text}"
            );
        }
    }
}
