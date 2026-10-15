//! T0: whatever the apps, the outputs, the layers, the locker and the person do, in any order,
//! the model keeps every invariant, and the same events give the same model.

use crate::support;

use casement_core::app::AppKey;
use casement_core::focus::{HitPart, Press, TokenCheck};
use casement_core::geometry::{LogicalPoint, LogicalSize};
use casement_core::input::{DesktopCommand, DeviceClass, Origin, SpaceStep};
use casement_core::layer::{
    Anchor, Anchors, ExclusiveZone, KeyboardInteractivity, LayerLevel, LayerSpec, Margins,
    ShellRoleTag,
};
use casement_core::lock::LockEvent;
use casement_core::settings::{CasementSettings, Enabled};
use casement_core::time::TimerKey;
use casement_core::window::{FirstMouse, MaxSize, Parent, SizeLimits, WindowTitle};
use casement_core::{
    ClientKey, Event, KeyboardTarget, LayerKey, MappedToplevel, ResizeEdges, ToplevelMeta,
    ToplevelRequest, WindowHintChange, WindowId,
};
use casement_protocols::vocab::{ActorKind, MacMods, Modality, PlaceTarget, Toggle, WindowRole};
use proptest::prelude::*;
use support::driver::{Op, run};
use support::{claimed, headless, proven};

const OUTPUTS: [&str; 2] = ["HEADLESS-1", "HEADLESS-2"];

fn app() -> impl Strategy<Value = AppKey> {
    prop_oneof![
        Just(proven("org.example.A")),
        Just(proven("org.example.B")),
        Just(AppKey::Claimed(claimed("c")))
    ]
}

fn token() -> impl Strategy<Value = TokenCheck> {
    prop_oneof![
        Just(TokenCheck::Valid),
        Just(TokenCheck::Stale),
        Just(TokenCheck::Missing)
    ]
}

/// Few windows, so the same one is asked for things again and again.
fn window() -> impl Strategy<Value = u64> {
    1u64..5
}

fn output() -> impl Strategy<Value = &'static str> {
    prop_oneof![Just(OUTPUTS[0]), Just(OUTPUTS[1])]
}

fn role() -> impl Strategy<Value = WindowRole> {
    prop_oneof![
        Just(WindowRole::Document),
        Just(WindowRole::Utility),
        Just(WindowRole::Dialog),
        Just(WindowRole::Shell)
    ]
}

fn toggle() -> impl Strategy<Value = Toggle> {
    prop_oneof![Just(Toggle::On), Just(Toggle::Off)]
}

fn place_target() -> impl Strategy<Value = PlaceTarget> {
    prop_oneof![
        Just(PlaceTarget::Left),
        Just(PlaceTarget::TopRight),
        Just(PlaceTarget::Fill),
        Just(PlaceTarget::Centre),
        Just(PlaceTarget::Restore)
    ]
}

fn mapping() -> impl Strategy<Value = Op> {
    (
        window(),
        app(),
        token(),
        role(),
        proptest::option::of(window()),
        0u8..3,
    )
        .prop_map(|(n, app, token, role, parent, modality)| {
            Op::Event(Event::ToplevelMapped(MappedToplevel {
                role,
                parent: parent.map_or(Parent::None, |p| Parent::Window(WindowId(p))),
                modality: match modality {
                    0 => Modality::None,
                    1 => Modality::WindowModal,
                    _ => Modality::Modeless,
                },
                ..support::driver::mapped(n, &app, token)
            }))
        })
}

fn toplevel_request() -> impl Strategy<Value = ToplevelRequest> {
    prop_oneof![
        Just(ToplevelRequest::Move {
            serial: casement_core::InputSerial(1)
        }),
        Just(ToplevelRequest::Resize {
            serial: casement_core::InputSerial(2),
            edges: ResizeEdges::BottomRight
        }),
        toggle().prop_map(ToplevelRequest::Maximise),
        Just(ToplevelRequest::Minimise),
        (toggle(), proptest::option::of(output())).prop_map(|(toggle, name)| {
            ToplevelRequest::FullScreen {
                toggle,
                output: name.map(|name| headless(name).key),
            }
        }),
        token().prop_map(ToplevelRequest::Activate),
        Just(ToplevelRequest::WindowMenu {
            at: LogicalPoint { x: 1, y: 2 }
        }),
        place_target().prop_map(|target| ToplevelRequest::Place {
            target,
            serial: casement_core::InputSerial(3)
        }),
    ]
}

