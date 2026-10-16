//! Each invariant can fail: a healthy model is corrupted one way per row, and the check names
//! exactly that break. (The public API cannot build a broken model, which is the point of
//! the invariants; these corrupt the private fields.)

use super::*;
use crate::app::AppVisibility;
use crate::focus::TokenCheck;
use crate::geometry::{LogicalSize, PhysicalSize};
use crate::ids::OutputName;
use crate::layer::{
    Anchor, Anchors, ExclusiveZone, KeyboardInteractivity, LayerLevel, LayerSpec, Margins,
    ShellRoleTag,
};
use crate::lock::LockEvent;
use crate::output::{EdidFacts, EdidSerial, Mode, ModePreference, PhysicalMm, VrrCapability};
use crate::space::{Space, SpaceContent, SpaceIndex, SplitPair};
use crate::time::Millihertz;
use crate::window::{
    MaxSize, Parent, SizeLimits, SpacePlace, StackRank, WindowOrigin, WindowState,
};
use crate::{LayerKey, MappedToplevel};
use casement_protocols::vocab::{Emphasis, Modality, WindowRole};
use ds_core::vocab::Fraction;
use porter_core::{AppId, AppName, ClaimedId, Isolation, SpaceId};
use std::time::Duration;

fn at(ms: u64) -> Mono {
    Mono(Duration::from_millis(ms))
}

fn app(name: &str) -> AppKey {
    AppKey::Proven(AppId {
        name: AppName::parse(name).expect("app name"),
        isolation: Isolation::Unsandboxed,
    })
}

fn facts(name: &str) -> OutputFacts {
    let edid = EdidFacts {
        make: String::new(),
        model: String::new(),
        serial: EdidSerial::Unknown,
    };
    let name = OutputName::parse(name).expect("name");
    OutputFacts {
        key: OutputKey::of(&edid, &name),
        name,
        edid,
        modes: vec![Mode {
            size: PhysicalSize { w: 1920, h: 1080 },
            refresh: Millihertz(60_000),
            preference: ModePreference::Preferred,
        }],
        physical: PhysicalMm { w: 0, h: 0 },
        vrr: VrrCapability::Incapable,
    }
}

fn key(name: &str) -> OutputKey {
    facts(name).key
}

fn toplevel(n: u64, app: &AppKey) -> Event {
    Event::ToplevelMapped(MappedToplevel {
        window: WindowId(n),
        client: ClientKey(n),
        app: app.clone(),
        origin: WindowOrigin::Wayland,
        claimed_id: ClaimedId::parse("test").expect("claimed"),
        role: WindowRole::Document,
        modality: Modality::None,
        parent: Parent::None,
        limits: SizeLimits {
            min: LogicalSize { w: 0, h: 0 },
            max: MaxSize::Unbounded,
        },
        first_size: LogicalSize { w: 800, h: 600 },
        activation: TokenCheck::Valid,
    })
}

/// Two outputs, windows 1 and 2 of A and 3 of B, a layer on the first output and the session
/// locked with a surface on it. Window 3 is key.
fn healthy() -> Model {
    let layer = LayerSpec {
        output: key("HEADLESS-1"),
        level: LayerLevel::Top,
        anchor: Anchors {
            top: Anchor::Held,
            bottom: Anchor::Free,
            left: Anchor::Held,
            right: Anchor::Held,
        },
        exclusive: ExclusiveZone::Reserve(28),
        margin: Margins {
            top: 0,
            right: 0,
            bottom: 0,
            left: 0,
        },
        keyboard: KeyboardInteractivity::None,
        namespace: "bar".to_owned(),
        size: crate::geometry::LogicalSize::ZERO,
        role: ShellRoleTag::Unassigned,
    };
    let events = vec![
        Event::OutputPlugged(facts("HEADLESS-1")),
        Event::OutputPlugged(facts("HEADLESS-2")),
        toplevel(1, &app("org.example.A")),
        toplevel(2, &app("org.example.A")),
        toplevel(3, &app("org.example.B")),
        Event::LayerMapped {
            layer: LayerKey(1),
            spec: layer,
        },
        Event::Lock(LockEvent::Requested {
            client: ClientKey(9),
        }),
        Event::Lock(LockEvent::SurfaceCommitted {
            output: key("HEADLESS-1"),
        }),
    ];
    events.into_iter().enumerate().fold(
        Model::new(
            CasementSettings::defaults(),
            at(0),
            SpaceSeed::from_entropy([3; 16]),
        ),
        |model, (n, event)| model.step(event, at(n as u64 + 1)).0,
    )
}

