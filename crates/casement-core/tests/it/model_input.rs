//! Presses, desktop commands, timers, settings and states through the model's entry points.
//! Every row runs with the invariants checked after each step.

use crate::support;

use casement_core::ClientKey;
use casement_core::app::AppKey;
use casement_core::app::{
    AppLife, ForceQuitRow, PING_DEADLINE, QuitPhase, StopSignal, Termination,
};
use casement_core::desktop::{AppAsk, DesktopRequest};
use casement_core::focus::{HitPart, PressRoute, TokenCheck};
use casement_core::input::{DesktopCommand, DeviceClass, Origin};
use casement_core::settings::{CasementSettings, Enabled};
use casement_core::shell::ShellNote;
use casement_core::time::{Due, TimerKey};
use casement_core::window::{WindowOrigin, WindowState};
use casement_core::{CoreNote, Effect, Event, ToplevelRequest, WindowId};
use casement_protocols::vocab::{HeldMods, Toggle};
use support::driver::{Op, Run, assert_same_world, last, map, plug, press, request, run, run_from};
use support::{claimed, proven};

fn two_apps() -> Run {
    let (mail, web) = (proven("org.quire.Mail"), proven("org.example.Web"));
    run(vec![
        plug("HEADLESS-1"),
        map(1, &web, TokenCheck::Valid),
        map(2, &mail, TokenCheck::Valid),
        map(3, &mail, TokenCheck::Valid),
    ])
}

#[test]
fn presses_are_routed_before_any_client_sees_them() {
    let physical = Origin::Physical(DeviceClass::Pointer);
    let on = |n: u64, part: HitPart, mods: HeldMods| {
        Op::Press(casement_core::focus::Press {
            window: WindowId(n),
            part,
            mods,
            origin: physical,
        })
    };
    let cases = [
        (
            "the key window delivers",
            on(3, HitPart::Content, HeldMods::NONE),
            PressRoute::Deliver,
        ),
        (
            "an inactive window's content activates and swallows",
            on(1, HitPart::Content, HeldMods::NONE),
            PressRoute::ActivateRaiseSwallow,
        ),
        (
            "its titlebar activates and delivers",
            on(1, HitPart::Titlebar, HeldMods::NONE),
            PressRoute::ActivateRaiseDeliver,
        ),
        (
            "command-click delivers without activating",
            on(1, HitPart::Content, HeldMods::COMMAND),
            PressRoute::DeliverWithoutActivation,
        ),
        (
            "command-drag on a titlebar moves without activating",
            on(1, HitPart::Titlebar, HeldMods::COMMAND),
            PressRoute::MoveWithoutActivation,
        ),
    ];
    for (name, press, route) in cases {
        let run = run_from(two_apps().model, vec![press]);
        assert!(last(&run).contains(&Effect::Route(route)), "{name}");
        let activated = matches!(
            route,
            PressRoute::ActivateRaiseSwallow | PressRoute::ActivateRaiseDeliver
        );
        assert_eq!(
            run.model.focus().key == Some(WindowId(1)),
            activated,
            "{name}"
        );
    }
}

#[test]
fn a_press_activating_a_window_raises_it_and_moves_the_keyboard() {
    let run = run_from(two_apps().model, vec![press(1)]);
    let effects = last(&run);
    assert!(effects.contains(&Effect::Raise(vec![WindowId(1)])));
    assert!(
        effects.contains(&Effect::Keyboard(casement_core::KeyboardTarget::Window(
            WindowId(1)
        )))
    );
    assert_eq!(
        run.model.windows()[&WindowId(1)].emphasis,
        casement_protocols::vocab::Emphasis::Key
    );
    assert_eq!(
        run.model.windows()[&WindowId(3)].emphasis,
        casement_protocols::vocab::Emphasis::Inactive
    );
}

#[test]
fn a_press_behind_a_sheet_rings_the_bell() {
    let mail = proven("org.quire.Mail");
    let run = run(vec![
        plug("HEADLESS-1"),
        map(1, &mail, TokenCheck::Valid),
        support::driver::map_sheet(2, &mail, 1),
        press(1),
    ]);
    let effects = last(&run);
    assert!(effects.contains(&Effect::Route(PressRoute::SwallowWithBell)));
    assert!(effects.contains(&Effect::Bell { app: mail }));
}

