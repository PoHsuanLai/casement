//! The fake KMS (P1-Z; filled by P1-D2): `Kms` over recorded and synthetic topologies, with a
//! pixman image per connector. The recorded fixtures live in `crates/casement-testkit/fixtures/
//! kms/` (the person's optional `dev/hw/smoke.sh` records them; an agent never does); the
//! synthetic ones are built by [`SyntheticTopology`] (a laptop panel, a dock with two monitors,
//! a monitor that retrains its link, a connector with no modes).
//!
//! A [`FakeKms`] is a shared handle: the backend holds one clone as its `Box<dyn Kms>` and the
//! test keeps another, so a test plugs and unplugs connectors and reads back the commits, the
//! power and the gamma the backend made.

mod fake;
mod topology;

pub use fake::{FakeKms, FakeKmsOpener, KmsWorld};
pub use topology::{KmsFixtureError, SyntheticTopology, load_fixture};