fn space_id(text: &str) -> SpaceId {
    SpaceId::parse(text).expect("space id")
}

fn a() -> AppKey {
    app("org.example.A")
}

/// A Space `id` of `output` at `position` holding `content`.
fn space(id: &str, output: &str, position: u32, content: SpaceContent) -> Space {
    Space {
        id: space_id(id),
        output: key(output),
        content,
        position: SpaceIndex(position),
    }
}

#[test]
fn the_healthy_model_breaks_nothing() {
    assert_eq!(healthy().check_invariants(), Ok(()));
}

#[test]
fn every_invariant_can_fail() {
    type Corrupt = fn(&mut Model);
    let cases: Vec<(&str, Corrupt, InvariantBreak)> = vec![
        (
            "a window on a Space that does not exist",
            |m| {
                if let Some(w) = m.windows.get_mut(&WindowId(1)) {
                    w.place = SpacePlace::On(space_id("s-nowhere"));
                }
            },
            InvariantBreak::WindowWithoutSpace(WindowId(1)),
        ),
        (
            "an every-Space window whose home is not plugged",
            |m| {
                if let Some(w) = m.windows.get_mut(&WindowId(1)) {
                    w.place = SpacePlace::AllSpaces;
                    w.home = Some(key("HEADLESS-9"));
                }
            },
            InvariantBreak::HomeOutputMissing(WindowId(1)),
        ),
        (
            "an every-Space window with no home while outputs are plugged",
            |m| {
                if let Some(w) = m.windows.get_mut(&WindowId(1)) {
                    w.place = SpacePlace::AllSpaces;
                    w.home = None;
                }
            },
            InvariantBreak::HomeOutputMissing(WindowId(1)),
        ),
        (
            "a Space on an output that is not plugged",
            |m| {
                let stray = space("s-stray", "HEADLESS-9", 0, SpaceContent::Desktop);
                m.spaces = m.spaces.clone().with_space(stray);
            },
            InvariantBreak::SpaceOnMissingOutput(space_id("s-stray")),
        ),
        (
            "two Spaces of one output at one position",
            |m| {
                let twin = space("s-twin", "HEADLESS-1", 0, SpaceContent::Desktop);
                m.spaces = m.spaces.clone().with_space(twin);
            },
            InvariantBreak::DuplicateSpaceIndex(key("HEADLESS-1")),
        ),
        (
            "an output showing nothing",
            |m| {
                m.spaces = m.spaces.clone().without_shown(&key("HEADLESS-2"));
            },
            InvariantBreak::OutputShowsNothing(key("HEADLESS-2")),
        ),
        (
            "an output showing another output's Space",
            |m| {
                let other = m.spaces.shown_on(&key("HEADLESS-2")).map(|s| s.id.clone());
                if let Some(other) = other {
                    m.spaces = m.spaces.clone().with_shown(key("HEADLESS-1"), other);
                }
            },
            InvariantBreak::OutputShowsNothing(key("HEADLESS-1")),
        ),
        (
            "a full-screen Space whose window is not full screen",
            |m| {
                let fs = space(
                    "s-fs",
                    "HEADLESS-1",
                    1,
                    SpaceContent::FullScreen(WindowId(1)),
                );
                m.spaces = m.spaces.clone().with_space(fs);
            },
            InvariantBreak::FullScreenOutOfStep(space_id("s-fs")),
        ),
        (
            "a Split View naming a window that is not mapped",
            |m| {
                let pair = SplitPair {
                    leading: WindowId(1),
                    trailing: WindowId(77),
                    divider: Fraction(500),
                };
                let split = space("s-split", "HEADLESS-1", 1, SpaceContent::SplitView(pair));
                m.spaces = m.spaces.clone().with_space(split);
            },
            InvariantBreak::SplitViewWindowMissing(space_id("s-split")),
        ),
        (
            "a window whose app has no record",
            |m| {
                m.apps.remove(&app("org.example.B"));
            },
            InvariantBreak::AppWithoutKey(WindowId(3)),
        ),
        (
            "an app whose order lists a window twice",
            |m| {
                if let Some(record) = m.apps.get_mut(&a()) {
                    record.windows.push(WindowId(1));
                }
            },
            InvariantBreak::AppWindowsOutOfStep(a()),
        ),
        (
            "an app whose order misses a window",
            |m| {
                if let Some(record) = m.apps.get_mut(&a()) {
                    record.windows.pop();
                }
            },
            InvariantBreak::AppWindowsOutOfStep(a()),
        ),
        (
            "a window whose parent is not mapped",
            |m| {
                if let Some(w) = m.windows.get_mut(&WindowId(1)) {
                    w.parent = Parent::Window(WindowId(88));
                }
            },
            InvariantBreak::ParentMissing(WindowId(1)),
        ),
        (
            "two windows at one place in the stack",
            |m| {
                if let Some(w) = m.windows.get_mut(&WindowId(2)) {
                    w.stack = StackRank(0);
                }
            },
            InvariantBreak::DuplicateStackRank(WindowId(2)),
        ),
        (
            "a key window that is not mapped",
            |m| m.focus.key = Some(WindowId(99)),
            InvariantBreak::KeyWindowNotMapped(WindowId(99)),
        ),
        (
            "a main window that is not mapped",
            |m| m.focus.main = Some(WindowId(99)),
            InvariantBreak::MainWindowNotMapped(WindowId(99)),
        ),
        (
            "a key window outside the active app",
            |m| m.focus.active = Some(a()),
            InvariantBreak::KeyOutsideActiveApp(WindowId(3)),
        ),
        (
            "a key window in the dock",
            |m| {
                if let Some(w) = m.windows.get_mut(&WindowId(3)) {
                    w.state = WindowState::Minimised {
                        restore: crate::window::Restore::New(LogicalSize { w: 1, h: 1 }),
                    };
                }
            },
            InvariantBreak::KeyWindowNotVisible(WindowId(3)),
        ),
        (
            "an emphasis that does not follow focus",
            |m| {
                if let Some(w) = m.windows.get_mut(&WindowId(1)) {
                    w.emphasis = Emphasis::Key;
                }
            },
            InvariantBreak::EmphasisOutOfStep(WindowId(1)),
        ),
        (
            "a hidden active app",
            |m| {
                let ghost = app("org.example.Ghost");
                m.focus.active = Some(ghost.clone());
                m.focus.key = None;
                m.focus.main = None;
                for w in m.windows.values_mut() {
                    w.emphasis = Emphasis::Inactive;
                }
                let record = crate::app::App {
                    key: ghost.clone(),
                    pids: crate::app::PidSet::default(),
                    windows: Vec::new(),
                    life: crate::app::AppLife::Windowless,
                    visibility: AppVisibility::Hidden {
                        restore_order: Vec::new(),
                    },
                    attention: casement_protocols::vocab::Attention::None,
                };
                m.apps.insert(ghost, record);
            },
            InvariantBreak::FocusOnHiddenApp(app("org.example.Ghost")),
        ),
        (
            "a layer on an output that is not plugged",
            |m| {
                let mut stray = m.layers.get(&LayerKey(1)).cloned().expect("layer");
                stray.output = key("HEADLESS-9");
                m.layers = m.layers.clone().with_layer(LayerKey(2), stray);
            },
            InvariantBreak::LayerOnMissingOutput(LayerKey(2)),
        ),
        (
            "a lock surface covering an output that is not plugged",
            |m| {
                m.lock = LockState::Locked {
                    surfaces: crate::lock::LockSurfaces::Covering(vec![key("HEADLESS-9")]),
                };
            },
            InvariantBreak::LockCoversMissingOutput(key("HEADLESS-9")),
        ),
    ];
    for (name, corrupt, want) in cases {
        let mut model = healthy();
        corrupt(&mut model);
        assert_eq!(model.check_invariants(), Err(want), "{name}");
    }
}

/// A hand placement (a test's) is remembered until the person drags the window or it unmaps.
#[test]
fn a_hand_placement_lasts_until_a_drag_or_an_unmap() {
    use crate::geometry::{LogicalPoint, LogicalRect, LogicalSize};
    let rect = LogicalRect::new(
        LogicalPoint { x: 64, y: -5 },
        LogicalSize { w: 300, h: 300 },
    );
    let window = WindowId(1);
    let placed = healthy()
        .step(Event::WindowPlaced { window, rect }, at(100))
        .0;
    assert!(placed.is_placed_by_hand(window), "put by hand");
    let dragged = placed
        .clone()
        .step(Event::WindowDragged { window, rect }, at(101))
        .0;
    assert!(!dragged.is_placed_by_hand(window), "a drag is the person's");
    let gone = placed.step(Event::ToplevelUnmapped { window }, at(102)).0;
    assert!(!gone.is_placed_by_hand(window), "forgotten with the window");
}