fn layer_spec() -> impl Strategy<Value = LayerSpec> {
    (
        output(),
        prop_oneof![
            Just(LayerLevel::Background),
            Just(LayerLevel::Bottom),
            Just(LayerLevel::Top),
            Just(LayerLevel::Overlay)
        ],
        0u8..4,
        0u32..80,
        prop_oneof![
            Just(KeyboardInteractivity::None),
            Just(KeyboardInteractivity::OnDemand),
            Just(KeyboardInteractivity::Exclusive)
        ],
    )
        .prop_map(|(output, level, edge, zone, keyboard)| {
            let held = |on: bool| if on { Anchor::Held } else { Anchor::Free };
            let [top, bottom, left, right] = match edge {
                0 => [true, false, true, true],
                1 => [false, true, true, true],
                2 => [true, true, true, false],
                _ => [true, true, false, true],
            };
            LayerSpec {
                output: headless(output).key,
                level,
                anchor: Anchors {
                    top: held(top),
                    bottom: held(bottom),
                    left: held(left),
                    right: held(right),
                },
                exclusive: if zone == 0 {
                    ExclusiveZone::Ignore
                } else {
                    ExclusiveZone::Reserve(zone)
                },
                margin: Margins {
                    top: 0,
                    right: 0,
                    bottom: 0,
                    left: 0,
                },
                keyboard,
                namespace: "ns".to_owned(),
                size: casement_core::geometry::LogicalSize::ZERO,
                role: ShellRoleTag::Unassigned,
            }
        })
}

fn lock_event() -> impl Strategy<Value = LockEvent> {
    prop_oneof![
        (1u64..4).prop_map(|c| LockEvent::Requested {
            client: ClientKey(c)
        }),
        output().prop_map(|name| LockEvent::SurfaceCommitted {
            output: headless(name).key
        }),
        Just(LockEvent::Unlocked),
        Just(LockEvent::LockerGone),
    ]
}

fn command() -> impl Strategy<Value = DesktopCommand> {
    prop_oneof![
        Just(DesktopCommand::CloseKey),
        Just(DesktopCommand::MinimiseKey),
        Just(DesktopCommand::HideActive),
        Just(DesktopCommand::HideOthers),
        Just(DesktopCommand::QuitActive),
        Just(DesktopCommand::ForceQuitPanel),
        Just(DesktopCommand::MissionControl),
        Just(DesktopCommand::SwitchSpace(SpaceStep::Next)),
        Just(DesktopCommand::SwitchSpace(SpaceStep::Previous)),
        (1u8..4).prop_map(|n| DesktopCommand::SwitchSpace(SpaceStep::Number(n))),
        place_target().prop_map(DesktopCommand::Place),
    ]
}

fn settings() -> impl Strategy<Value = CasementSettings> {
    prop_oneof![Just(Enabled::On), Just(Enabled::Off)].prop_map(|follow| {
        let mut settings = CasementSettings::defaults();
        settings.spaces.switch_on_activate = follow;
        settings
    })
}

