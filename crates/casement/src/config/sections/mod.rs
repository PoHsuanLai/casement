//! The document's sections: one file per `CasementSettings` part. Each key has a
//! `#[settings(...)]` row (ds-settings' `SettingsSchema` derive), which is the schema detent and
//! the Settings app read; each section's `Default` is the model's own default through
//! `convert`, so no default is written twice.

mod bindings;
mod gestures;
mod gpu;
mod keyboard;
mod outputs;
mod pointer;
mod spaces;
mod touchpad;
mod windows;

pub use bindings::BindingsSection;
pub use gestures::GesturesSection;
pub use gpu::GpuSection;
pub use keyboard::KeyboardSection;
pub use outputs::{FirstSight, OutputsSection};
pub use pointer::PointerSection;
pub use spaces::SpacesSection;
pub use touchpad::TouchpadSection;
pub use windows::WindowsSection;
