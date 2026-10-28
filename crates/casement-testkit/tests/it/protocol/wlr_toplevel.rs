//! `wlr_toplevel` (P1-M2): the wlr window list. A window is listed with its title, app id,
//! states and outputs; a handle's requests take the transitions the window's own requests do;
//! `set_rectangle` is stored for the minimise animation; the listing's transcript is a golden.

use crate::protocol::consent::{ScriptedShell, kept, program};
use crate::protocol::xdg_shell::{boot, check_golden, golden, windows};
use casement_core::geometry::{LogicalPoint, LogicalRect, LogicalSize, PhysicalSize};
use casement_core::policy::ConnectionRole;
use casement_core::privilege::{UnixSeconds, Verdict};
use casement_core::window::WindowState;
use casement_core::{Event, WindowId};
use casement_protocols::vocab::{Answer, Emphasis, Power};
use casement_testclient::manage::wlr_toplevel::{Closing, ToplevelWatch, WlrListed, WlrState};
use casement_testclient::surface::toplevel::{Toplevel, WmBase};
use casement_testclient::{Pattern, ShmBuffer, shm_buffer};
use casement_testkit::{ASKED_PROGRAM, ClientId, Fixture, FixtureSpec};

/// Maps a titled window of `app_id` (the buffer must outlive the window).
pub(super) fn map_titled(
    fixture: &mut Fixture,
    id: ClientId,
    base: &WmBase,
    title: &str,
) -> (Toplevel, ShmBuffer) {
    let top = base.toplevel(fixture.client(id));
    top.set_title(title);
    top.set_app_id("org.example.Mail");
    top.commit_bare();
    fixture.pump();
    assert!(top.ack_latest().is_some(), "the initial configure arrives");
    let buffer = shm_buffer(
        fixture.client(id),
        PhysicalSize { w: 320, h: 200 },
        Pattern::Gradient,
    );
    top.show(&buffer);
    fixture.pump();
    (top, buffer)
}

/// A started desktop, one client with a mapped window and a bound window list, all read.
fn listed_window() -> (
    Fixture,
    ClientId,
    Toplevel,
    ShmBuffer,
    ToplevelWatch,
    WindowId,
) {
    let (mut fixture, id) = boot();
    let base = WmBase::bind(fixture.client(id), 7);
    let (top, buffer) = map_titled(&mut fixture, id, &base, "Inbox");
    let window = windows(&fixture)[0];
    let mut watch = ToplevelWatch::start(fixture.client(id).globals()).expect("the list");
    fixture.pump();
    watch.pump().expect("pump");
    (fixture, id, top, buffer, watch, window)
}

/// Pumps the compositor and the list client until both are quiet.
fn settle(fixture: &mut Fixture, watch: &mut ToplevelWatch) {
    fixture.pump();
    watch.pump().expect("pump");
}

/// Makes a request on the list, then pumps both ends until the answer is read.
fn ask(fixture: &mut Fixture, watch: &mut ToplevelWatch, request: impl FnOnce(&mut ToplevelWatch)) {
    request(watch);
    for _ in 0..2 {
        settle(fixture, watch);
    }
}

/// The states a window of the model shows, as the list names them.
fn states_of(fixture: &Fixture, window: WindowId) -> Vec<WlrState> {
    let held = &fixture.model().windows()[&window];
    let mut states = Vec::new();
    match held.state {
        WindowState::Filled { .. } => states.push(WlrState::Maximized),
        WindowState::Minimised { .. } => states.push(WlrState::Minimized),
        _ => {}
    }
    if held.emphasis == Emphasis::Key {
        states.push(WlrState::Activated);
    }
    if matches!(held.state, WindowState::FullScreen { .. }) {
        states.push(WlrState::Fullscreen);
    }
    states
}

