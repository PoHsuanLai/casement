//! casement's one test harness (11 §2.9, CONVENTIONS addendum 10): every protocol, scene and
//! loop test drives the real `casement::State` through [`Fixture`], on the headless backend,
//! with clients on socket pairs and the virtual clock as a calloop event source (A1). No test
//! assembles its own `State`, and nothing here sleeps: time moves only when a test advances it.
//!
//! Also here: the mintable test origins ([`TestOrigin`]), the fakes (session, udev, logind,
//! palmrest), the soft GPU topology (A3, A4), the golden environment ([`Env`]) and the typed
//! fuzz ops.

mod env;
pub mod fakes;
mod fixture;
pub mod fuzz;
pub mod gpu;
mod input;
mod origin;
mod output;
mod spec;
mod tty_world;
pub mod udmabuf;
pub mod writeback;

pub use env::Env;
pub use fixture::{Advanced, ClientId, Fixture, FixtureError, FrameCount, SceneDump, ShotTarget};
pub use input::FixtureInput;
pub use origin::TestOrigin;
pub use spec::FixtureSpec;
pub use tty_world::{CARD, TtyWorld};