#[test]
fn a_press_on_a_window_nobody_can_see_is_ignored() {
    let mail = proven("org.quire.Mail");
    let hidden = run(vec![
        plug("HEADLESS-1"),
        map(1, &mail, TokenCheck::Valid),
        map(2, &proven("org.example.Web"), TokenCheck::Valid),
        Op::Command(DesktopCommand::HideActive),
    ]);
    let after = run_from(hidden.model.clone(), vec![press(2), press(9)]);
    assert_same_world(&after.model, &hidden.model, "unchanged");
    assert_eq!(last(&after), []);
}

#[test]
fn the_desktop_commands_that_close_minimise_and_place_the_key_window() {
    let run_cmd = |command| run_from(two_apps().model, vec![Op::Command(command)]);
    let close = run_cmd(DesktopCommand::CloseKey);
    assert_eq!(
        last(&close),
        [Effect::SendClose {
            window: WindowId(3)
        }]
    );

    let minimise = run_cmd(DesktopCommand::MinimiseKey);
    assert!(matches!(
        minimise.model.windows()[&WindowId(3)].state,
        WindowState::Minimised { .. }
    ));
    assert_eq!(minimise.model.focus().key, Some(WindowId(2)));

    let place = run_cmd(DesktopCommand::Place(
        casement_protocols::vocab::PlaceTarget::Right,
    ));
    assert!(matches!(
        place.model.windows()[&WindowId(3)].state,
        WindowState::Tiled {
            zone: casement_protocols::vocab::TileZone::Right,
            ..
        }
    ));
}

/// Ctrl-Cmd-F: the key window goes into a Space of its own and comes back, in any app.
#[test]
fn the_full_screen_command_takes_the_key_window_into_its_own_space_and_back() {
    let start = two_apps().model;
    let shown = |model: &casement_core::Model| {
        model
            .spaces()
            .shown()
            .map(|(_, space)| space.clone())
            .collect::<Vec<_>>()
    };
    let full = run_from(
        start.clone(),
        vec![Op::Command(DesktopCommand::FullScreenKey)],
    );
    let WindowState::FullScreen { space, .. } = full.model.windows()[&WindowId(3)].state.clone()
    else {
        panic!("full screen");
    };
    assert!(!shown(&start).contains(&space), "a Space of its own");
    assert!(shown(&full.model).contains(&space), "shown");
    let back = run_from(full.model, vec![Op::Command(DesktopCommand::FullScreenKey)]);
    assert_eq!(
        back.model.windows()[&WindowId(3)].state,
        start.windows()[&WindowId(3)].state,
        "it comes back where it was"
    );
}

#[test]
fn quitting_asks_every_window_of_the_active_app_to_close() {
    let run = run_from(
        two_apps().model,
        vec![Op::Command(DesktopCommand::QuitActive)],
    );
    let closes: Vec<_> = last(&run)
        .iter()
        .filter_map(|e| match e {
            Effect::SendClose { window } => Some(*window),
            _ => None,
        })
        .collect();
    assert_eq!(closes, vec![WindowId(2), WindowId(3)]);
}

#[test]
fn commands_with_nothing_to_act_on_do_nothing() {
    let empty = run(vec![plug("HEADLESS-1")]);
    for command in [
        DesktopCommand::CloseKey,
        DesktopCommand::MinimiseKey,
        DesktopCommand::FullScreenKey,
        DesktopCommand::HideActive,
        DesktopCommand::HideOthers,
        DesktopCommand::QuitActive,
        DesktopCommand::Place(casement_protocols::vocab::PlaceTarget::Fill),
        DesktopCommand::SplitTile(casement_protocols::vocab::SplitSide::Left),
    ] {
        let after = run_from(empty.model.clone(), vec![Op::Command(command)]);
        assert_same_world(&after.model, &empty.model, &format!("{command:?}"));
        assert_eq!(last(&after), [], "{command:?}");
    }
}