/// A mapped window is listed with its title, app id, state and output; closing it sends
/// `closed`.
#[test]
fn a_window_is_listed_with_its_state_and_withdrawn_when_it_closes() {
    let (mut fixture, _id, top, _buffer, mut watch, window) = listed_window();
    let want = WlrListed {
        title: "Inbox".to_owned(),
        app_id: "org.example.Mail".to_owned(),
        states: states_of(&fixture, window),
        outputs: 1,
        closed: Closing::Open,
    };
    assert_eq!(watch.listed(), std::slice::from_ref(&want));

    top.set_title("Drafts");
    top.surface.commit();
    settle(&mut fixture, &mut watch);
    assert_eq!(
        watch.listed(),
        [WlrListed {
            title: "Drafts".to_owned(),
            ..want.clone()
        }],
        "a new title is announced on the same handle"
    );

    top.destroy();
    settle(&mut fixture, &mut watch);
    let after = watch.listed();
    assert_eq!(after.len(), 1);
    assert_eq!(after[0].closed, Closing::Closed);
    assert_eq!(after[0].title, "Drafts");
}

/// A list client that binds after a window mapped is sent that window.
#[test]
fn a_late_list_client_is_sent_the_windows_already_open() {
    let (mut fixture, id, _top, _buffer, _first, _window) = listed_window();
    let mut late = ToplevelWatch::start(fixture.client(id).globals()).expect("the list");
    settle(&mut fixture, &mut late);
    assert_eq!(late.listed().len(), 1);
    assert_eq!(late.listed()[0].title, "Inbox");
}

/// activate, maximize, minimize and close on a handle reach the same transitions the window's
/// own requests take.
#[test]
fn handle_requests_move_the_window_like_its_own_requests() {
    let (mut fixture, _id, top, _buffer, mut watch, window) = listed_window();
    let state = |fixture: &Fixture| fixture.model().windows()[&window].state.clone();
    ask(&mut fixture, &mut watch, |watch| {
        watch.maximize(0).expect("request")
    });
    assert!(
        matches!(state(&fixture), WindowState::Filled { .. }),
        "maximised"
    );
    assert!(watch.listed()[0].states.contains(&WlrState::Maximized));
    ask(&mut fixture, &mut watch, |watch| {
        watch.unmaximize(0).expect("request")
    });
    assert!(
        matches!(state(&fixture), WindowState::Floating(_)),
        "restored"
    );

    ask(&mut fixture, &mut watch, |watch| {
        watch.minimize(0).expect("request")
    });
    assert!(
        matches!(state(&fixture), WindowState::Minimised { .. }),
        "minimised"
    );
    assert!(watch.listed()[0].states.contains(&WlrState::Minimized));
    ask(&mut fixture, &mut watch, |watch| {
        watch.unminimize(0).expect("request")
    });
    assert!(
        !matches!(state(&fixture), WindowState::Minimised { .. }),
        "brought back from the dock"
    );

    ask(&mut fixture, &mut watch, |watch| {
        watch.activate(0).expect("request")
    });
    assert_eq!(fixture.model().focus().key, Some(window), "activated");
    assert!(watch.listed()[0].states.contains(&WlrState::Activated));

    ask(&mut fixture, &mut watch, |watch| {
        watch.fullscreen(0).expect("request")
    });
    assert!(
        matches!(state(&fixture), WindowState::FullScreen { .. }),
        "full screen"
    );
    assert!(watch.listed()[0].states.contains(&WlrState::Fullscreen));
    ask(&mut fixture, &mut watch, |watch| {
        watch.unfullscreen(0).expect("request")
    });
    assert!(
        !matches!(state(&fixture), WindowState::FullScreen { .. }),
        "back from full screen"
    );

    let closes = top.log.with(|seen| seen.closes);
    ask(&mut fixture, &mut watch, |watch| {
        watch.close(0).expect("request")
    });
    assert_eq!(
        top.log.with(|seen| seen.closes),
        closes + 1,
        "close asks the window to close"
    );
}

