//! `ext_idle_notifier_v1`: idle and resumed notifications. smithay's notifier arms calloop
//! timers on the wall clock, which the virtual clock cannot stop, and hands the handler no
//! hook when a client asks for one, so casement serves the protocol itself (A36): the global
//! and both object types are ours, and idleness is timed on the session's timers
//! (`TimerKey::IdleNotify`, armed through `Effect::ArmTimer`) by the pure [`IdleBook`].
//!
//! smithay's `IdleNotifierState` is not created: clients see `ext_idle_notifier_v1` exactly
//! once, from here.
//!
//! The hooks the core loop calls: [`on_activity`] for every input event of any origin (virtual
//! keyboards included, I8), [`on_timer`] when `TimerFired(IdleNotify(..))` arrives and
//! [`on_inhibit`] when idle-inhibit changes. Each returns timer effects for `exec` and sends
//! `idled` or `resumed` itself. See `crate::idle_notify::machine` for what they decide.

mod machine;

pub use machine::{IdleBook, Moved, Signal};

use crate::client::ClientState;
use crate::limits::{self, Limit};
use crate::states::{GlobalFilter, WaylandFault};
use casement_core::policy::ConnectionRole;
use casement_core::time::{IdleNotifierId, Mono};
use casement_core::{ClientKey, Effect};
use porter_core::PeerIdentity;
use smithay::reexports::wayland_protocols::ext::idle_notify::v1::server::{
    ext_idle_notification_v1::{self, ExtIdleNotificationV1},
    ext_idle_notifier_v1::{self, ExtIdleNotifierV1},
};
use smithay::reexports::wayland_server::backend::{ClientId, GlobalId};
use smithay::reexports::wayland_server::{
    Client, DataInit, Dispatch, DisplayHandle, GlobalDispatch, New,
};
use smithay::wayland::{Dispatch2, GlobalDispatch2};
use std::collections::BTreeMap;
use std::time::Duration;

/// The version casement serves (2 adds `get_input_idle_notification`).
pub const VERSION: u32 = 2;

/// The shortest idle timeout a sandboxed or unproven peer is served (CASW-07, decided
/// 2026-10-16): a `resumed` fires on every input event on the desktop, so a 1 ms watch would
/// read another window's keystroke timing; 5 s is far from useful for that and still short
/// enough for a screensaver or a video player.
pub const UNTRUSTED_MIN_TIMEOUT: Duration = Duration::from_secs(5);

/// The shortest timeout `role` and `peer` get: the shell and the agent tier any, and every
/// other peer [`UNTRUSTED_MIN_TIMEOUT`] (NEW-CAS-7). A native peer proven only by the name of a
/// launcher's scope is no exception: any program of the person's can make a scope of any name,
/// so the name proves nothing about who it is.
pub fn timeout_floor(role: &ConnectionRole, peer: &PeerIdentity) -> Duration {
    let _ = peer;
    match role {
        ConnectionRole::Shell | ConnectionRole::Agent => Duration::ZERO,
        ConnectionRole::Sandboxed(_) | ConnectionRole::Public | ConnectionRole::Portal => {
            UNTRUSTED_MIN_TIMEOUT
        }
    }
}

/// `ext_idle_notifier_v1`: the watches and their objects.
#[derive(Debug)]
pub struct IdleNotifyPart {
    /// Our global.
    pub global: GlobalId,
    /// Who is watching, and for how long they have been quiet.
    pub book: IdleBook,
    objects: BTreeMap<IdleNotifierId, ExtIdleNotificationV1>,
}

impl IdleNotifyPart {
    /// Creates the global (version 2). Every connection sees it.
    pub fn new<D>(display: &DisplayHandle, role_filter: GlobalFilter) -> IdleNotifyPart
    where
        D: GlobalDispatch<ExtIdleNotifierV1, IdleGlobal> + 'static,
    {
        let _ = role_filter;
        IdleNotifyPart {
            global: display.create_global::<D, ExtIdleNotifierV1, _>(VERSION, IdleGlobal),
            book: IdleBook::default(),
            objects: BTreeMap::new(),
        }
    }
}

/// What the composition root provides to this protocol's dispatch.
pub trait IdleNotifyHandler: Sized + 'static {
    /// The part.
    fn idle_notify_part(&mut self) -> &mut IdleNotifyPart;
    /// The session's now (the one clock, `casement::time`).
    fn session_now(&self) -> Mono;
    /// Carries out timer effects (`casement::exec::exec`).
    fn idle_effects(&mut self, effects: Vec<Effect>);
}

