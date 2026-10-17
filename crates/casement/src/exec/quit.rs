//! `Effect::Terminate` (P3-QU): force quit. The processes stopped are the ones on the other end
//! of the named connections' sockets (their credentials, read by the kernel when the client
//! connected), never a process found by name. Only connections that came through the public
//! socket are signalled: a sandboxed client's credentials are in another process namespace,
//! and the shell's and the agent tier's connections are not apps to kill. The one exception to
//! "never by name" is X11: xwayland-satellite's connection carries every X11 window and its
//! pid is no app's, so for an app on that connection the X server is asked which processes own
//! the windows whose class names the app (`xwayland::owners`).

use crate::state::State;
use crate::xwayland::owners::{self, Spared, XWindows};
use crate::xwayland::xres::XServer;
use casement_core::app::{StopSignal, Termination};
use rustix::process::{Pid, RawPid, Signal, getpid, kill_process};
use std::collections::BTreeSet;

/// Signals the processes of `stop`'s connections.
pub(super) fn terminate(state: &mut State, stop: Termination) {
    let signal = match stop.signal {
        StopSignal::Term => Signal::TERM,
        StopSignal::Kill => Signal::KILL,
    };
    let own = getpid().as_raw_pid();
    let satellite = state.xwayland.satellite_pid();
    // The connections' own processes through the handles held since they connected (CASB-20),
    // then the X11 programs the X server names, by pid, minus those already reached.
    let reached = state
        .processes
        .signal(&stop.clients, signal, own, satellite);
    let x11: BTreeSet<RawPid> = x11_pids_of(state, &stop, satellite)
        .difference(&reached)
        .copied()
        .collect();
    let sent = reached.len() + signal_all(&x11, own, signal);
    tracing::debug!(app = %stop.app.wire(), signal = ?stop.signal, sent, "an app was force quit");
}

/// The processes that own the X11 windows of `stop`'s app, when it has some and X11 is being
/// served. Only the processes whose windows name this app, so another X11 program is never
/// named; never the satellite, this process or init.
fn x11_pids_of(state: &State, stop: &Termination, satellite: Option<RawPid>) -> BTreeSet<RawPid> {
    let display = state.xwayland.display().filter(|_| !stop.x11.is_empty());
    let Some(display) = display else {
        return BTreeSet::new();
    };
    let windows = XServer::at(display).top_levels();
    let spared = Spared {
        satellite,
        own: getpid().as_raw_pid(),
    };
    owners::pids_of_app(&windows, &owners::name_of(&stop.app), &spared)
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