#[test]
fn an_unanswered_ping_marks_the_app_not_responding_and_a_pong_clears_it() {
    let mail = proven("org.quire.Mail");
    let base = run(vec![plug("HEADLESS-1"), map(1, &mail, TokenCheck::Valid)]);
    let ping = casement_core::time::TimerKey::Ping(casement_core::ClientKey(1));
    let stuck = run_from(base.model, vec![Op::Event(Event::TimerFired(ping.clone()))]);
    assert!(matches!(
        stuck.model.apps()[&mail].life,
        AppLife::NotResponding { .. }
    ));
    let answered = run_from(
        stuck.model,
        vec![Op::Event(Event::Pong {
            client: casement_core::ClientKey(1),
        })],
    );
    assert_eq!(answered.model.apps()[&mail].life, AppLife::Running);
    assert!(last(&answered).contains(&Effect::CancelTimer(ping)));
}

#[test]
fn a_timer_for_something_gone_is_noted() {
    let base = run(vec![plug("HEADLESS-1")]);
    let ping = casement_core::time::TimerKey::Ping(casement_core::ClientKey(4));
    let after = run_from(
        base.model.clone(),
        vec![Op::Event(Event::TimerFired(ping.clone()))],
    );
    assert_same_world(&after.model, &base.model, "unchanged");
    assert_eq!(last(&after), [Effect::Note(CoreNote::StaleTimer(ping))]);
    let repeat = run_from(
        base.model.clone(),
        vec![Op::Event(Event::TimerFired(
            casement_core::time::TimerKey::SwitcherDelay,
        ))],
    );
    assert_eq!(
        last(&repeat),
        [],
        "a timer the model does not track is the edge's"
    );
}

#[test]
fn settings_replace_and_lay_out_again_only_when_they_differ() {
    let base = run(vec![plug("HEADLESS-1")]);
    let same = run_from(
        base.model.clone(),
        vec![Op::Event(Event::SettingsChanged(Box::new(
            CasementSettings::defaults(),
        )))],
    );
    assert_same_world(&same.model, &base.model, "unchanged");
    assert_eq!(last(&same), []);

    let mut settings = CasementSettings::defaults();
    settings.spaces.switch_on_activate = Enabled::Off;
    let changed = run_from(
        base.model,
        vec![Op::Event(Event::SettingsChanged(Box::new(
            settings.clone(),
        )))],
    );
    assert_eq!(changed.model.settings(), &settings);
    assert!(last(&changed).contains(&Effect::Relayout(casement_core::Cause::Configure)));
}

#[test]
fn states_follow_the_record_and_focus() {
    let run = run_from(
        two_apps().model,
        vec![request(
            3,
            ToplevelRequest::Place {
                target: casement_protocols::vocab::PlaceTarget::TopLeft,
                serial: casement_core::InputSerial(1),
            },
        )],
    );
    let key = run.model.states_of(WindowId(3)).expect("window");
    assert!(key.activated);
    assert_eq!((key.tiled.top, key.tiled.left), (true, true));
    assert_eq!((key.tiled.bottom, key.tiled.right), (false, false));
    assert!(!key.maximised);
    let other = run.model.states_of(WindowId(1)).expect("window");
    assert!(!other.activated);
    assert_eq!(run.model.states_of(WindowId(9)), None);
}

fn typing() -> Op {
    Op::Event(Event::Input(casement_core::input::InputEvent {
        origin: Origin::Test,
        device: casement_core::DeviceId(1),
        at: support::ms(5),
        kind: casement_core::input::InputKind::Key {
            code: casement_core::input::KeyCode(30),
            edge: casement_protocols::vocab::PressEdge::Press,
        },
    }))
}

fn ping_of(run: &Run) -> Vec<Effect> {
    last(run)
        .iter()
        .filter(|e| matches!(e, Effect::Ping { .. }))
        .cloned()
        .collect()
}

#[test]
fn input_pings_the_key_window_once_per_gap_and_times_the_answer() {
    let mail = proven("org.quire.Mail");
    let base = run(vec![plug("HEADLESS-1"), map(1, &mail, TokenCheck::Valid)]);
    let first = run_from(base.model, vec![typing()]);
    let client = ClientKey(1);
    assert_eq!(ping_of(&first), [Effect::Ping { client }]);
    let deadline = Due {
        at: first.model.now().after(PING_DEADLINE),
        key: TimerKey::Ping(client),
    };
    assert!(last(&first).contains(&Effect::ArmTimer(deadline)));
    let again = run_from(first.model, vec![typing()]);
    assert_eq!(ping_of(&again), [], "a ping is out");
    let answered = run_from(
        again.model,
        vec![Op::Event(Event::Pong { client }), typing()],
    );
    assert_eq!(ping_of(&answered), [], "one went a moment ago");
    let later = run_from(
        answered.model,
        vec![
            Op::Event(Event::TimerFired(TimerKey::PingGap(client))),
            typing(),
        ],
    );
    assert_eq!(ping_of(&later), [Effect::Ping { client }]);
}

