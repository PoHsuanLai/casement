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
