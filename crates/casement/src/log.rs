//! Logging: smithay and casement log through `tracing`; the binary installs one subscriber.

/// The filter when `CASEMENT_LOG` is unset: `info`, with smithay's renderer lines that repeat for
/// every surface at every frame when an import fails capped at errors, and its raw EGL entry
/// point trace off. casement logs the cause once itself (`frame_faults`).
pub const DEFAULT_FILTER: &str = "info,smithay::backend::egl::ffi=off,smithay::backend::renderer::element=error,smithay::backend::renderer::utils=error";

/// Installs a stderr subscriber filtered by `filter` (an `EnvFilter` directive list such as
/// `info` or `casement=debug`; the binary reads it from `CASEMENT_LOG`). Called once by the
/// binary; tests install none.
pub fn init(filter: &str) {
    let installed = tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::new(filter))
        .with_writer(std::io::stderr)
        .try_init();
    if let Err(error) = installed {
        eprintln!("casement: logging not installed: {error}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::timing::{Sink, Stamp, WindowReady};
    use std::sync::{Arc, Mutex};

    /// A writer that keeps what the subscriber prints.
    #[derive(Clone, Default)]
    struct Capture(Arc<Mutex<Vec<u8>>>);

    impl std::io::Write for Capture {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0
                .lock()
                .expect("not poisoned")
                .extend_from_slice(bytes);
            Ok(bytes.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    /// The timing lines pass the default filter (`info`) and print as the event name followed by
    /// its `key=value` fields on one line: what the desktop's scripts read in the log file.
    #[test]
    fn a_timing_line_passes_the_default_filter_and_stays_on_one_line() {
        let capture = Capture::default();
        let writer = capture.clone();
        let subscriber = tracing_subscriber::fmt()
            .with_env_filter(tracing_subscriber::EnvFilter::new(DEFAULT_FILTER))
            .with_writer(move || writer.clone())
            .without_time()
            .finish();
        let line = WindowReady {
            app_id: crate::timing::Token::of("foot"),
            pid: casement_core::Pid(4321),
            connected_at: Stamp(100),
            first_commit_at: Stamp(250),
            first_frame_at: Stamp(900),
            settled_at: Stamp(1_400),
        }
        .line();
        tracing::subscriber::with_default(subscriber, || Sink::Journal.write(line));
        let printed =
            String::from_utf8(capture.0.lock().expect("not poisoned").clone()).expect("utf-8");
        assert_eq!(
            printed,
            " INFO timing: window-ready app_id=foot pid=4321 connected_at=100 first_commit_at=250 first_frame_at=900 settled_at=1400\n"
        );
    }
}