#[test]
fn a_client_that_misses_the_deadline_is_not_responding_until_it_answers() {
    let mail = proven("org.quire.Mail");
    let base = run(vec![plug("HEADLESS-1"), map(1, &mail, TokenCheck::Valid)]);
    let client = ClientKey(1);
    let missed = run_from(
        base.model,
        vec![
            typing(),
            Op::Event(Event::TimerFired(TimerKey::Ping(client))),
        ],
    );
    assert!(matches!(
        missed.model.apps()[&mail].life,
        AppLife::NotResponding { .. }
    ));
    let still = run_from(
        missed.model,
        vec![
            Op::Event(Event::TimerFired(TimerKey::PingGap(client))),
            typing(),
        ],
    );
    assert_eq!(
        ping_of(&still),
        [],
        "the ping that is out is the answer awaited"
    );
    let back = run_from(still.model, vec![Op::Event(Event::Pong { client })]);
    assert_eq!(back.model.apps()[&mail].life, AppLife::Running);
}

#[test]
fn quitting_shows_the_app_as_quitting_until_the_grace_ends_with_it_still_here() {
    let mail = proven("org.quire.Mail");
    let quit = run_from(
        two_apps().model,
        vec![Op::Command(DesktopCommand::QuitActive)],
    );
    assert!(matches!(
        quit.model.apps()[&mail].life,
        AppLife::Quitting(QuitPhase::Asked { .. })
    ));
    assert!(last(&quit).iter().any(|e| matches!(
        e,
        Effect::ArmTimer(Due { key: TimerKey::QuitGrace(app), .. }) if *app == mail
    )));
    let kept = run_from(
        quit.model,
        vec![Op::Event(Event::TimerFired(TimerKey::QuitGrace(
            mail.clone(),
        )))],
    );
    assert_eq!(kept.model.apps()[&mail].life, AppLife::Running);
}

fn listening(client: u64, toggle: Toggle) -> Op {
    Op::Event(Event::QuitListener {
        client: ClientKey(client),
        toggle,
    })
}

fn quit_asked(app: &AppKey) -> Op {
    Op::Event(Event::Desktop(DesktopRequest::App {
        app: app.clone(),
        ask: AppAsk::Quit,
    }))
}

fn quits_and_closes(run: &Run) -> (Vec<ClientKey>, Vec<WindowId>) {
    let quits = last(run)
        .iter()
        .filter_map(|e| match e {
            Effect::SendQuit { client } => Some(*client),
            _ => None,
        })
        .collect();
    let closes = last(run)
        .iter()
        .filter_map(|e| match e {
            Effect::SendClose { window } => Some(*window),
            _ => None,
        })
        .collect();
    (quits, closes)
}

#[test]
fn an_app_whose_clients_listen_is_sent_quit_and_no_window_is_closed() {
    let mail = proven("org.quire.Mail");
    let quit = run_from(
        two_apps().model,
        vec![
            listening(2, Toggle::On),
            listening(3, Toggle::On),
            Op::Command(DesktopCommand::QuitActive),
        ],
    );
    assert_eq!(
        quits_and_closes(&quit),
        (vec![ClientKey(2), ClientKey(3)], vec![])
    );
    assert!(matches!(
        quit.model.apps()[&mail].life,
        AppLife::Quitting(QuitPhase::Asked { .. })
    ));
    assert!(last(&quit).iter().any(|e| matches!(
        e,
        Effect::ArmTimer(Due { key: TimerKey::QuitGrace(app), .. }) if *app == mail
    )));
    let kept = run_from(
        quit.model,
        vec![Op::Event(Event::TimerFired(TimerKey::QuitGrace(
            mail.clone(),
        )))],
    );
    assert_eq!(kept.model.apps()[&mail].life, AppLife::Running);
}

