//! GPUs as pure facts and pure plans (P1-Z; filled by P1-D4).
//!
//! Detection (the backend's `RenderNodes`, reading an injected `SysRoot`) turns what the
//! machine has into [`GpuFacts`]; [`plan_for`] turns facts and the `gpu.render` setting
//! ([`GpuChoice`]) into a [`RenderPlan`] (which GPU renders, which scans each output out);
//! [`feedback_for`] turns the plan and each node's formats into the dma-buf feedback a client
//! is offered. Nothing here opens a device or reads `/sys`: hardware values come from
//! detection plus an override, never this machine (CONVENTIONS addendum 9).
//!
//! ```
//! use casement_core::gpu::{GpuChoice, plan_for};
//!
//! let plan = plan_for(&[], &GpuChoice::Auto);
//! assert!(plan.is_err(), "no GPU, no plan");
//! ```

mod bridge;
mod display;
mod feedback;
mod node;
mod plan;
mod policy;
mod route;

pub use bridge::bridge_format;
pub use display::{DisplayKind, keep_displays};
pub use feedback::{
    FeedbackPlan, FormatEntry, FormatTable, Fourcc, Modifier, Tranche, TrancheUse, feedback_for,
};
pub use node::{BootVga, ByPathId, DrmNodeId, GpuFacts, VendorFact};
pub use plan::{PlanRefusal, RenderPlan, plan_for};
pub use policy::GpuChoice;
pub use route::OutputRoute;