/// The id of a notification object that watches nothing (its client went over the cap): ids
/// are minted from 0 and cannot reach it.
const UNWATCHED: IdleNotifierId = IdleNotifierId(u64::MAX);

/// The global's data.
#[derive(Debug, Clone, Copy, Default)]
pub struct IdleGlobal;

/// A bound notifier's data.
#[derive(Debug, Clone, Copy, Default)]
pub struct IdleNotifierData;

/// One notification object's data: which watch it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IdleNotificationData {
    /// The watch.
    pub id: IdleNotifierId,
}

/// A watch the book accepted: its id and the timer to arm.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Watched {
    /// The notifier's id.
    pub id: IdleNotifierId,
    /// Timers to arm.
    pub effects: Vec<Effect>,
}

/// `client` asked to be told after `timeout` of idleness at `now` (`honours_inhibitors`:
/// whether idle inhibitors hold it off).
pub fn on_notification(
    part: &mut IdleNotifyPart,
    client: ClientKey,
    timeout: Duration,
    honours_inhibitors: bool,
    now: Mono,
) -> Watched {
    let (book, id, moved) =
        std::mem::take(&mut part.book).watch(client, timeout, honours_inhibitors, now);
    part.book = book;
    Watched {
        id,
        effects: settle(part, moved),
    }
}

/// Binds notifier `id` to its protocol object.
pub fn attach(part: &mut IdleNotifyPart, id: IdleNotifierId, object: ExtIdleNotificationV1) {
    part.objects.insert(id, object);
}

/// Input of any origin arrived at `now`: idle watches are told `resumed`.
pub fn on_activity(part: &mut IdleNotifyPart, now: Mono) -> Vec<Effect> {
    let (book, moved) = std::mem::take(&mut part.book).activity(now);
    part.book = book;
    settle(part, moved)
}

/// `TimerFired(IdleNotify(client, id))` at `now`: the watch is told `idled` if it has been
/// quiet its whole timeout, else its timer is re-armed.
pub fn on_timer(
    part: &mut IdleNotifyPart,
    client: ClientKey,
    id: IdleNotifierId,
    now: Mono,
) -> Vec<Effect> {
    let (book, moved) = std::mem::take(&mut part.book).fire(client, id, now);
    part.book = book;
    settle(part, moved)
}

/// Idle inhibition changed at `now` (`inhibited`: whether a visible surface holds idleness off).
pub fn on_inhibit(part: &mut IdleNotifyPart, inhibited: bool, now: Mono) -> Vec<Effect> {
    let (book, moved) = std::mem::take(&mut part.book).inhibit(inhibited, now);
    part.book = book;
    settle(part, moved)
}

/// The notification object `id` was destroyed (by its client or with it).
pub fn on_destroyed(part: &mut IdleNotifyPart, id: IdleNotifierId) -> Vec<Effect> {
    part.objects.remove(&id);
    let (book, moved) = std::mem::take(&mut part.book).forget(id);
    part.book = book;
    moved.effects
}

/// Sends `idled` to notifier `id`.
pub fn apply_idle(part: &mut IdleNotifyPart, id: IdleNotifierId) -> Result<(), WaylandFault> {
    part.objects
        .get(&id)
        .map(ExtIdleNotificationV1::idled)
        .ok_or(WaylandFault::SurfaceGone)
}

/// Sends `resumed` to notifier `id`.
pub fn apply_resumed(part: &mut IdleNotifyPart, id: IdleNotifierId) -> Result<(), WaylandFault> {
    part.objects
        .get(&id)
        .map(ExtIdleNotificationV1::resumed)
        .ok_or(WaylandFault::SurfaceGone)
}

/// Tells clients what `moved` says, in order, and returns its timer effects. A signal for an
/// object that is gone is dropped: its client left between the transition and the wire.
fn settle(part: &mut IdleNotifyPart, moved: Moved) -> Vec<Effect> {
    for signal in &moved.signals {
        let _ = match signal {
            Signal::Idled(id) => apply_idle(part, *id),
            Signal::Resumed(id) => apply_resumed(part, *id),
        };
    }
    moved.effects
}

impl<D> GlobalDispatch2<ExtIdleNotifierV1, D> for IdleGlobal
where
    D: Dispatch<ExtIdleNotifierV1, IdleNotifierData> + 'static,
{
    fn bind(
        &self,
        _state: &mut D,
        _handle: &DisplayHandle,
        _client: &Client,
        resource: New<ExtIdleNotifierV1>,
        data_init: &mut DataInit<'_, D>,
    ) {
        data_init.init(resource, IdleNotifierData);
    }
}