#[test]
fn a_client_that_does_not_listen_keeps_the_window_by_window_close() {
    let none = run_from(
        two_apps().model,
        vec![Op::Command(DesktopCommand::QuitActive)],
    );
    assert_eq!(
        quits_and_closes(&none),
        (vec![], vec![WindowId(2), WindowId(3)])
    );
    let mixed = run_from(
        two_apps().model,
        vec![
            listening(2, Toggle::On),
            Op::Command(DesktopCommand::QuitActive),
        ],
    );
    assert_eq!(
        quits_and_closes(&mixed),
        (vec![ClientKey(2)], vec![WindowId(3)])
    );
    let let_go = run_from(
        two_apps().model,
        vec![
            listening(2, Toggle::On),
            listening(2, Toggle::Off),
            Op::Command(DesktopCommand::QuitActive),
        ],
    );
    assert_eq!(
        quits_and_closes(&let_go),
        (vec![], vec![WindowId(2), WindowId(3)])
    );
}

#[test]
fn an_app_with_no_window_is_asked_to_quit_when_its_client_listens() {
    let mail = proven("org.quire.Mail");
    let windowless = run(vec![
        plug("HEADLESS-1"),
        map(1, &mail, TokenCheck::Valid),
        listening(1, Toggle::On),
        Op::Event(Event::ToplevelUnmapped {
            window: WindowId(1),
        }),
    ]);
    assert_eq!(windowless.model.apps()[&mail].life, AppLife::Windowless);
    let quit = run_from(windowless.model, vec![quit_asked(&mail)]);
    assert_eq!(quits_and_closes(&quit), (vec![ClientKey(1)], vec![]));
    assert!(matches!(
        quit.model.apps()[&mail].life,
        AppLife::Quitting(QuitPhase::Asked { .. })
    ));
}

#[test]
fn a_listener_that_is_not_connected_is_not_remembered() {
    let mail = proven("org.quire.Mail");
    let base = run(vec![plug("HEADLESS-1"), map(1, &mail, TokenCheck::Valid)]);
    let late = run_from(base.model, vec![listening(9, Toggle::On)]);
    let quit = run_from(late.model, vec![quit_asked(&mail)]);
    assert_eq!(quits_and_closes(&quit), (vec![], vec![WindowId(1)]));
}

fn force_quit(app: &AppKey) -> Op {
    Op::Event(Event::Desktop(DesktopRequest::App {
        app: app.clone(),
        ask: AppAsk::ForceQuit,
    }))
}

#[test]
fn force_quit_stops_only_that_apps_connections_term_then_kill() {
    let (mail, web) = (proven("org.quire.Mail"), proven("org.example.Web"));
    let stopped = run_from(two_apps().model, vec![force_quit(&mail)]);
    let term = Effect::Terminate(Termination {
        app: mail.clone(),
        clients: vec![ClientKey(2), ClientKey(3)],
        x11: Vec::new(),
        signal: StopSignal::Term,
    });
    assert!(last(&stopped).contains(&term));
    assert!(matches!(
        stopped.model.apps()[&mail].life,
        AppLife::Quitting(QuitPhase::Stopping)
    ));
    assert_eq!(stopped.model.apps()[&web].life, AppLife::Running);
    let killed = run_from(
        stopped.model,
        vec![Op::Event(Event::TimerFired(TimerKey::QuitGrace(
            mail.clone(),
        )))],
    );
    assert!(last(&killed).contains(&Effect::Terminate(Termination {
        app: mail.clone(),
        clients: vec![ClientKey(2), ClientKey(3)],
        x11: Vec::new(),
        signal: StopSignal::Kill,
    })));
    let gone = run_from(
        killed.model,
        vec![
            Op::Event(Event::ClientGone {
                client: ClientKey(2),
            }),
            Op::Event(Event::ClientGone {
                client: ClientKey(3),
            }),
        ],
    );
    assert!(!gone.model.apps().contains_key(&mail));
    assert!(gone.model.apps().contains_key(&web));
}

