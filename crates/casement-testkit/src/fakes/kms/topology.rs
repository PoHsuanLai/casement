//! Fake topologies: recorded fixtures and synthetic ones, as `KmsResources`.

use casement_backend::tty::KmsResources;
use std::path::Path;

/// Why a fixture could not be loaded.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum KmsFixtureError {
    /// The file could not be read.
    #[error("{}: {source}", path.display())]
    Io {
        /// The file.
        path: std::path::PathBuf,
        /// Why.
        #[source]
        source: std::io::Error,
    },
    /// The file is not a recorded topology.
    #[error("{}: {source}", path.display())]
    Parse {
        /// The file.
        path: std::path::PathBuf,
        /// Why.
        #[source]
        source: serde_json::Error,
    },
}

/// The recorded topology in `path` (JSON of `KmsResources`).
pub fn load_fixture(path: &Path) -> Result<KmsResources, KmsFixtureError> {
    let _ = path;
    todo!("P1-D2: load_fixture")
}

/// Builders for the synthetic topologies the FakeKms suite runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SyntheticTopology {
    /// One laptop panel, `eDP-1`, 2880x1800 at 120 Hz, variable refresh capable.
    LaptopPanel,
    /// A panel and two external monitors on a dock.
    Dock,
    /// One monitor whose link retrains: it reports `Changed` after every commit.
    RetrainingMonitor,
    /// A connected connector that offers no mode at all.
    NoModes,
}

impl SyntheticTopology {
    /// The resources the topology offers.
    pub fn resources(self) -> KmsResources {
        todo!("P1-D2: SyntheticTopology::resources")
    }
}
