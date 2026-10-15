//! The fakes: what stands in for the session, udev, logind, KMS, scanout, libinput's
//! configuration, the Spaces file, sill's D-Bus methods, the service manager's notifications, the account service's Spaces, the process table, the control groups
//! of bus peers and palmrest inside the jail.

mod cgroups;
mod input_devices;
mod input_feed;
pub mod kms;
pub mod logind;
mod notify;
mod palmrest;
mod scanout;
mod session;
mod shell_calls;
mod space_registry;
mod space_store;
mod spawner;
mod udev;
mod writeback;

pub use cgroups::FakeCgroups;
pub use input_devices::FakeInputDevices;
pub use input_feed::FakeInputFeed;
pub use kms::{
    CaptureGpuWorld, FakeCaptureGpu, FakeKms, FakeKmsOpener, GpuBehaviour, KmsFixtureError,
    KmsWorld, LinkBehaviour, SyntheticTopology, edid, load_fixture,
};
pub use logind::{
    FakeLogind, IdleHint, LogindCall, LogindError, PauseKind, PrivateBus, PrivateBusError,
    SleepPhase,
};
pub use notify::FakeNotify;
pub use palmrest::FakePalmrest;
pub use scanout::{FakeScanout, GlassHandle, Pacing};
pub use session::{FakeSession, SessionFixture, SharedSession};
pub use shell_calls::FakeShellCalls;
pub use space_registry::FakeSpaceRegistry;
pub use space_store::MemorySpaceStore;
pub use spawner::{FakeSpawner, StartedProgram};
pub use udev::{FakeUdev, SharedUdev};
pub use writeback::{FakeWriteback, WritebackWorld};
