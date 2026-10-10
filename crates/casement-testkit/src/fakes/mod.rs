//! The fakes: what stands in for the session, udev, logind, KMS, scanout, libinput's
//! configuration, the Spaces file, sill's D-Bus methods, the process table and palmrest inside
//! the jail.

mod input_devices;
pub mod kms;
pub mod logind;
mod palmrest;
mod scanout;
mod session;
mod shell_calls;
mod space_store;
mod spawner;
mod udev;

pub use input_devices::FakeInputDevices;
pub use kms::{FakeKms, FakeKmsOpener, KmsWorld, SyntheticTopology};
pub use logind::{FakeLogind, LogindCall, LogindError};
pub use palmrest::FakePalmrest;
pub use scanout::FakeScanout;
pub use session::{FakeSession, SessionFixture};
pub use shell_calls::FakeShellCalls;
pub use space_store::MemorySpaceStore;
pub use spawner::{FakeSpawner, StartedProgram};
pub use udev::FakeUdev;
