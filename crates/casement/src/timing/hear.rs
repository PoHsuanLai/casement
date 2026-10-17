//! The moments, heard from the loop: the glue between what `State` knows and the pure records.
//! Each function is a few lookups; a frame with nothing to report returns at once.

use super::first_show::{Awaiting, Opened};
use super::session_ready::ShellSurface;
use super::stamp::Stamp;
use super::token::Token;
use super::trigger::{FeedKind, press_in};
use crate::state::State;
use casement_core::layer::{LayerOwner, ShellRoleTag};
use casement_core::policy::ConnectionRole;
use casement_core::time::Mono;
use casement_core::window::WindowOrigin;
use casement_core::{ClientKey, Event, MappedToplevel, OutputKey, Pid, WindowId};
use casement_motion::OutputFrame;
use casement_protocols::vocab::Emphasis;

/// A turn of the loop begins with `events`: what kind of turn it is, and the press it heard, are
/// what a change of key window in it is judged by. Returns the enclosing turn's kind for
/// [`leave`].
pub(crate) fn enter(state: &mut State, events: &[Event]) -> FeedKind {
    let press = press_in(events, |input| state.timing.stamp(input.at));
    state.timing.enter(FeedKind::of(events), press)
}

/// The turn that returned `outer` from [`enter`] ends.
pub(crate) fn leave(state: &mut State, outer: FeedKind) {
    state.timing.leave(outer);
}

/// A client was taken in as `key`, through `role`; `origin` says whether it is the X11 satellite.
pub(crate) fn connected(
    state: &mut State,
    key: ClientKey,
    role: &ConnectionRole,
    origin: WindowOrigin,
) {
    let at = state.timing.stamp(state.clock.now());
    state.timing.client_connected(key, at);
    if matches!(role, ConnectionRole::Shell) {
        state.timing.shell_connected(at);
    }
    if matches!(origin, WindowOrigin::X11) {
        state.timing.satellite_connected(at);
    }
}

/// xwayland-satellite was started at `now`.
pub(crate) fn satellite_started(state: &mut State, now: Mono) {
    let at = state.timing.stamp(now);
    state.timing.satellite_started(at);
}

/// Replaces the stamp of when `key` connected with the earlier one its socket was accepted at
/// (a public connection waits for the look at its program before it is taken in).
pub(crate) fn accepted_at(state: &mut State, key: ClientKey, at: Stamp) {
    state.timing.client_connected(key, at);
}

/// The model heard `events` at `now`: a toplevel mapped (its first commit with a buffer), went,
/// or its client did.
pub(crate) fn hear(state: &mut State, events: &[Event], now: Mono) {
    for event in events {
        match event {
            Event::ClientGone { client } => state.timing.client_gone(*client),
            Event::ToplevelUnmapped { window } => state.timing.window_unmapped(*window),
            Event::ToplevelMapped(mapped) => mapped_at(state, mapped, now),
            _ => {}
        }
    }
}

fn mapped_at(state: &mut State, mapped: &MappedToplevel, now: Mono) {
    let first_commit_at = state.timing.stamp(now);
    let connected_at = state
        .timing
        .connected_at(mapped.client)
        .unwrap_or(first_commit_at);
    let pid = state
        .processes
        .of(&[mapped.client])
        .next()
        .and_then(|raw| u32::try_from(raw).ok())
        .map_or(Pid(0), Pid);
    let opened = Opened {
        app_id: Token::of(mapped.claimed_id.as_str()),
        pid,
        connected_at,
        first_commit_at,
    };
    state
        .timing
        .window_mapped(mapped.window, mapped.client, opened);
}

/// `changes` of emphasis are being carried out: a window made key is a change of key window.
pub(crate) fn emphasis(state: &mut State, changes: &[(WindowId, Emphasis)]) {
    let at = state.timing.stamp(state.clock.now());
    for (window, emphasis) in changes {
        match emphasis {
            Emphasis::Key => {
                let app_id = state
                    .core
                    .model
                    .windows()
                    .get(window)
                    .map(|held| Token::of(held.claimed_id.as_str()));
                if let Some(app_id) = app_id {
                    state.timing.key_changed(*window, app_id, at);
                }
            }
            _ => state.timing.key_lost(*window),
        }
    }
}

/// A frame of `output` was queued to the screen: the windows it shows for the first time, the
/// ones it shows at rest, the key changes it carries and the shell's surfaces it shows. `sampled`
/// is the instant the frame's motion was sampled at.
///
/// The moment is the one read as the page flip is queued: the glass shows the frame at the next
/// vblank, up to one refresh later.
pub(crate) fn flipped(state: &mut State, output: &OutputKey, frame: &OutputFrame, sampled: Mono) {
    if state.timing.is_idle() {
        return;
    }
    let at = state.timing.stamp(state.clock.now());
    state.timing.keys_presented(at);
    for window in state.timing.awaiting(Awaiting::Frame) {
        if frame.windows.iter().any(|placed| placed.window == window) {
            state.timing.window_shown(window, at);
        }
    }
    for window in state.timing.awaiting(Awaiting::Rest) {
        if !state.core.motion.window_is_moving(window, sampled) {
            state.timing.window_rested(window, at);
        }
    }
    shell_shown(state, output, at);
}

/// A frame of `output` changed nothing on the glass: the key changes made before it are shown.
pub(crate) fn unchanged(state: &mut State) {
    if state.timing.is_idle() {
        return;
    }
    let at = state.timing.stamp(state.clock.now());
    state.timing.keys_presented(at);
}

/// The shell's bar and dock that `output`'s frame shows, while the session's start is open.
fn shell_shown(state: &mut State, output: &OutputKey, at: Stamp) {
    if !state.timing.watches_shell() {
        return;
    }
    let shown: Vec<ShellSurface> = state
        .core
        .model
        .layers()
        .iter()
        .filter(|(_, spec)| spec.output == *output && spec.owner == LayerOwner::Shell)
        .filter(|(key, _)| state.render.textures.layer(key).is_some())
        .filter_map(|(_, spec)| match spec.role {
            ShellRoleTag::Assigned(role) => ShellSurface::of(role),
            ShellRoleTag::Unassigned => None,
        })
        .collect();
    for surface in shown {
        state.timing.shell_shown(surface, at);
    }
}
