//! Windows through `Model::step`: mapping onto a Space, unmapping, sizes, sheets and a client
//! going. Every row runs with the invariants checked after each step.

use crate::support;

use casement_core::focus::TokenCheck;
use casement_core::geometry::{LogicalPoint, LogicalRect, LogicalSize};
use casement_core::window::{SpacePlace, StackRank, WindowState};
use casement_core::{
    Cause, ClientKey, CoreNote, Effect, Event, InputSerial, ToplevelRequest, WindowId,
};
use casement_protocols::vocab::{Emphasis, PlaceTarget};
use support::driver::{
    Op, Run, assert_same_world, last, map, map_sheet, plug, request, run, unmap,
};
use support::proven;

fn state_of(run: &Run, n: u64) -> WindowState {
    run.model.windows()[&WindowId(n)].state.clone()
}

fn size(w: u32, h: u32) -> LogicalSize {
    LogicalSize { w, h }
}

#[test]
fn a_mapped_window_lands_unplaced_on_the_shown_desktop_at_the_top() {
    let mail = proven("org.quire.Mail");
    let run = run(vec![
        plug("HEADLESS-1"),
        map(1, &mail, TokenCheck::Valid),
        map(2, &mail, TokenCheck::Missing),
    ]);
    let key = support::headless("HEADLESS-1").key;
    let shown = run.model.spaces().shown_on(&key).expect("shown").id.clone();
    for n in [1, 2] {
        let window = &run.model.windows()[&WindowId(n)];
        assert_eq!(window.place, SpacePlace::On(shown.clone()), "window {n}");
        assert_eq!(window.state, WindowState::New(size(800, 600)), "window {n}");
    }
    let ranks: Vec<StackRank> = [1, 2]
        .iter()
        .map(|n| run.model.windows()[&WindowId(*n)].stack)
        .collect();
    assert_eq!(ranks, vec![StackRank(0), StackRank(1)]);
    assert_eq!(
        run.model.apps()[&mail].windows,
        vec![WindowId(1), WindowId(2)]
    );
    assert!(last(&run).contains(&Effect::Relayout(Cause::Open)));
    assert!(last(&run).contains(&Effect::RequestFrames(key)));
}

#[test]
fn a_window_mapped_before_any_output_waits_and_the_first_output_takes_it() {
    let mail = proven("org.quire.Mail");
    let waiting = run(vec![map(1, &mail, TokenCheck::Valid)]);
    let window = &waiting.model.windows()[&WindowId(1)];
    assert_eq!(
        (window.place.clone(), window.home.clone()),
        (SpacePlace::AllSpaces, None)
    );
    assert_eq!(
        waiting.model.focus().key,
        Some(WindowId(1)),
        "it is still key"
    );

    let landed = support::driver::run_from(waiting.model, vec![plug("HEADLESS-1")]);
    let key = support::headless("HEADLESS-1").key;
    let desktop = landed
        .model
        .spaces()
        .shown_on(&key)
        .expect("shown")
        .id
        .clone();
    assert_eq!(
        landed.model.windows()[&WindowId(1)].place,
        SpacePlace::On(desktop)
    );
}

#[test]
fn a_window_of_the_active_app_takes_the_key_without_a_token() {
    let mail = proven("org.quire.Mail");
    let run = run(vec![
        plug("HEADLESS-1"),
        map(1, &mail, TokenCheck::Valid),
        map(2, &mail, TokenCheck::Missing),
    ]);
    assert_eq!(run.model.focus().key, Some(WindowId(2)));
    assert_eq!(run.model.windows()[&WindowId(2)].emphasis, Emphasis::Key);
    assert_eq!(
        run.model.windows()[&WindowId(1)].emphasis,
        Emphasis::Inactive
    );
    assert_eq!(run.model.focus().main, Some(WindowId(2)));
}

