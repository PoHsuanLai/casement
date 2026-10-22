//! `xdg_activation` (P0-H2): A7. A token earned by a recent press of the asking client, used
//! while it is recent, activates; a stale or unearned one only marks attention. Token age is
//! judged on the session clock, so these tests move the virtual clock and never sleep (A36).
//! Ignored until the fixture and the core loop are filled.

use crate::protocol::xdg_shell::{
    Corner, boot, click_at, corner_of, map_toplevel, pointer, settle, windows,
};
use casement_core::geometry::PhysicalSize;
use casement_core::policy::ConnectionRole;
use casement_testclient::misc::activation::{Activation, TokenError};
use casement_testclient::surface::toplevel::{WmBase, bind_pointer};
use casement_testclient::{Pattern, shm_buffer};
use std::time::Duration;

/// A token from a recent press of the focused app activates; a stale or foreign one only marks attention.
#[test]
fn activation_follows_the_a7_rule() {
    let (mut fixture, launcher) = boot();
    let app = fixture.connect(ConnectionRole::Public);
    fixture.pump();
    let launcher_base = WmBase::bind(fixture.client(launcher), 7);
    let launcher_pointer = bind_pointer(fixture.client(launcher));
    let activation = Activation::bind(fixture.client(launcher));
    fixture.pump();
    map_toplevel(&mut fixture, launcher, &launcher_base, (400, 300));
    let device = pointer(&mut fixture);
    let launcher_window = windows(&fixture)[0];
    // Two clicks: the first activates the launcher (swallowed), the second reaches it with a serial.
    // (At the launcher's corner: the app's windows are cascaded over its middle later.)
    settle(&mut fixture);
    let on_launcher = corner_of(&fixture, launcher_window, Corner::TopLeft);
    click_at(&mut fixture, device, on_launcher);
    click_at(&mut fixture, device, on_launcher);
    let serial = launcher_pointer
        .last_press()
        .expect("the launcher was pressed");
    assert_eq!(fixture.model().focus().key, Some(launcher_window));

    // 1. A token earned by that press, used at once before the new app maps: the app takes focus.
    let earned = activation.token(
        fixture.client(launcher),
        Some((&launcher_pointer.seat, serial)),
        Some("org.example.App"),
    );
    fixture.pump();
    let token = earned.with(|seen| seen.clone()).expect("a token arrives");
    let app_base = WmBase::bind(fixture.client(app), 7);
    let app_activation = Activation::bind(fixture.client(app));
    let first = app_base.toplevel(fixture.client(app));
    app_activation.activate(&token, &first);
    first.commit_bare();
    fixture.pump();
    first.ack_latest();
    let buffer = shm_buffer(
        fixture.client(app),
        PhysicalSize { w: 300, h: 200 },
        Pattern::Gradient,
    );
    first.show(&buffer);
    fixture.pump();
    let first_window = windows(&fixture)[1];
    assert_eq!(
        fixture.model().focus().key,
        Some(first_window),
        "a valid token activates"
    );

    // 2. A token with no press behind it: the next window maps without focus (attention only).
    let bare = app_activation.token(fixture.client(app), None, None);
    fixture.pump();
    let bare_token = bare.with(|seen| seen.clone()).expect("a token arrives");
    // Another app's window: a new window of the active app takes the key by mapping, with or
    // without a token (03 section 1.2), so attention-only needs a different app.
    let second = app_base.toplevel(fixture.client(app));
    second.set_app_id("org.example.Other");
    app_activation.activate(&bare_token, &second);
    second.commit_bare();
    fixture.pump();
    second.ack_latest();
    second.show(&buffer);
    fixture.pump();
    assert_eq!(
        fixture.model().focus().key,
        Some(first_window),
        "an unearned token does not move focus"
    );

    // 3. A token earned by a press but used after its lifetime: stale, attention only.
    click_at(&mut fixture, device, on_launcher);
    let serial = launcher_pointer.last_press().expect("a fresh press");
    let old = activation.token(
        fixture.client(launcher),
        Some((&launcher_pointer.seat, serial)),
        None,
    );
    fixture.pump();
    let old_token = old.with(|seen| seen.clone()).expect("a token arrives");
    fixture.advance(Duration::from_secs(6));
    let third = app_base.toplevel(fixture.client(app));
    third.set_app_id("org.example.Other");
    app_activation.activate(&old_token, &third);
    third.commit_bare();
    fixture.pump();
    third.ack_latest();
    third.show(&buffer);
    fixture.pump();
    let focused = fixture.model().focus().key;
    assert_ne!(
        focused,
        Some(windows(&fixture)[3]),
        "a stale token does not activate"
    );

    // 4. A launch token (it names the app) earned by a press, used after the press lifetime:
    // a slow app's first window still takes focus.
    click_at(&mut fixture, device, on_launcher);
    let serial = launcher_pointer.last_press().expect("a fresh press");
    let launch = activation.token(
        fixture.client(launcher),
        Some((&launcher_pointer.seat, serial)),
        Some("org.example.Slow"),
    );
    fixture.pump();
    let launch_token = launch.with(|seen| seen.clone()).expect("a token arrives");
    fixture.advance(Duration::from_secs(20));
    let fourth = app_base.toplevel(fixture.client(app));
    fourth.set_app_id("org.example.Slow");
    app_activation.activate(&launch_token, &fourth);
    fourth.commit_bare();
    fixture.pump();
    fourth.ack_latest();
    fourth.show(&buffer);
    fixture.pump();
    assert_eq!(
        fixture.model().focus().key,
        Some(windows(&fixture)[4]),
        "a launch token outlives the press lifetime"
    );
}

/// A token request committed twice is `already_used`; the global binds at version 1.
#[test]
fn a_token_commits_once() {
    let (mut fixture, id) = boot();
    let activation = Activation::bind(fixture.client(id));
    let (request, text) = activation.request(fixture.client(id));
    request.commit();
    fixture.pump();
    assert!(
        text.with(|seen| seen.is_some()),
        "the first commit is answered with a token"
    );
    assert!(fixture.client(id).protocol_error().is_none());
    request.commit();
    fixture.pump();
    let error = fixture
        .client(id)
        .protocol_error()
        .expect("a protocol error");
    assert_eq!(error.code, TokenError::AlreadyUsed as u32);
}