/// A minimised window asked to maximise or go full screen comes back from the dock and takes the
/// state; a full-screen window asked to maximise stays full screen and is maximised when it leaves.
#[test]
fn a_state_asked_of_a_minimised_window_brings_it_back_and_a_fill_waits_out_full_screen() {
    let (mut fixture, _id, _top, _buffer, mut watch, window) = listed_window();
    let state = |fixture: &Fixture| fixture.model().windows()[&window].state.clone();

    ask(&mut fixture, &mut watch, |watch| {
        watch.minimize(0).expect("request")
    });
    ask(&mut fixture, &mut watch, |watch| {
        watch.maximize(0).expect("request")
    });
    assert!(
        matches!(state(&fixture), WindowState::Filled { .. }),
        "out of the dock and maximised"
    );

    ask(&mut fixture, &mut watch, |watch| {
        watch.minimize(0).expect("request")
    });
    ask(&mut fixture, &mut watch, |watch| {
        watch.fullscreen(0).expect("request")
    });
    assert!(
        matches!(state(&fixture), WindowState::FullScreen { .. }),
        "out of the dock and full screen"
    );

    ask(&mut fixture, &mut watch, |watch| {
        watch.unmaximize(0).expect("request")
    });
    ask(&mut fixture, &mut watch, |watch| {
        watch.unfullscreen(0).expect("request")
    });
    ask(&mut fixture, &mut watch, |watch| {
        watch.fullscreen(0).expect("request")
    });
    ask(&mut fixture, &mut watch, |watch| {
        watch.maximize(0).expect("request")
    });
    assert!(
        matches!(state(&fixture), WindowState::FullScreen { .. }),
        "still full screen"
    );
    ask(&mut fixture, &mut watch, |watch| {
        watch.unfullscreen(0).expect("request")
    });
    assert!(
        matches!(state(&fixture), WindowState::Filled { .. }),
        "maximised once it leaves full screen"
    );
}

/// `set_rectangle` is stored for the minimise animation.
#[test]
fn the_minimise_rectangle_is_stored() {
    let (mut fixture, id, _top, _buffer, mut watch, window) = listed_window();
    let tile = fixture.client(id).create_surface().expect("a surface");
    assert_eq!(
        fixture.model().minimise_target(window),
        None,
        "none set yet"
    );
    watch
        .set_rectangle(0, &tile, (500, 700, 64, 48))
        .expect("request");
    watch.minimize(0).expect("request");
    settle(&mut fixture, &mut watch);
    let want = LogicalRect::new(
        LogicalPoint { x: 500, y: 700 },
        LogicalSize { w: 64, h: 48 },
    );
    assert_eq!(
        fixture
            .model()
            .minimise_target(window)
            .map(|target| target.rect),
        Some(want),
        "a surface casement did not place sits at the origin"
    );
    assert!(matches!(
        fixture.model().windows()[&window].state,
        WindowState::Minimised { .. }
    ));

    watch
        .set_rectangle(0, &tile, (0, 0, 0, 0))
        .expect("request");
    settle(&mut fixture, &mut watch);
    assert_eq!(
        fixture.model().minimise_target(window),
        None,
        "an empty rectangle takes it back"
    );
}

/// Ids in a transcript are the process's: they are numbered by first appearance.
pub(crate) fn normalised(text: &str) -> String {
    let mut seen: Vec<String> = Vec::new();
    let mut out = String::new();
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        out.push(c);
        if c != '#' {
            continue;
        }
        let mut digits = String::new();
        while let Some(digit) = chars.peek().copied().filter(|d| d.is_ascii_digit()) {
            digits.push(digit);
            chars.next();
        }
        let at = match seen.iter().position(|held| *held == digits) {
            Some(at) => at,
            None => {
                seen.push(digits);
                seen.len() - 1
            }
        };
        out.push_str(&(at + 1).to_string());
    }
    out
}

