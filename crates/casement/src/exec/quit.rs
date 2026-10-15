//! `Effect::Terminate` (P3-QU): force quit. The processes stopped are the ones on the other end
//! of the named connections' sockets (their credentials, read by the kernel when the client
//! connected), never a process found by name. Only connections that came through the public
//! socket are signalled: a sandboxed client's credentials are in another process namespace,
//! and the shell's and the agent tier's connections are not apps to kill.

use crate::state::State;
use casement_core::ClientKey;
use casement_core::app::{StopSignal, Termination};
use rustix::process::{Pid, RawPid, Signal, getpid, kill_process};
use std::collections::BTreeSet;

/// Signals the processes of `stop`'s connections.
pub(super) fn terminate(state: &mut State, stop: Termination) {
    let pids = pids_of(state, &stop.clients);
    let signal = match stop.signal {
        StopSignal::Term => Signal::TERM,
        StopSignal::Kill => Signal::KILL,
    };
    let sent = signal_all(&pids, getpid().as_raw_pid(), signal);
    tracing::debug!(app = %stop.app.wire(), signal = ?stop.signal, sent, "an app was force quit");
}

/// The processes behind these connections: those that came through the public socket, as
/// recorded when they connected.
fn pids_of(state: &State, clients: &[ClientKey]) -> BTreeSet<RawPid> {
    state.processes.of(clients).collect()
}

/// Sends `signal` to each of `pids`, except init and `own` (this process). How many were sent.
pub(crate) fn signal_all(pids: &BTreeSet<RawPid>, own: RawPid, signal: Signal) -> usize {
    pids.iter()
        .filter(|raw| **raw != own)
        .filter_map(|raw| Pid::from_raw(*raw))
        .filter(|pid| !pid.is_init())
        .filter(|pid| kill_process(*pid, signal).is_ok())
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::process::ExitStatusExt;
    use std::process::{Child, Command};

    fn sleeper() -> Child {
        Command::new("sleep")
            .arg("30")
            .spawn()
            .expect("sleep starts")
    }

    fn raw(child: &Child) -> RawPid {
        RawPid::try_from(child.id()).expect("a pid fits")
    }

    #[test]
    fn a_signal_reaches_only_the_named_process() {
        let (mut target, mut bystander) = (sleeper(), sleeper());
        let named = BTreeSet::from([raw(&target)]);
        assert_eq!(signal_all(&named, getpid().as_raw_pid(), Signal::KILL), 1);
        let status = target.wait().expect("the target ends");
        assert_eq!(status.signal(), Some(9));
        assert_eq!(bystander.try_wait().expect("polled"), None, "still running");
        bystander.kill().expect("cleanup");
        bystander.wait().expect("reaped");
    }

    #[test]
    fn init_and_this_process_are_never_signalled() {
        let own = getpid().as_raw_pid();
        let both = BTreeSet::from([1, own]);
        assert_eq!(signal_all(&both, own, Signal::TERM), 0);
    }
}
