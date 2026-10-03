//! `ext_idle_notifier_v1`: idle and resumed notifications. smithay's notifier arms calloop
//! timers on the wall clock, which the virtual clock cannot stop, and hands the handler no
//! hook when a client asks for one, so casement serves the protocol itself (A36): the global
//! and both object types are ours, and idleness is timed on the session's timers
//! (`TimerKey::IdleNotify`, armed through `Effect::ArmTimer`) by the pure [`IdleBook`].
//!
//! smithay's [`IdleNotifierState`] stays in the part only because [`WaylandStates`] bounds its
//! state type by smithay's `IdleNotifierHandler`; its global is withdrawn at once, so clients
//! see `ext_idle_notifier_v1` exactly once, from here.
//!
//! The hooks the core loop calls: [`on_activity`] for every input event of any origin (virtual
//! keyboards included, I8), [`on_timer`] when `TimerFired(IdleNotify(..))` arrives and
//! [`on_inhibit`] when idle-inhibit changes. Each returns timer effects for `exec` and sends
//! `idled` or `resumed` itself. See `crate::idle_notify::machine` for what they decide.
//!
//! [`WaylandStates`]: crate::states::WaylandStates

mod machine;

pub use machine::{Honours, IdleBook, Inhibited, Moved, Signal};

use crate::client::ClientState;
use crate::states::{GlobalFilter, WaylandFault};
use casement_core::time::{IdleNotifierId, Mono};
use casement_core::{ClientKey, Effect};
use smithay::reexports::calloop::LoopHandle;
use smithay::reexports::wayland_protocols::ext::idle_notify::v1::server::{
    ext_idle_notification_v1::{self, ExtIdleNotificationV1},
    ext_idle_notifier_v1::{self, ExtIdleNotifierV1},
};
use smithay::reexports::wayland_server::backend::{ClientId, GlobalId};
use smithay::reexports::wayland_server::{
    Client, DataInit, Dispatch, DisplayHandle, GlobalDispatch, New,
};
use smithay::wayland::idle_notify::{IdleNotifierHandler, IdleNotifierState};
use smithay::wayland::{Dispatch2, GlobalData, GlobalDispatch2};
use std::collections::BTreeMap;
use std::time::Duration;

/// The version casement serves (2 adds `get_input_idle_notification`).
pub const VERSION: u32 = 2;

/// `ext_idle_notifier_v1`: the watches and their objects.
#[derive(Debug)]
pub struct IdleNotifyPart<D: IdleNotifierHandler + 'static> {
    /// smithay's state, kept for the handler trait that bounds `WaylandStates`; its global is
    /// withdrawn and nothing reads it.
    pub state: IdleNotifierState<D>,
    /// Our global.
    pub global: GlobalId,
    /// Who is watching, and for how long they have been quiet.
    pub book: IdleBook,
    objects: BTreeMap<IdleNotifierId, ExtIdleNotificationV1>,
}

impl<D: IdleNotifierHandler + 'static> IdleNotifyPart<D> {
    /// Creates the global (version 2). Every connection sees it.
    pub fn new(
        display: &DisplayHandle,
        role_filter: GlobalFilter,
        event_loop: LoopHandle<'static, D>,
    ) -> IdleNotifyPart<D>
    where
        D: GlobalDispatch<ExtIdleNotifierV1, GlobalData>
            + GlobalDispatch<ExtIdleNotifierV1, IdleGlobal>,
    {
        let _ = role_filter;
        let state = IdleNotifierState::new(display, event_loop);
        display.remove_global::<D>(state.global());
        IdleNotifyPart {
            state,
            global: display.create_global::<D, ExtIdleNotifierV1, _>(VERSION, IdleGlobal),
            book: IdleBook::default(),
            objects: BTreeMap::new(),
        }
    }
}

/// What the composition root provides to this protocol's dispatch.
pub trait IdleNotifyHandler: IdleNotifierHandler + Sized + 'static {
    /// The part.
    fn idle_notify_part(&mut self) -> &mut IdleNotifyPart<Self>;
    /// The session's now (the one clock, `casement::time`).
    fn session_now(&self) -> Mono;
    /// Carries out timer effects (`casement::exec::exec`).
    fn idle_effects(&mut self, effects: Vec<Effect>);
}

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