#[test]
fn a_shell_window_sits_on_every_space_of_the_primary_output() {
    let bar = proven("org.quire.Bar");
    let shell = Op::Event(Event::ToplevelMapped(casement_core::MappedToplevel {
        role: casement_protocols::vocab::WindowRole::Shell,
        ..support::driver::mapped(1, &bar, TokenCheck::Missing)
    }));
    let run = run(vec![plug("HEADLESS-1"), shell]);
    let window = &run.model.windows()[&WindowId(1)];
    assert_eq!(window.place, SpacePlace::AllSpaces);
    assert_eq!(window.home, Some(support::headless("HEADLESS-1").key));
}

#[test]
fn a_modal_sheet_takes_the_key_and_gives_it_back() {
    let mail = proven("org.quire.Mail");
    let opened = run(vec![
        plug("HEADLESS-1"),
        map(1, &mail, TokenCheck::Valid),
        map_sheet(2, &mail, 1),
    ]);
    assert_eq!(opened.model.focus().key, Some(WindowId(2)));
    assert_eq!(opened.model.focus().main, Some(WindowId(1)));
    let parent_place = opened.model.windows()[&WindowId(1)].place.clone();
    assert_eq!(opened.model.windows()[&WindowId(2)].place, parent_place);

    let closed = support::driver::run_from(opened.model, vec![unmap(2)]);
    assert_eq!(closed.model.focus().key, Some(WindowId(1)));
    assert!(
        last(&closed).contains(&Effect::Keyboard(casement_core::KeyboardTarget::Window(
            WindowId(1)
        )))
    );
}

#[test]
fn a_sheet_on_another_apps_window_is_not_a_sheet() {
    let (mail, web) = (proven("org.quire.Mail"), proven("org.example.Web"));
    let run = run(vec![
        plug("HEADLESS-1"),
        map(1, &mail, TokenCheck::Valid),
        map_sheet(2, &web, 1),
    ]);
    assert_eq!(
        run.model.windows()[&WindowId(2)].parent,
        casement_core::window::Parent::None
    );
    assert_eq!(run.model.focus().key, Some(WindowId(1)));
}

#[test]
fn unmapping_an_unknown_window_is_noted_and_changes_nothing() {
    let before = run(vec![plug("HEADLESS-1")]);
    let after = support::driver::run_from(before.model.clone(), vec![unmap(9)]);
    assert_same_world(&after.model, &before.model, "unchanged");
    assert_eq!(
        last(&after),
        [Effect::Note(CoreNote::UnknownWindow(WindowId(9)))]
    );
}

#[test]
fn the_last_window_unmapping_leaves_its_app_windowless_and_active() {
    let mail = proven("org.quire.Mail");
    let run = run(vec![
        plug("HEADLESS-1"),
        map(1, &mail, TokenCheck::Valid),
        unmap(1),
    ]);
    assert_eq!(
        run.model.apps()[&mail].life,
        casement_core::app::AppLife::Windowless
    );
    assert_eq!(run.model.focus().active, Some(mail));
    assert_eq!(run.model.focus().key, None);
    assert!(last(&run).contains(&Effect::Relayout(Cause::Close)));
}

#[test]
fn a_client_going_unmaps_its_windows_and_takes_its_app() {
    let (mail, web) = (proven("org.quire.Mail"), proven("org.example.Web"));
    let run = run(vec![
        plug("HEADLESS-1"),
        map(1, &web, TokenCheck::Valid),
        map(2, &mail, TokenCheck::Valid),
        Op::Event(Event::ClientGone {
            client: ClientKey(2),
        }),
    ]);
    assert!(!run.model.windows().contains_key(&WindowId(2)));
    assert!(!run.model.apps().contains_key(&mail));
    assert_eq!(run.model.focus().active, Some(web));
    assert_eq!(run.model.focus().key, Some(WindowId(1)));
    assert_eq!(
        run.model.focus().app_mru.len(),
        1,
        "the gone app left the MRU"
    );
}