/// The transcript text of a client's flow over objects whose names start with one of
/// `prefixes`, with the arguments of the events named in `masked` replaced by `<name>` and the
/// ids numbered by first appearance.
pub(crate) fn flow(
    fixture: &mut Fixture,
    id: ClientId,
    prefixes: &[&str],
    masked: &[&str],
) -> String {
    let text: String = fixture
        .client(id)
        .transcript()
        .0
        .iter()
        .filter(|line| {
            prefixes
                .iter()
                .any(|prefix| line.object.starts_with(prefix))
        })
        .map(|line| {
            let args = match masked.contains(&line.event.as_str()) {
                true => format!("<{}>", line.event),
                false => line.args.join(", "),
            };
            format!("{}.{}({args})\n", line.object, line.event)
        })
        .collect();
    normalised(&text)
}

/// The announcement of one window: the transcript matches
/// tests/golden/transcripts/wlr_toplevel-list.txt.
#[test]
fn the_listing_transcript_matches_its_golden() {
    let (mut fixture, id) = boot();
    let base = WmBase::bind(fixture.client(id), 7);
    let (_top, _buffer) = map_titled(&mut fixture, id, &base, "Inbox");
    let mut watch = ToplevelWatch::start(fixture.client(id).globals()).expect("the list");
    settle(&mut fixture, &mut watch);
    let text = flow(&mut fixture, id, &["zwlr_foreign_toplevel_"], &["state"]);
    check_golden(&golden("wlr_toplevel-list.txt"), &text).expect("the listing flow");
}

/// CASW-06: a client may bind the window list `MAX_WINDOW_LIST_MANAGERS` times (each bind lists
/// every window again and hears every title change); the next bind cuts it with
/// `wl_display.no_memory` (code 2).
#[test]
fn a_client_that_binds_the_window_list_without_end_is_cut() {
    let (mut fixture, id) = boot();
    let cap = casement_wayland::limits::MAX_WINDOW_LIST_MANAGERS;
    let mut watches = Vec::new();
    for _ in 0..cap {
        watches.push(ToplevelWatch::start(fixture.client(id).globals()).expect("the list"));
        fixture.pump();
    }
    assert!(
        fixture.client(id).protocol_error().is_none(),
        "up to the cap is allowed"
    );
    watches.push(ToplevelWatch::start(fixture.client(id).globals()).expect("a bind"));
    fixture.pump();
    // The cut closes the connection (the system backend sends no error first).
    assert!(
        !fixture.client(id).is_connected(),
        "the client past the cap is cut"
    );
}

/// A desktop with the scripted shell, a window "Inbox" mapped by a client the desktop trusts,
/// and a program that is asked about its powers.
fn asked_with_window() -> (
    Fixture,
    ScriptedShell,
    ClientId,
    Toplevel,
    ShmBuffer,
    WindowId,
) {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let shell = ScriptedShell::start(&mut fixture);
    let owner = fixture.connect(ConnectionRole::Public);
    fixture.pump();
    let base = WmBase::bind(fixture.client(owner), 7);
    let (top, buffer) = map_titled(&mut fixture, owner, &base, "Inbox");
    let window = windows(&fixture)[0];
    let asked = fixture.connect_via_ask();
    fixture.pump();
    (fixture, shell, asked, top, buffer, window)
}

/// Seeing and managing the windows is a power: a list bound by a program that is asked hears
/// nothing while the question is open, the shell is asked once, and on Allow the windows are
/// listed and kept up to date.
#[test]
fn a_window_list_bound_while_asked_is_listed_nothing_until_allowed() {
    let (mut fixture, shell, asked, top, _buffer, _window) = asked_with_window();
    let mut watch = ToplevelWatch::start(fixture.client(asked).globals()).expect("the list");
    settle(&mut fixture, &mut watch);
    assert_eq!(shell.asked().len(), 1, "the shell is asked once");
    assert_eq!(shell.asked()[0].power, Power::Windows);
    assert!(watch.listed().is_empty(), "no window is listed yet");
    assert!(!watch.finished());

    shell.answer(&mut fixture, shell.only_ask(), Answer::Allow);
    settle(&mut fixture, &mut watch);
    assert_eq!(watch.listed().len(), 1);
    assert_eq!(watch.listed()[0].title, "Inbox");
    assert!(!watch.finished());
    assert_eq!(
        kept(&fixture, ASKED_PROGRAM, Power::Windows),
        Some(Verdict::Allow)
    );

    top.set_title("Drafts");
    top.surface.commit();
    settle(&mut fixture, &mut watch);
    assert_eq!(watch.listed()[0].title, "Drafts", "and kept up to date");
}

