//! Logging: smithay and casement log through `tracing`; the binary installs one subscriber.

/// Installs a stderr subscriber filtered by `CASEMENT_LOG` (default `info`). Called once by
/// the binary; tests install none.
pub fn init() {
    let filter = std::env::var("CASEMENT_LOG").unwrap_or_else(|_| "info".to_owned());
    let installed = tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::new(filter))
        .with_writer(std::io::stderr)
        .try_init();
    if let Err(error) = installed {
        eprintln!("casement: logging not installed: {error}");
    }
}