#[test]
fn a_client_that_serves_two_windows_unmaps_both() {
    let mail = proven("org.quire.Mail");
    let both = Op::Event(Event::ToplevelMapped(casement_core::MappedToplevel {
        client: ClientKey(1),
        ..support::driver::mapped(2, &mail, TokenCheck::Missing)
    }));
    let run = run(vec![
        plug("HEADLESS-1"),
        map(1, &mail, TokenCheck::Valid),
        both,
        Op::Event(Event::ClientGone {
            client: ClientKey(1),
        }),
    ]);
    assert!(run.model.windows().is_empty());
    assert!(run.model.apps().is_empty());
    assert_eq!(run.model.focus().active, None);
}

/// CASC-04: a connection that carries windows of several apps (the X11 satellite; a peer that
/// claims a different app id for each toplevel) leaves none of them behind, and an app another
/// connection still serves stays.
#[test]
fn a_client_that_carried_several_apps_leaves_none_of_them_behind() {
    let (mail, web, notes) = (
        proven("org.quire.Mail"),
        proven("org.example.Web"),
        proven("org.example.Notes"),
    );
    let on_client = |n: u64, client: u64, app: &casement_core::app::AppKey| {
        Op::Event(Event::ToplevelMapped(casement_core::MappedToplevel {
            client: ClientKey(client),
            ..support::driver::mapped(n, app, TokenCheck::Missing)
        }))
    };
    let gone = |client: u64| {
        Op::Event(Event::ClientGone {
            client: ClientKey(client),
        })
    };
    let carried = run(vec![
        plug("HEADLESS-1"),
        on_client(1, 1, &mail),
        on_client(2, 1, &web),
        on_client(3, 1, &notes),
        on_client(4, 2, &mail),
    ]);
    assert_eq!(carried.model.apps().len(), 3);
    let left = support::driver::run_from(carried.model, vec![gone(1)]);
    assert_eq!(
        left.model.apps().keys().cloned().collect::<Vec<_>>(),
        vec![mail.clone()],
        "web and notes went with their only connection; mail is still served by client 2"
    );
    assert_eq!(left.model.windows().len(), 1);
    let empty = support::driver::run_from(left.model, vec![gone(2)]);
    assert!(empty.model.apps().is_empty());
}

#[test]
fn committed_sizes_update_unplaced_and_floating_windows_only() {
    let mail = proven("org.quire.Mail");
    let committed = |w, h| {
        Op::Event(Event::ToplevelCommitted {
            window: WindowId(1),
            size: size(w, h),
        })
    };
    let unplaced = run(vec![
        plug("HEADLESS-1"),
        map(1, &mail, TokenCheck::Valid),
        committed(640, 480),
    ]);
    assert_eq!(state_of(&unplaced, 1), WindowState::New(size(640, 480)));
    assert!(last(&unplaced).contains(&Effect::Relayout(Cause::Configure)));

    let centred = support::driver::run_from(
        unplaced.model,
        vec![request(
            1,
            ToplevelRequest::Place {
                target: PlaceTarget::Centre,
                serial: InputSerial(1),
            },
        )],
    );
    let origin = LogicalPoint { x: 640, y: 300 };
    assert_eq!(
        state_of(&centred, 1),
        WindowState::Floating(LogicalRect::new(origin, size(640, 480)))
    );
    let floating = support::driver::run_from(centred.model, vec![committed(320, 240)]);
    assert_eq!(
        state_of(&floating, 1),
        WindowState::Floating(LogicalRect::new(origin, size(320, 240)))
    );

    let same = support::driver::run_from(floating.model.clone(), vec![committed(320, 240)]);
    assert_same_world(&same.model, &floating.model, "unchanged");
    assert_eq!(last(&same), []);
}

#[test]
fn a_hint_or_meta_change_on_an_unknown_window_is_noted() {
    let run = run(vec![Op::Event(Event::ToplevelCommitted {
        window: WindowId(3),
        size: size(1, 1),
    })]);
    assert_eq!(
        last(&run),
        [Effect::Note(CoreNote::UnknownWindow(WindowId(3)))]
    );
}

