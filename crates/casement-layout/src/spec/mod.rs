//! The scene spec: what the model asks layout to arrange.

mod mode;
mod output;
mod scene;
mod space;
mod window;

pub use mode::{OverviewKind, SceneMode};
pub use output::OutputSpec;
pub use scene::SceneSpec;
pub use space::{SpaceArrangement, SpaceSpec};
pub use window::{StackIndex, WindowPlacement, WindowSpec};
