//! The arrangement: where layout puts everything, and what it could not honour.

#[allow(
    clippy::module_inception,
    reason = "the freeze plan names the record file after its concept"
)]
mod arrangement;
mod configure_need;
mod overview_slot;
mod report;
mod space_target;
mod space_thumb;
mod window_target;

pub use arrangement::Arrangement;
pub use configure_need::ConfigureNeed;
pub use overview_slot::{OverviewSlot, SlotKind};
pub use report::LayoutReport;
pub use space_target::SpaceTarget;
pub use space_thumb::SpaceThumb;
pub use window_target::WindowTarget;