#[test]
fn a_press_on_a_window_behind_its_sheet_rings_the_bell_and_goes_nowhere() {
    let mail = proven("org.quire.Mail");
    let behind = run(vec![
        plug("HEADLESS-1"),
        map(1, &mail, TokenCheck::Valid),
        map_sheet(2, &mail, 1),
        support::driver::press(1),
    ]);
    assert!(last(&behind).contains(&Effect::Bell { app: mail }));
    assert_eq!(
        behind.model.focus().key,
        Some(WindowId(2)),
        "the sheet keeps the key"
    );
}

fn hint(n: u64, hint: casement_core::WindowHintChange) -> Op {
    Op::Event(Event::WindowHint {
        window: WindowId(n),
        hint,
    })
}

#[test]
fn a_window_menu_is_recorded_and_the_shell_is_told_through_its_app() {
    use casement_core::WindowHintChange::Menu;
    use casement_core::app::MenuSource;
    use casement_core::desktop::DesktopNote;
    let qt = proven("org.example.Qt");
    let source = MenuSource::DbusMenu {
        bus: "org.example.Qt".into(),
        path: "/MenuBar".into(),
    };
    let mapped = run(vec![
        plug("HEADLESS-1"),
        map(1, &qt, TokenCheck::Valid),
        map(2, &qt, TokenCheck::Missing),
    ]);
    assert_eq!(mapped.model.app_menu(&qt), MenuSource::None);
    // Which of the two is key is the focus rules' business; the menu follows whichever it is.
    let key = mapped.model.focus().key.expect("a key window");
    let other = if key == WindowId(1) { 2 } else { 1 };

    let declared = support::driver::run_from(mapped.model, vec![hint(other, Menu(source.clone()))]);
    assert_eq!(
        declared.model.windows()[&WindowId(other)].hints.menu,
        source
    );
    assert_eq!(
        declared.model.app_menu(&qt),
        source,
        "the app's other window answers"
    );
    assert!(last(&declared).contains(&Effect::Desktop(DesktopNote::App(qt.clone()))));
    assert!(last(&declared).contains(&Effect::Desktop(DesktopNote::Window(WindowId(other)))));

    let own = MenuSource::GtkMenus {
        bus: ":1.9".into(),
        app_path: "/app".into(),
        menubar_path: "/menus".into(),
        window_path: "/win".into(),
    };
    let keyed = support::driver::run_from(declared.model, vec![hint(key.0, Menu(own.clone()))]);
    assert_eq!(keyed.model.app_menu(&qt), own, "the key window's menu wins");

    let cleared = support::driver::run_from(keyed.model, vec![hint(key.0, Menu(MenuSource::None))]);
    assert_eq!(cleared.model.app_menu(&qt), source);
}