#[test]
fn a_connection_that_carries_two_apps_is_never_force_quit() {
    let (mail, web) = (proven("org.quire.Mail"), proven("org.example.Web"));
    let shared = Op::Event(Event::ToplevelMapped(casement_core::MappedToplevel {
        client: ClientKey(1),
        ..support::driver::mapped(2, &web, TokenCheck::Valid)
    }));
    let base = run(vec![
        plug("HEADLESS-1"),
        map(1, &mail, TokenCheck::Valid),
        shared,
    ]);
    for app in [&mail, &web] {
        let asked = run_from(base.model.clone(), vec![force_quit(app)]);
        assert!(
            !last(&asked)
                .iter()
                .any(|e| matches!(e, Effect::Terminate(_))),
            "{app:?}"
        );
    }
}

/// The satellite's connection (client 1) carrying window 1 of `xterm` and window 2 of `gimp`,
/// and window 3 of a Wayland app on a connection of its own.
fn two_x11_apps_and_a_wayland_app() -> (Run, AppKey, AppKey, AppKey) {
    let (xterm, gimp) = (
        AppKey::Claimed(claimed("XTerm")),
        AppKey::Claimed(claimed("Gimp")),
    );
    let web = proven("org.example.Web");
    let x11 = |n: u64, app: &AppKey| {
        Op::Event(Event::ToplevelMapped(casement_core::MappedToplevel {
            client: ClientKey(1),
            origin: WindowOrigin::X11,
            ..support::driver::mapped(n, app, TokenCheck::Valid)
        }))
    };
    let base = run(vec![
        plug("HEADLESS-1"),
        x11(1, &xterm),
        x11(2, &gimp),
        map(3, &web, TokenCheck::Valid),
    ]);
    (base, xterm, gimp, web)
}

fn terminations(run: &Run) -> Vec<Termination> {
    last(run)
        .iter()
        .filter_map(|effect| match effect {
            Effect::Terminate(stop) => Some(stop.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn force_quit_of_one_of_two_x11_apps_names_only_its_own_windows_never_the_satellite() {
    let (base, xterm, gimp, _) = two_x11_apps_and_a_wayland_app();
    for (app, window) in [(&xterm, WindowId(1)), (&gimp, WindowId(2))] {
        let asked = run_from(base.model.clone(), vec![force_quit(app)]);
        assert_eq!(
            terminations(&asked),
            vec![Termination {
                app: app.clone(),
                clients: Vec::new(),
                x11: vec![window],
                signal: StopSignal::Term,
            }],
            "{app:?}: no connection is named, so the satellite is spared"
        );
        assert!(matches!(
            asked.model.apps()[app].life,
            AppLife::Quitting(QuitPhase::Stopping)
        ));
        let other = if app == &xterm { &gimp } else { &xterm };
        assert_eq!(asked.model.apps()[other].life, AppLife::Running);
    }
}

#[test]
fn the_kill_after_the_grace_names_the_same_x11_windows_and_a_wayland_app_keeps_its_connection() {
    let (base, xterm, _, web) = two_x11_apps_and_a_wayland_app();
    let termed = run_from(base.model.clone(), vec![force_quit(&xterm)]);
    let killed = run_from(
        termed.model,
        vec![Op::Event(Event::TimerFired(TimerKey::QuitGrace(
            xterm.clone(),
        )))],
    );
    assert_eq!(
        terminations(&killed),
        vec![Termination {
            app: xterm,
            clients: Vec::new(),
            x11: vec![WindowId(1)],
            signal: StopSignal::Kill,
        }]
    );
    let wayland = run_from(base.model, vec![force_quit(&web)]);
    assert_eq!(
        terminations(&wayland),
        vec![Termination {
            app: web,
            clients: vec![ClientKey(3)],
            x11: Vec::new(),
            signal: StopSignal::Term,
        }]
    );
}

#[test]
fn the_force_quit_panel_lists_the_apps_and_who_answers() {
    let (mail, web) = (proven("org.quire.Mail"), proven("org.example.Web"));
    let stuck = run_from(
        two_apps().model,
        vec![Op::Event(Event::TimerFired(TimerKey::Ping(ClientKey(2))))],
    );
    let panel = run_from(
        stuck.model,
        vec![Op::Command(DesktopCommand::ForceQuitPanel)],
    );
    assert_eq!(
        last(&panel),
        [Effect::Shell(ShellNote::ForceQuitPanel(vec![
            ForceQuitRow {
                app: web,
                responding: true
            },
            ForceQuitRow {
                app: mail,
                responding: false
            },
        ]))]
    );
}
