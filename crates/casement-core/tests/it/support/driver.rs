//! Drives a model through events and the entry points beside them, checking every invariant
//! after each step (the rule for every row of every table).

use super::{claimed, headless, ms};
use casement_core::app::AppKey;
use casement_core::focus::{HitPart, Press, TokenCheck};
use casement_core::geometry::LogicalSize;
use casement_core::input::{DesktopCommand, DeviceClass, Origin};
use casement_core::settings::CasementSettings;
use casement_core::space::SpaceSeed;
use casement_core::window::{MaxSize, Parent, SizeLimits, WindowOrigin};
use casement_core::{ClientKey, Effect, Event, MappedToplevel, Model, ToplevelRequest, WindowId};
use casement_protocols::vocab::{ActorKind, MacMods, Modality, WindowRole};

/// One thing that happens to the model.
#[derive(Clone)]
pub enum Op {
    /// An event, through `Model::step`.
    Event(Event),
    /// A press on a window, through `Model::press`.
    Press(Press),
    /// An app activated, through `Model::activate_app`.
    Activate(AppKey, ActorKind),
    /// A desktop command, through `Model::command`.
    Command(DesktopCommand),
}

impl std::fmt::Debug for Op {
    /// One line: a failing run's list of ops stays readable.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let full = match self {
            Op::Event(event) => format!("{event:?}"),
            Op::Press(press) => format!("{press:?}"),
            Op::Activate(app, by) => format!("Activate({app:?}, {by:?})"),
            Op::Command(command) => format!("Command({command:?})"),
        };
        let squeezed = full.split_whitespace().collect::<Vec<_>>().join(" ");
        write!(f, "{}", squeezed.chars().take(2000).collect::<String>())
    }
}

impl From<Event> for Op {
    fn from(event: Event) -> Op {
        Op::Event(event)
    }
}

/// An empty desktop with the default settings.
pub fn fresh() -> Model {
    Model::new(
        CasementSettings::defaults(),
        ms(0),
        SpaceSeed::from_entropy([9; 16]),
    )
}

/// `model` after `op`, one millisecond after the model's last step.
pub fn apply(model: Model, op: Op) -> (Model, Vec<Effect>) {
    let at = model.now().after(std::time::Duration::from_millis(1));
    match op {
        Op::Event(event) => model.step(event, at),
        Op::Press(press) => model.press(press, at),
        Op::Activate(app, by) => model.activate_app(app, by, at),
        Op::Command(command) => model.command(command, at),
    }
}

/// What a run of ops produced: the model and each step's effects.
pub struct Run {
    pub model: Model,
    pub effects: Vec<Vec<Effect>>,
}

/// Runs `ops` from `model` in order, asserting the invariants after each.
pub fn run_from(model: Model, ops: Vec<Op>) -> Run {
    ops.into_iter().enumerate().fold(
        Run {
            model,
            effects: Vec::new(),
        },
        |run, (n, op)| {
            let label = format!("{op:?}");
            let (model, out) = apply(run.model, op);
            assert_eq!(model.check_invariants(), Ok(()), "after step {n}: {label}");
            let mut effects = run.effects;
            effects.push(out);
            Run { model, effects }
        },
    )
}

/// Runs `ops` from an empty desktop.
pub fn run(ops: Vec<Op>) -> Run {
    run_from(fresh(), ops)
}

pub fn plug(name: &str) -> Op {
    Op::Event(Event::OutputPlugged(headless(name)))
}

pub fn unplug(name: &str) -> Op {
    Op::Event(Event::OutputUnplugged(headless(name).key))
}

/// Window `n` of `app` mapping with `activation`, a document with no parent.
pub fn map(n: u64, app: &AppKey, activation: TokenCheck) -> Op {
    Op::Event(Event::ToplevelMapped(mapped(n, app, activation)))
}

pub fn mapped(n: u64, app: &AppKey, activation: TokenCheck) -> MappedToplevel {
    MappedToplevel {
        window: WindowId(n),
        client: ClientKey(n),
        app: app.clone(),
        origin: WindowOrigin::Wayland,
        claimed_id: claimed("test"),
        role: WindowRole::Document,
        modality: Modality::None,
        parent: Parent::None,
        limits: SizeLimits {
            min: LogicalSize { w: 0, h: 0 },
            max: MaxSize::Unbounded,
        },
        first_size: LogicalSize { w: 800, h: 600 },
        activation,
    }
}

/// Window `n` mapping as a modal sheet of window `parent`.
pub fn map_sheet(n: u64, app: &AppKey, parent: u64) -> Op {
    Op::Event(Event::ToplevelMapped(MappedToplevel {
        role: WindowRole::Dialog,
        modality: Modality::WindowModal,
        parent: Parent::Window(WindowId(parent)),
        ..mapped(n, app, TokenCheck::Missing)
    }))
}

pub fn unmap(n: u64) -> Op {
    Op::Event(Event::ToplevelUnmapped {
        window: WindowId(n),
    })
}

pub fn request(n: u64, request: ToplevelRequest) -> Op {
    Op::Event(Event::ToplevelRequest {
        window: WindowId(n),
        request,
    })
}

/// A press on window `n`'s content from the person's hands.
pub fn press(n: u64) -> Op {
    Op::Press(Press {
        window: WindowId(n),
        part: HitPart::Content,
        mods: MacMods::NONE,
        origin: Origin::Physical(DeviceClass::Pointer),
    })
}

/// The effects of the last step.
pub fn last(run: &Run) -> &[Effect] {
    run.effects.last().map_or(&[], Vec::as_slice)
}

/// Asserts the two models hold the same world: everything but the instant of the last step.
pub fn assert_same_world(got: &Model, want: &Model, label: &str) {
    assert_eq!(got.windows(), want.windows(), "{label}: windows");
    assert_eq!(got.apps(), want.apps(), "{label}: apps");
    assert_eq!(got.spaces(), want.spaces(), "{label}: spaces");
    assert_eq!(got.outputs(), want.outputs(), "{label}: outputs");
    assert_eq!(got.layers(), want.layers(), "{label}: layers");
    assert_eq!(got.lock(), want.lock(), "{label}: lock");
    assert_eq!(got.focus(), want.focus(), "{label}: focus");
    assert_eq!(got.timers(), want.timers(), "{label}: timers");
    assert_eq!(got.settings(), want.settings(), "{label}: settings");
}