#[test]
fn picture_in_picture_joins_every_space_keeps_its_aspect_and_snaps_to_a_corner() {
    use casement_core::WindowHintChange::Role;
    use casement_protocols::vocab::WindowRole;
    let web = proven("org.example.Web");
    let output = support::headless("HEADLESS-1").key;
    let drag = |x, y, w, h| {
        Op::Event(Event::WindowDragged {
            window: WindowId(1),
            rect: LogicalRect::new(LogicalPoint { x, y }, size(w, h)),
        })
    };
    let floating = run(vec![
        plug("HEADLESS-1"),
        map(1, &web, TokenCheck::Valid),
        Op::Event(Event::ToplevelCommitted {
            window: WindowId(1),
            size: size(320, 180),
        }),
        map(2, &web, TokenCheck::Missing),
        hint(1, Role(WindowRole::Pip)),
        drag(800, 500, 320, 180),
    ]);
    let window = &floating.model.windows()[&WindowId(1)];
    assert_eq!(window.place, SpacePlace::AllSpaces);
    assert_eq!(window.home, Some(output));
    assert_eq!(
        floating.model.windows()[&WindowId(2)].place,
        SpacePlace::On(
            floating
                .model
                .spaces()
                .shown_on(&support::headless("HEADLESS-1").key)
                .expect("shown")
                .id
                .clone()
        ),
        "a plain window stays on its Space"
    );

    // Pulled wider by its right edge, it keeps 16:9; let go, it snaps to the bottom right.
    let resized = support::driver::run_from(floating.model, vec![drag(800, 500, 480, 180)]);
    assert_eq!(
        state_of(&resized, 1),
        WindowState::Floating(LogicalRect::new(
            LogicalPoint { x: 800, y: 500 },
            size(480, 270)
        ))
    );
    let dropped = support::driver::run_from(
        resized.model,
        vec![Op::Event(Event::WindowDragEnded {
            window: WindowId(1),
        })],
    );
    assert_eq!(
        state_of(&dropped, 1),
        WindowState::Floating(LogicalRect::new(
            LogicalPoint { x: 1424, y: 794 },
            size(480, 270)
        ))
    );
    assert!(last(&dropped).contains(&Effect::Relayout(Cause::Drag)));

    // An ordinary window is neither fitted nor snapped.
    let plain = support::driver::run_from(
        dropped.model,
        vec![
            Op::Event(Event::WindowDragged {
                window: WindowId(2),
                rect: LogicalRect::new(LogicalPoint { x: 10, y: 10 }, size(300, 300)),
            }),
            Op::Event(Event::WindowDragEnded {
                window: WindowId(2),
            }),
        ],
    );
    assert_eq!(
        plain.model.windows()[&WindowId(2)].state,
        WindowState::Floating(LogicalRect::new(
            LogicalPoint { x: 10, y: 10 },
            size(300, 300)
        ))
    );
}

#[test]
fn a_browsers_picture_in_picture_window_is_guessed_from_its_title() {
    use casement_core::{ToplevelMeta, window::WindowTitle};
    let firefox = support::driver::mapped(1, &proven("org.mozilla.firefox"), TokenCheck::Valid);
    let run = run(vec![
        plug("HEADLESS-1"),
        Op::Event(Event::ToplevelMapped(casement_core::MappedToplevel {
            claimed_id: support::claimed("firefox"),
            ..firefox
        })),
        Op::Event(Event::ToplevelMeta {
            window: WindowId(1),
            meta: ToplevelMeta::Title(WindowTitle("Picture-in-Picture".into())),
        }),
    ]);
    assert_eq!(
        run.model.windows()[&WindowId(1)].place,
        SpacePlace::AllSpaces
    );
}