impl<D> Dispatch2<ExtIdleNotifierV1, D> for IdleNotifierData
where
    D: IdleNotifyHandler + Dispatch<ExtIdleNotificationV1, IdleNotificationData>,
{
    fn request(
        &self,
        state: &mut D,
        client: &Client,
        _resource: &ExtIdleNotifierV1,
        request: ext_idle_notifier_v1::Request,
        handle: &DisplayHandle,
        data_init: &mut DataInit<'_, D>,
    ) {
        let (new, timeout, honours_inhibitors) = match request {
            ext_idle_notifier_v1::Request::GetIdleNotification { id, timeout, .. } => {
                (id, timeout, true)
            }
            ext_idle_notifier_v1::Request::GetInputIdleNotification { id, timeout, .. } => {
                (id, timeout, false)
            }
            _ => return,
        };
        let (key, floor) = ClientState::of(client)
            .map_or((ClientKey(0), UNTRUSTED_MIN_TIMEOUT), |data| {
                (data.key, timeout_floor(&data.role, &data.peer))
            });
        // A client past its cap is cut after this request (CASW-07's watch cap, NEW-CAS-2); the
        // object it asked for is made inert, with an id no watch ever has.
        let held = state.idle_notify_part().book.held_by(key);
        if !limits::admit(handle, client, Limit::IdleWatches, held) {
            data_init.init(new, IdleNotificationData { id: UNWATCHED });
            return;
        }
        let now = state.session_now();
        let watched = on_notification(
            state.idle_notify_part(),
            key,
            Duration::from_millis(u64::from(timeout)).max(floor),
            honours_inhibitors,
            now,
        );
        let object = data_init.init(new, IdleNotificationData { id: watched.id });
        attach(state.idle_notify_part(), watched.id, object);
        state.idle_effects(watched.effects);
    }
}

impl<D> Dispatch2<ExtIdleNotificationV1, D> for IdleNotificationData
where
    D: IdleNotifyHandler,
{
    fn request(
        &self,
        _state: &mut D,
        _client: &Client,
        _resource: &ExtIdleNotificationV1,
        request: ext_idle_notification_v1::Request,
        _handle: &DisplayHandle,
        _data_init: &mut DataInit<'_, D>,
    ) {
        // `destroy` is the only request; the object's destruction does the work.
        let _ = request;
    }

    fn destroyed(&self, state: &mut D, _client: ClientId, _resource: &ExtIdleNotificationV1) {
        let effects = on_destroyed(state.idle_notify_part(), self.id);
        state.idle_effects(effects);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use porter_core::{AppId, AppName, Isolation};

    fn proven(isolation: Isolation) -> PeerIdentity {
        PeerIdentity::Proven(AppId {
            name: AppName::parse("org.quire.Mail").expect("an app name"),
            isolation,
        })
    }

    /// Who gets a floor, and what the timeout becomes (CASW-07).
    #[test]
    fn an_unproven_or_sandboxed_peer_is_served_no_less_than_five_seconds() {
        let one_ms = Duration::from_millis(1);
        let cases = [
            (
                "a public unproven peer",
                ConnectionRole::Public,
                PeerIdentity::Unproven,
                true,
            ),
            (
                "a public proven native peer (a scope name proves nothing)",
                ConnectionRole::Public,
                proven(Isolation::Unsandboxed),
                true,
            ),
            (
                "a public proven Flatpak peer",
                ConnectionRole::Public,
                proven(Isolation::Flatpak),
                true,
            ),
            (
                "the shell",
                ConnectionRole::Shell,
                PeerIdentity::Unproven,
                false,
            ),
            (
                "the agent tier",
                ConnectionRole::Agent,
                PeerIdentity::Unproven,
                false,
            ),
        ];
        for (name, role, peer, floored) in cases {
            let got = one_ms.max(timeout_floor(&role, &peer));
            let want = if floored {
                UNTRUSTED_MIN_TIMEOUT
            } else {
                one_ms
            };
            assert_eq!(got, want, "{name}");
        }
        assert_eq!(
            Duration::from_secs(60).max(timeout_floor(
                &ConnectionRole::Public,
                &PeerIdentity::Unproven
            )),
            Duration::from_secs(60),
            "a long timeout is untouched"
        );
    }
}
