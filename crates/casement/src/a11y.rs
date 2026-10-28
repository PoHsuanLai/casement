//! A window's accessibility root, checked (PORTAL-15).
//!
//! `quire_window.set_a11y_root(bus_name, object_path)` names an object any program on the bus
//! could own, and the assistant tier reads the object it names. So a root a client sets is not
//! adopted as it comes: the window keeps an unknown root, the claim is queued with the pid
//! casement holds the client by, and the bus is asked which process owns the name
//! (`casement_ipc::a11y`). Only when that process is the client's own does the window get the
//! root. A client casement keeps no process for (the shell's and the agent tier's hand-outs, a
//! sandbox's) never verifies: its tree is not offered, the safe default.

use crate::core_loop;
use crate::state::State;
use casement_core::window::A11yRoot;
use casement_core::{ClientKey, Event, WindowHintChange, WindowId};
use casement_ipc::a11y::{A11yAsk, A11yFault, A11yVerdict, A11yWorker};
use smithay::reexports::calloop::channel::{Channel, Event as ChannelEvent};
use smithay::reexports::calloop::{Error as LoopError, LoopHandle};
use std::fmt::Debug;

/// Who checks the roots clients claim.
pub trait A11yChecks: Debug {
    /// Queues one claim for checking; the answer comes back through [`adopt`].
    fn check(&self, ask: A11yAsk) -> Result<(), A11yFault>;
}

impl A11yChecks for A11yWorker {
    fn check(&self, ask: A11yAsk) -> Result<(), A11yFault> {
        A11yWorker::check(self, ask)
    }
}

/// Nobody: no claim is ever verified, so every root stays unknown.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoA11y;

impl A11yChecks for NoA11y {
    fn check(&self, ask: A11yAsk) -> Result<(), A11yFault> {
        let _ = ask;
        Err(A11yFault::Stopped)
    }
}

/// Starts checking claims against the bus at `address`, and feeds each verdict into the model
/// from `handle`'s loop.
pub fn attach(
    state: &mut State,
    handle: &LoopHandle<'static, State>,
    address: &str,
) -> Result<(), LoopError> {
    let (worker, verdicts) = match A11yWorker::start(address.to_owned()) {
        Ok(started) => started,
        Err(error) => {
            tracing::warn!(%error, "window accessibility roots are not checked, so none is offered");
            return Ok(());
        }
    };
    state.seams.a11y = Box::new(worker);
    listen(handle, verdicts)
}

fn listen(
    handle: &LoopHandle<'static, State>,
    verdicts: Channel<A11yVerdict>,
) -> Result<(), LoopError> {
    handle
        .insert_source(verdicts, |event, (), state: &mut State| {
            if let ChannelEvent::Msg(verdict) = event {
                adopt(state, verdict);
            }
        })
        .map_err(LoopError::from)?;
    Ok(())
}

/// The model hears what the check came to. A window that closed meanwhile is not there to hear it.
pub fn adopt(state: &mut State, verdict: A11yVerdict) {
    core_loop::feed(
        state,
        vec![Event::WindowHint {
            window: verdict.window,
            hint: WindowHintChange::A11y(verdict.root),
        }],
    );
}

/// `events` as the model should hear them: a root a client claims becomes an unknown root, and
/// the claim is queued for checking.
pub fn claimed(state: &State, events: Vec<Event>) -> Vec<Event> {
    events
        .into_iter()
        .map(|event| match event {
            Event::WindowHint {
                window,
                hint:
                    WindowHintChange::A11y(Some(A11yRoot {
                        bus_name,
                        object_path,
                    })),
            } => {
                queue(state, window, bus_name, object_path);
                Event::WindowHint {
                    window,
                    hint: WindowHintChange::A11y(None),
                }
            }
            other => other,
        })
        .collect()
}

/// The pid of the process behind the client of `window`, when casement holds one.
fn pid_of(state: &State, window: WindowId) -> Option<u32> {
    let client: ClientKey = state.core.model.windows().get(&window)?.client;
    state
        .processes
        .of(&[client])
        .next()
        .and_then(|pid| u32::try_from(pid).ok())
}

fn queue(state: &State, window: WindowId, bus_name: String, object_path: String) {
    let Some(client_pid) = pid_of(state, window) else {
        return;
    };
    let ask = A11yAsk {
        window,
        client_pid,
        bus_name,
        object_path,
    };
    if let Err(fault) = state.seams.a11y.check(ask) {
        tracing::debug!(
            ?fault,
            "an accessibility root was not queued for checking; it stays unknown"
        );
    }
}