/// CASC-11: what floats above every other window is small, and a guess from a title and an app
/// id counts only for a proven app; one picture in picture window per app joins every Space.
#[test]
fn only_a_small_believed_window_floats_above_and_one_per_app_joins_every_space() {
    use casement_core::window::{WindowTitle, floats_above};
    use casement_core::{ToplevelMeta, WindowHintChange::Role};
    use casement_protocols::vocab::WindowRole;
    let proven_app = proven("org.mozilla.firefox");
    let claimed_app = casement_core::app::AppKey::Claimed(support::claimed("firefox"));
    let meta = |n: u64, meta: ToplevelMeta| {
        Op::Event(Event::ToplevelMeta {
            window: WindowId(n),
            meta,
        })
    };
    let guessed = |n: u64| {
        vec![
            meta(n, ToplevelMeta::ClaimedId(support::claimed("firefox"))),
            meta(
                n,
                ToplevelMeta::Title(WindowTitle("Picture-in-Picture".to_owned())),
            ),
        ]
    };
    let committed = |n: u64, w: u32, h: u32| {
        Op::Event(Event::ToplevelCommitted {
            window: WindowId(n),
            size: size(w, h),
        })
    };
    let spread =
        |run: &Run, n: u64| run.model.windows()[&WindowId(n)].place == SpacePlace::AllSpaces;
    let floats = |run: &Run, n: u64| floats_above(&run.model.windows()[&WindowId(n)]);

    // A guess floats for a proven app and not for a claimed one.
    for (name, app, want) in [
        ("proven", &proven_app, true),
        ("claimed", &claimed_app, false),
    ] {
        let mut ops = vec![
            plug("HEADLESS-1"),
            map(1, app, TokenCheck::Valid),
            committed(1, 320, 180),
        ];
        ops.extend(guessed(1));
        let run = run(ops);
        assert_eq!(floats(&run, 1), want, "{name}: floats");
        assert_eq!(spread(&run, 1), want, "{name}: joins every Space");
    }

    // A declared window floats only while small.
    let small = run(vec![
        plug("HEADLESS-1"),
        map(1, &proven_app, TokenCheck::Valid),
        committed(1, 320, 180),
        hint(1, Role(WindowRole::Pip)),
    ]);
    assert!(floats(&small, 1) && spread(&small, 1));
    let grown = support::driver::run_from(small.model, vec![committed(1, 1920, 1080)]);
    assert!(
        !floats(&grown, 1),
        "a window the size of the screen does not float"
    );
    let huge = run(vec![
        plug("HEADLESS-1"),
        map(1, &proven_app, TokenCheck::Valid),
        committed(1, 1920, 1080),
        hint(1, Role(WindowRole::Pip)),
    ]);
    assert!(!floats(&huge, 1) && !spread(&huge, 1), "too big to join");

    // The second picture in picture window of an app stays on its Space.
    let two = run(vec![
        plug("HEADLESS-1"),
        map(1, &proven_app, TokenCheck::Valid),
        map(2, &proven_app, TokenCheck::Missing),
        committed(1, 320, 180),
        committed(2, 320, 180),
        hint(1, Role(WindowRole::Pip)),
        hint(2, Role(WindowRole::Pip)),
    ]);
    assert!(spread(&two, 1) && !spread(&two, 2));
    let other_app = run(vec![
        plug("HEADLESS-1"),
        map(1, &proven_app, TokenCheck::Valid),
        map(2, &proven("org.example.Video"), TokenCheck::Missing),
        committed(1, 320, 180),
        committed(2, 320, 180),
        hint(1, Role(WindowRole::Pip)),
        hint(2, Role(WindowRole::Pip)),
    ]);
    assert!(
        spread(&other_app, 1) && spread(&other_app, 2),
        "another app has its own"
    );
}

/// CASC-11: the one-per-app rule counts a connection, not the app id its windows claim. A client
/// that maps windows with a different claimed app id each still gets one picture in picture
/// window on every Space.
#[test]
fn one_connection_gets_one_picture_in_picture_window_whatever_app_ids_it_claims() {
    use casement_core::WindowHintChange::Role;
    use casement_core::app::AppKey;
    use casement_protocols::vocab::WindowRole;
    let on_one_connection = |n: u64, id: &str| {
        let app = AppKey::Claimed(support::claimed(id));
        Op::Event(Event::ToplevelMapped(casement_core::MappedToplevel {
            client: ClientKey(1),
            ..support::driver::mapped(n, &app, TokenCheck::Missing)
        }))
    };
    let committed = |n: u64| {
        Op::Event(Event::ToplevelCommitted {
            window: WindowId(n),
            size: size(320, 180),
        })
    };
    let spread =
        |run: &Run, n: u64| run.model.windows()[&WindowId(n)].place == SpacePlace::AllSpaces;
    let run = run(vec![
        plug("HEADLESS-1"),
        on_one_connection(1, "one"),
        on_one_connection(2, "two"),
        on_one_connection(3, "three"),
        committed(1),
        committed(2),
        committed(3),
        hint(1, Role(WindowRole::Pip)),
        hint(2, Role(WindowRole::Pip)),
        hint(3, Role(WindowRole::Pip)),
    ]);
    let spread_count = [1, 2, 3].into_iter().filter(|n| spread(&run, *n)).count();
    assert_eq!(spread_count, 1, "one window, not one per claimed id");
}
