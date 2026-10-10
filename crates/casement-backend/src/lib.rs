//! casement's backends and device seams.
//!
//! P0 has the headless backend (virtual outputs at any mode and scale, a vblank source on the
//! session clock, a render-cost model, [`VirtualInput`]) and winit (nested, for the user's own
//! feel checks; never run by an agent). P1-Z adds the tty seams ([`tty::Kms`],
//! [`tty::ScanoutSurface`], [`tty::TtySession`], [`tty::DeviceSource`], [`input::InputDevices`],
//! [`gpu::RenderNodes`]) and their thin real adapters, the only files that name the device
//! crates (row B7). The adapters compile only under the features `tty` and `libinput` (off by
//! default; the `casement` binary turns them on), so headless consumers build with no system
//! libraries.
//!
//! Nothing here names `casement_wayland` or a `smithay::wayland` module but dmabuf (row B5).
//!
//! ```
//! use casement_backend::{BackendError, VirtualOutputSpec};
//!
//! let spec = VirtualOutputSpec::parse("HEADLESS-1:1920x1080@60000:120:0,0").unwrap();
//! assert_eq!(spec.mode.size.w, 1920);
//! let refused = VirtualOutputSpec::parse("nonsense");
//! assert!(matches!(refused, Err(BackendError::BadSpec { .. })));
//! ```

mod backend;
pub mod gpu;
pub mod headless;
pub mod input;
pub mod session;
pub mod tty;
mod winit;

pub use backend::{BackendChoice, BackendError, BackendKind};
pub use headless::{
    ConnectorEvent, DmabufMode, EdidSource, FixtureName, FramePixels, GpuId, HeadlessBackend,
    HeadlessConfig, OutputChange, PresentedFrame, RenderCostModel, RgbaFrame, SoftRenderer,
    SpecError, VblankSource, VirtualOutputSpec,
};
pub use input::{DeviceName, DeviceSpec, VirtualDevice, VirtualInput};
pub use session::{SessionFault, SessionSeam};
pub use tty::TtyBackend;
pub use winit::WinitBackend;
