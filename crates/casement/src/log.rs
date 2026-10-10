//! Logging: smithay and casement log through `tracing`; the binary installs one subscriber.

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