/// Don't Allow: the manager is `finished` and lists nothing.
#[test]
fn a_refused_window_list_is_finished() {
    let (mut fixture, shell, asked, _top, _buffer, _window) = asked_with_window();
    let mut watch = ToplevelWatch::start(fixture.client(asked).globals()).expect("the list");
    settle(&mut fixture, &mut watch);
    shell.answer(&mut fixture, shell.only_ask(), Answer::Deny);
    settle(&mut fixture, &mut watch);
    assert!(watch.finished());
    assert!(watch.listed().is_empty());
    assert_eq!(
        kept(&fixture, ASKED_PROGRAM, Power::Windows),
        Some(Verdict::Deny)
    );

    // The refusal is kept: binding again is finished at once, without a second question.
    let mut again = ToplevelWatch::start(fixture.client(asked).globals()).expect("the list");
    settle(&mut fixture, &mut again);
    assert!(again.finished());
    assert_eq!(shell.asked().len(), 1);
}

/// Taking the power back finishes the manager in use; its handles change nothing from then on
/// and it hears nothing more.
#[test]
fn a_power_taken_back_finishes_the_window_list_and_silences_its_handles() {
    let (mut fixture, shell, asked, top, _buffer, window) = asked_with_window();
    let mut watch = ToplevelWatch::start(fixture.client(asked).globals()).expect("the list");
    settle(&mut fixture, &mut watch);
    shell.answer(&mut fixture, shell.only_ask(), Answer::Allow);
    settle(&mut fixture, &mut watch);
    assert_eq!(watch.listed().len(), 1);
    assert!(!watch.finished());

    fixture.feed_events(vec![Event::PrivilegeEdited {
        edit: casement_core::privilege::Edit::Deny(program(ASKED_PROGRAM), Power::Windows),
        at: UnixSeconds(2),
    }]);
    settle(&mut fixture, &mut watch);
    assert!(watch.finished(), "the manager in use is finished");

    let before = states_of(&fixture, window);
    ask(&mut fixture, &mut watch, |watch| {
        watch.maximize(0).expect("request")
    });
    assert_eq!(
        states_of(&fixture, window),
        before,
        "the handle does nothing"
    );
    assert!(!states_of(&fixture, window).contains(&WlrState::Maximized));

    top.set_title("Drafts");
    top.surface.commit();
    settle(&mut fixture, &mut watch);
    assert_eq!(watch.listed()[0].title, "Inbox", "and hears nothing more");
}

/// Managers that wait for the answer count toward the cap of window lists a client may bind:
/// past it the client is cut, however long the question stays open.
#[test]
fn waiting_window_lists_count_toward_the_cap_of_binds() {
    let (mut fixture, shell, asked, _top, _buffer, _window) = asked_with_window();
    let cap = casement_wayland::limits::MAX_WINDOW_LIST_MANAGERS;
    let mut watches = Vec::new();
    for _ in 0..cap {
        watches.push(ToplevelWatch::start(fixture.client(asked).globals()).expect("the list"));
        fixture.pump();
    }
    assert!(
        fixture.client(asked).is_connected(),
        "up to the cap is allowed"
    );
    assert_eq!(shell.asked().len(), 1, "one question for all of them");
    assert!(watches.iter().all(|watch| watch.listed().is_empty()));
    watches.push(ToplevelWatch::start(fixture.client(asked).globals()).expect("a bind"));
    fixture.pump();
    assert!(
        !fixture.client(asked).is_connected(),
        "the client past the cap is cut while its lists wait"
    );
}