fn op() -> impl Strategy<Value = Op> {
    let event = prop_oneof![
        output().prop_map(|name| Event::OutputPlugged(headless(name))),
        output().prop_map(|name| Event::OutputUnplugged(headless(name).key)),
        (window(), 1u32..2000, 1u32..2000).prop_map(|(n, w, h)| Event::ToplevelCommitted {
            window: WindowId(n),
            size: LogicalSize { w, h }
        }),
        window().prop_map(|n| Event::ToplevelUnmapped {
            window: WindowId(n)
        }),
        window().prop_map(|n| Event::ClientGone {
            client: ClientKey(n)
        }),
        window().prop_map(|n| Event::ClientConnected {
            client: ClientKey(n),
            peer: porter_core::PeerIdentity::Unproven,
            role: casement_core::policy::ConnectionRole::Public,
        }),
        (window(), window()).prop_map(|(n, parent)| Event::ToplevelMeta {
            window: WindowId(n),
            meta: ToplevelMeta::Parent(Parent::Window(WindowId(parent)))
        }),
        (window(), 0u32..200).prop_map(|(n, min)| Event::ToplevelMeta {
            window: WindowId(n),
            meta: ToplevelMeta::Limits(SizeLimits {
                min: LogicalSize { w: min, h: min },
                max: MaxSize::Unbounded
            })
        }),
        (window(), role()).prop_map(|(n, role)| Event::WindowHint {
            window: WindowId(n),
            hint: WindowHintChange::Role(role)
        }),
        window().prop_map(|n| Event::ToplevelMeta {
            window: WindowId(n),
            meta: ToplevelMeta::Title(WindowTitle::default())
        }),
        (1u64..4, layer_spec()).prop_map(|(k, spec)| Event::LayerMapped {
            layer: LayerKey(k),
            spec
        }),
        (1u64..4, layer_spec()).prop_map(|(k, spec)| Event::LayerChanged {
            layer: LayerKey(k),
            spec
        }),
        (1u64..4).prop_map(|k| Event::LayerUnmapped { layer: LayerKey(k) }),
        lock_event().prop_map(Event::Lock),
        window().prop_map(|n| Event::TimerFired(TimerKey::Ping(ClientKey(n)))),
        output().prop_map(|name| Event::TimerFired(TimerKey::SpaceFinish(headless(name).key))),
        window().prop_map(|n| Event::Pong {
            client: ClientKey(n)
        }),
        settings().prop_map(|settings| Event::SettingsChanged(Box::new(settings))),
        Just(Event::Input(casement_core::input::InputEvent {
            origin: Origin::Test,
            device: casement_core::DeviceId(1),
            at: support::ms(1),
            kind: casement_core::input::InputKind::Key {
                code: casement_core::input::KeyCode(30),
                edge: casement_core::input::Edge::Press,
            },
        })),
        window().prop_map(|n| Event::ToplevelMeta {
            window: WindowId(n),
            meta: ToplevelMeta::ClaimedId(claimed("again"))
        }),
        window().prop_map(|n| Event::WindowHint {
            window: WindowId(n),
            hint: WindowHintChange::FirstMouse(FirstMouse::Nowhere)
        }),
    ];
    prop_oneof![
        3 => mapping(),
        4 => (window(), toplevel_request()).prop_map(|(n, request)| Op::Event(Event::ToplevelRequest {
            window: WindowId(n),
            request
        })),
        8 => event.prop_map(Op::Event),
        2 => (window(), prop_oneof![Just(HitPart::Content), Just(HitPart::Titlebar)], prop_oneof![Just(MacMods::NONE), Just(MacMods::COMMAND)])
            .prop_map(|(n, part, mods)| Op::Press(Press {
                window: WindowId(n),
                part,
                mods,
                origin: Origin::Physical(DeviceClass::Pointer),
            })),
        2 => (app(), prop_oneof![Just(ActorKind::User), Just(ActorKind::App), Just(ActorKind::System)])
            .prop_map(|(app, by)| Op::Activate(app, by)),
        2 => command().prop_map(Op::Command),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(3000))]

    /// Every invariant holds after every step of any run, and a run is deterministic.
    #[test]
    fn random_runs_keep_every_invariant(ops in proptest::collection::vec(op(), 0..60)) {
        let first = run(ops.clone());
        let again = run(ops);
        prop_assert_eq!(first.model, again.model, "step is deterministic");
        prop_assert_eq!(first.effects, again.effects, "so are its effects");
    }

    /// The keyboard effect is sent exactly when the party holding the keyboard changed, and
    /// names the party the model says holds it.
    #[test]
    fn the_keyboard_effect_follows_who_holds_it(ops in proptest::collection::vec(op(), 0..40)) {
        let mut model = support::driver::fresh();
        for op in ops {
            let before = model.keyboard_target();
            let (next, effects) = support::driver::apply(model, op);
            let sent: Vec<&KeyboardTarget> = effects
                .iter()
                .filter_map(|e| match e {
                    casement_core::Effect::Keyboard(target) => Some(target),
                    _ => None,
                })
                .collect();
            let after = next.keyboard_target();
            if before == after {
                prop_assert!(sent.is_empty(), "unchanged but sent {sent:?}");
            } else {
                prop_assert_eq!(sent, vec![&after]);
            }
            model = next;
        }
    }
}
