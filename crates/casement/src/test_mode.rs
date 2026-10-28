//! The hooks of the vkms test mode (P1-V2): what `casement --backend tty --drm-node PATH` adds
//! for `dev/vkms/accept.sh`, and nothing else. A test mode, not a feature: it is built only when
//! `--drm-node` is given.
//!
//! * `SIGUSR1` and `SIGUSR2` are the session paused and activated, sent into the tty session
//!   through its [`SleepLink`] (the path a suspend takes), so a "VT switch" runs the same
//!   backend code a real one does.
//! * `SIGHUP` asks for one writeback frame; a few flips later the raw buffer is written to the
//!   `--writeback-raw` path and a line `WIDTH HEIGHT PITCH FOURCC` to its `.meta` (written
//!   last, so a reader that waits for the meta line finds the raw file whole).
//! * `--kms-report` gets [`KmsReport::json_line`](casement_backend::tty::KmsReport::json_line)
//!   appended once a second.
//!
//! [`Hooks`] holds the decisions and the files and takes the backend and the clock as
//! arguments, so it is tested over the fakes with no signal and no device; [`TestHooks`] is
//! `Hooks` plus the signal thread.

use crate::cli::{DirectTty, TtyMode};
use crate::state::State;
use casement_backend::BackendKind;
use casement_backend::tty::{
    RawCapture, ReportPacer, SleepLink, SleepSettled, TtyBackend, WritebackError,
};
use casement_core::time::Mono;
use casement_ipc::signals::{SignalError, SignalHandler, SignalWatch};
use std::ffi::OsString;
use std::fs::OpenOptions;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::Duration;

/// How long a pause waits for the backend to let go of its devices.
const LET_GO_WITHIN: Duration = Duration::from_secs(2);

/// How often the loop turns in this mode even when nothing happens, so the report is written
/// while the seat is lent and no frame flips.
pub const TURN: Duration = Duration::from_millis(250);

/// Why the hooks could not be attached.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum HookError {
    /// The signal watcher could not start.
    #[error("{0}")]
    Signals(#[from] SignalError),
    /// The session has no way to be paused from outside.
    #[error("the session cannot be paused from outside")]
    NoSessionLink,
}

/// Appends `line` and a newline to the file at `path`, creating it.
pub fn append_line(path: &Path, line: &str) -> io::Result<()> {
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    file.write_all(format!("{line}\n").as_bytes())
}

/// `path` with `.meta` added to its file name.
pub fn meta_path(path: &Path) -> PathBuf {
    let mut name = OsString::from(path.as_os_str());
    name.push(".meta");
    PathBuf::from(name)
}

/// Writes the frame's raw buffer to `path` and its description to `path.meta`, the
/// description last.
pub fn write_capture(path: &Path, frame: &RawCapture) -> io::Result<()> {
    std::fs::write(path, &frame.bytes)?;
    std::fs::write(meta_path(path), format!("{}\n", frame.meta_line()))
}

/// The files the hooks write, and the clock of the report.
#[derive(Debug)]
pub struct Hooks {
    report: Option<PathBuf>,
    pacer: ReportPacer,
    writeback_raw: Option<PathBuf>,
    captures: Receiver<()>,
}

impl Hooks {
    /// The hooks for `direct`, and the sender a `SIGHUP` is told through.
    pub fn new(direct: &DirectTty) -> (Hooks, Sender<()>) {
        let (asks, captures) = channel();
        let hooks = Hooks {
            report: direct.report.clone(),
            pacer: ReportPacer::new(),
            writeback_raw: direct.writeback_raw.clone(),
            captures,
        };
        (hooks, asks)
    }

    /// One turn: the report when it is due at `now`, a writeback job when `SIGHUP` asked for
    /// one, and the frame of the job queued earlier when it is ready. A failure is logged and
    /// the run goes on.
    pub fn turn(&mut self, tty: &mut TtyBackend, now: Mono) {
        if let Some(path) = &self.report
            && self.pacer.check(now)
            && let Err(error) = append_line(path, &tty.report().json_line())
        {
            tracing::warn!(path = %path.display(), %error, "the KMS report was not written");
        }
        let asked = self.captures.try_iter().count() > 0;
        if asked && self.writeback_raw.is_some() {
            self.request(tty);
        }
        if let (Some(path), Some(done)) = (&self.writeback_raw, tty.take_writeback()) {
            match done {
                Ok(frame) => match write_capture(path, &frame) {
                    Ok(()) => {
                        tracing::info!(path = %path.display(), "a writeback frame was written")
                    }
                    Err(error) => {
                        tracing::warn!(path = %path.display(), %error, "the writeback frame was not written")
                    }
                },
                Err(error) => tracing::warn!(%error, "the writeback frame was not read"),
            }
        }
    }

