//! Builders shared by casement-core's tests (not API).

pub mod driver;

use casement_core::app::AppKey;
use casement_core::geometry::{LogicalPoint, LogicalRect, LogicalSize, PhysicalSize};
use casement_core::output::{
    EdidFacts, EdidSerial, Mode, ModePreference, OutputFacts, PhysicalMm, VrrCapability,
};
use casement_core::space::{Space, SpaceContent, SpaceIndex};
use casement_core::time::{Millihertz, Mono};
use casement_core::window::{
    MaxSize, Parent, SizeLimits, SpacePlace, StackRank, Window, WindowHints, WindowState,
    WindowTitle,
};
use casement_core::{ClientKey, OutputKey, OutputName, WindowId};
use casement_protocols::vocab::{Emphasis, Modality, SensitivityLevel, WindowRole};
use porter_core::{AppId, AppName, ClaimedId, Isolation, SpaceId};
use std::time::Duration;

pub fn ms(n: u64) -> Mono {
    Mono(Duration::from_millis(n))
}

pub fn rect(x: i32, y: i32, w: u32, h: u32) -> LogicalRect {
    LogicalRect::new(LogicalPoint { x, y }, LogicalSize { w, h })
}

pub fn proven(name: &str) -> AppKey {
    AppKey::Proven(AppId {
        name: AppName::parse(name).expect("app name"),
        isolation: Isolation::Unsandboxed,
    })
}

pub fn claimed(name: &str) -> ClaimedId {
    ClaimedId::parse(name).expect("claimed id")
}

pub fn space_id(text: &str) -> SpaceId {
    SpaceId::parse(text).expect("space id")
}

pub fn mode(w: u32, h: u32, millihertz: u32, preference: ModePreference) -> Mode {
    Mode {
        size: PhysicalSize { w, h },
        refresh: Millihertz(millihertz),
        preference,
    }
}

/// A display on connector `name` with `modes` over `mm` millimetres, no EDID serial.
pub fn display(name: &str, modes: Vec<Mode>, mm: (u32, u32)) -> OutputFacts {
    let edid = EdidFacts {
        make: "Test".to_owned(),
        model: name.to_owned(),
        serial: EdidSerial::Unknown,
    };
    let name = OutputName::parse(name).expect("output name");
    OutputFacts {
        key: OutputKey::of(&edid, &name),
        name,
        edid,
        modes,
        physical: PhysicalMm { w: mm.0, h: mm.1 },
        vrr: VrrCapability::Incapable,
    }
}

/// A headless 1920x1080 60 Hz output.
pub fn headless(name: &str) -> OutputFacts {
    display(
        name,
        vec![mode(1920, 1080, 60_000, ModePreference::Preferred)],
        (0, 0),
    )
}

pub fn desktop(id: &str, output: &OutputKey, position: u32) -> Space {
    Space {
        id: space_id(id),
        output: output.clone(),
        content: SpaceContent::Desktop,
        position: SpaceIndex(position),
    }
}

/// A floating document window `n` of `app` on Space `space`.
pub fn window(n: u64, app: &AppKey, space: &str) -> Window {
    Window {
        id: WindowId(n),
        client: ClientKey(n),
        app: app.clone(),
        role: WindowRole::Document,
        modality: Modality::None,
        place: SpacePlace::On(space_id(space)),
        home: None,
        stack: StackRank(n),
        state: WindowState::Floating(rect(100, 100, 800, 600)),
        parent: Parent::None,
        emphasis: Emphasis::Inactive,
        sensitivity: SensitivityLevel::Normal,
        limits: SizeLimits {
            min: LogicalSize { w: 0, h: 0 },
            max: MaxSize::Unbounded,
        },
        title: WindowTitle::default(),
        claimed_id: claimed("test"),
        hints: WindowHints::unset(),
    }
}
