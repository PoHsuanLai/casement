//! `org.quire.Compositor1.Connect` in the event loop (P2-Z): the D-Bus object forwards every
//! call down a calloop channel and [`answer`] decides it with the `State` in hand: the caller's
//! unit against the role it asked for (`casement_agent::hand_out`), and the server end of the
//! new socket inserted into the display as a client of that role.
//!
//! `casement-agent` makes the socket pair and the decision; only this crate can build a
//! client's data (its key, its gone queue), so the insertion is here. Frozen at P2-Z; filled
//! by P2-Q2.

use crate::State;
use casement_ipc::dbus::{ConnectAsk, ConnectCall, ConnectReply};
use smithay::reexports::calloop::channel::{Channel, Event};
use smithay::reexports::calloop::{Error as LoopError, LoopHandle};

/// Starts answering `calls` on `handle`'s loop.
pub fn attach(
    handle: &LoopHandle<'static, State>,
    calls: Channel<ConnectCall>,
) -> Result<(), LoopError> {
    handle
        .insert_source(calls, |event, (), state: &mut State| {
            if let Event::Msg(call) = event {
                let reply = answer(state, call.ask);
                // A caller that gave up has dropped the receiver; nothing to tell it.
                let _ = call.reply.send(reply);
            }
        })
        .map_err(LoopError::from)?;
    Ok(())
}

/// Decides one `Connect`: the unit in `ask.caller_cgroup` against `ask.role`; on a grant the
/// server end of a new socket pair is inserted as a client with that role (the model hears
/// `ClientConnected`), and the client end is the reply.
pub fn answer(state: &mut State, ask: ConnectAsk) -> ConnectReply {
    let _ = (state, ask);
    todo!("P2-Q2: answer a Connect")
}