    /// Queues the job `SIGHUP` asked for.
    fn request(&self, tty: &mut TtyBackend) {
        match tty.request_writeback() {
            Ok(()) => {}
            Err(error @ (WritebackError::Busy | WritebackError::SeatLent)) => {
                tracing::info!(%error, "SIGHUP: no writeback now")
            }
            Err(error) => tracing::warn!(%error, "SIGHUP: the writeback job was not queued"),
        }
    }
}

/// The signal thread's side: pauses and activates the session, and tells the loop about
/// `SIGHUP`.
struct Edge {
    link: SleepLink,
    captures: Sender<()>,
    /// How long a pause waits for the backend to let go.
    wait: Duration,
}

impl SignalHandler for Edge {
    fn paused(&mut self) {
        match self.link.paused(self.wait) {
            SleepSettled::Done | SleepSettled::Gone => {}
            _ => tracing::warn!("the session was paused but the devices were not let go in time"),
        }
    }

    fn activated(&mut self) {
        self.link.activated();
    }

    fn capture(&mut self) {
        // The loop may have left; then there is nobody to ask.
        let _ = self.captures.send(());
    }
}

/// The hooks and the thread that hears the signals; dropping it stops the thread.
#[derive(Debug, Default)]
pub struct TestHooks {
    running: Option<(Hooks, SignalWatch)>,
}

impl TestHooks {
    /// The hooks for `mode`: none for a real seat; for `--drm-node`, the signals are heard
    /// and the files written.
    pub fn attach(state: &mut State, mode: &TtyMode) -> Result<TestHooks, HookError> {
        let (TtyMode::Direct(direct), BackendKind::Tty(tty)) = (mode, &mut state.backend) else {
            return Ok(TestHooks::default());
        };
        let link = tty
            .seams
            .session
            .sleep_link()
            .ok_or(HookError::NoSessionLink)?;
        let (hooks, captures) = Hooks::new(direct);
        let watch = SignalWatch::start(Edge {
            link,
            captures,
            wait: LET_GO_WITHIN,
        })?;
        Ok(TestHooks {
            running: Some((hooks, watch)),
        })
    }

    /// One loop turn of the hooks (nothing without `--drm-node`).
    pub fn turn(&mut self, state: &mut State) {
        let Some((hooks, _)) = &mut self.running else {
            return;
        };
        let now = state.clock.now();
        if let BackendKind::Tty(tty) = &mut state.backend {
            hooks.turn(tty, now);
        }
    }

    /// How long the loop may sleep: [`TURN`] in this mode, forever otherwise.
    pub fn wake(&self) -> Option<Duration> {
        self.running.as_ref().map(|_| TURN)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use casement_backend::tty::{DirectSession, SessionNote, TtySession};
    use casement_core::geometry::PhysicalSize;
    use casement_core::gpu::Fourcc;

    fn scratch(name: &str) -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().expect("a scratch directory");
        let path = dir.path().join(name);
        (dir, path)
    }

    #[test]
    fn the_signals_become_session_notes_and_a_capture_request() {
        let mut session = DirectSession::new(|| {});
        let link = session.sleep_link().expect("the edge's end");
        let (captures, asked) = channel();
        let mut edge = Edge {
            link,
            captures,
            wait: Duration::ZERO,
        };
        edge.paused();
        assert_eq!(session.notes(), [SessionNote::Paused]);
        edge.activated();
        assert_eq!(session.notes(), [SessionNote::Activated]);
        assert!(asked.try_recv().is_err());
        edge.capture();
        assert!(asked.try_recv().is_ok());
    }

    #[test]
    fn lines_are_appended_one_per_call() {
        let (_dir, path) = scratch("report.jsonl");
        append_line(&path, r#"{"flips":1}"#).expect("first");
        append_line(&path, r#"{"flips":2}"#).expect("second");
        assert_eq!(
            std::fs::read_to_string(&path).expect("read"),
            "{\"flips\":1}\n{\"flips\":2}\n"
        );
    }

    #[test]
    fn a_capture_is_the_raw_bytes_and_a_meta_line() {
        let (_dir, path) = scratch("writeback.raw");
        let frame = RawCapture::new(
            PhysicalSize { w: 2, h: 1 },
            8,
            Fourcc(0x3432_5258),
            vec![1, 2, 3, 4, 5, 6, 7, 8],
        );
        write_capture(&path, &frame).expect("written");
        assert_eq!(std::fs::read(&path).expect("raw"), [1, 2, 3, 4, 5, 6, 7, 8]);
        assert_eq!(
            std::fs::read_to_string(meta_path(&path)).expect("meta"),
            "2 1 8 XR24\n"
        );
        assert!(meta_path(&path).ends_with("writeback.raw.meta"));
    }
}