/// `client` asked to be told after `timeout` of idleness at `now` (`honours`: whether idle
/// inhibitors hold it off).
pub fn on_notification<D: IdleNotifierHandler + 'static>(
    part: &mut IdleNotifyPart<D>,
    client: ClientKey,
    timeout: Duration,
    honours: Honours,
    now: Mono,
) -> Watched {
    let (book, id, moved) = std::mem::take(&mut part.book).watch(client, timeout, honours, now);
    part.book = book;
    Watched {
        id,
        effects: settle(part, moved),
    }
}

/// Binds notifier `id` to its protocol object.
pub fn attach<D: IdleNotifierHandler + 'static>(
    part: &mut IdleNotifyPart<D>,
    id: IdleNotifierId,
    object: ExtIdleNotificationV1,
) {
    part.objects.insert(id, object);
}

/// Input of any origin arrived at `now`: idle watches are told `resumed`.
pub fn on_activity<D: IdleNotifierHandler + 'static>(
    part: &mut IdleNotifyPart<D>,
    now: Mono,
) -> Vec<Effect> {
    let (book, moved) = std::mem::take(&mut part.book).activity(now);
    part.book = book;
    settle(part, moved)
}

/// `TimerFired(IdleNotify(client, id))` at `now`: the watch is told `idled` if it has been
/// quiet its whole timeout, else its timer is re-armed.
pub fn on_timer<D: IdleNotifierHandler + 'static>(
    part: &mut IdleNotifyPart<D>,
    client: ClientKey,
    id: IdleNotifierId,
    now: Mono,
) -> Vec<Effect> {
    let (book, moved) = std::mem::take(&mut part.book).fire(client, id, now);
    part.book = book;
    settle(part, moved)
}

/// Idle inhibition changed at `now` (idle-inhibit's [`Inhibited`]).
pub fn on_inhibit<D: IdleNotifierHandler + 'static>(
    part: &mut IdleNotifyPart<D>,
    inhibited: Inhibited,
    now: Mono,
) -> Vec<Effect> {
    let (book, moved) = std::mem::take(&mut part.book).inhibit(inhibited, now);
    part.book = book;
    settle(part, moved)
}

/// The notification object `id` was destroyed (by its client or with it).
pub fn on_destroyed<D: IdleNotifierHandler + 'static>(
    part: &mut IdleNotifyPart<D>,
    id: IdleNotifierId,
) -> Vec<Effect> {
    part.objects.remove(&id);
    let (book, moved) = std::mem::take(&mut part.book).forget(id);
    part.book = book;
    moved.effects
}

/// Sends `idled` to notifier `id`.
pub fn apply_idle<D: IdleNotifierHandler + 'static>(
    part: &mut IdleNotifyPart<D>,
    id: IdleNotifierId,
) -> Result<(), WaylandFault> {
    part.objects
        .get(&id)
        .map(ExtIdleNotificationV1::idled)
        .ok_or(WaylandFault::SurfaceGone)
}

/// Sends `resumed` to notifier `id`.
pub fn apply_resumed<D: IdleNotifierHandler + 'static>(
    part: &mut IdleNotifyPart<D>,
    id: IdleNotifierId,
) -> Result<(), WaylandFault> {
    part.objects
        .get(&id)
        .map(ExtIdleNotificationV1::resumed)
        .ok_or(WaylandFault::SurfaceGone)
}

/// Tells clients what `moved` says, in order, and returns its timer effects. A signal for an
/// object that is gone is dropped: its client left between the transition and the wire.
fn settle<D: IdleNotifierHandler + 'static>(
    part: &mut IdleNotifyPart<D>,
    moved: Moved,
) -> Vec<Effect> {
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
        _handle: &DisplayHandle,
        data_init: &mut DataInit<'_, D>,
    ) {
        let (new, timeout, honours) = match request {
            ext_idle_notifier_v1::Request::GetIdleNotification { id, timeout, .. } => {
                (id, timeout, Honours::Inhibitors)
            }
            ext_idle_notifier_v1::Request::GetInputIdleNotification { id, timeout, .. } => {
                (id, timeout, Honours::InputOnly)
            }
            _ => return,
        };
        let key = ClientState::of(client).map_or(ClientKey(0), |data| data.key);
        let now = state.session_now();
        let watched = on_notification(
            state.idle_notify_part(),
            key,
            Duration::from_millis(u64::from(timeout)),
            honours,
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
