//! The GPU detection seam (P1-Z; filled by P1-D4): which GPUs this machine has, and a handle
//! to open one.
//!
//! Detection reads an injected [`SysRoot`] (A6): the real adapter (`render_nodes_real.rs`, feature
//! `tty`) is handed `/sys` and `/dev`; a test is handed a fabricated tree
//! (`casement_testkit::gpu::FabricatedSysRoot`). The plan logic over the facts is pure, in
//! `casement_core::gpu`.
//!
//! ```no_run
//! use casement_backend::gpu::{RenderNodes, SysRoot};
//! use casement_core::gpu::{GpuChoice, plan_for};
//!
//! fn plan(nodes: &dyn RenderNodes) {
//!     let _ = plan_for(&nodes.nodes(), &GpuChoice::Auto);
//! }
//! let _ = SysRoot::host();
//! ```

use casement_core::gpu::{DrmNodeId, GpuFacts, PlanRefusal};
use std::os::fd::OwnedFd;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Where `/sys` and `/dev` are: the host's, or a tree a test fabricated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SysRoot {
    sys: PathBuf,
    dev: PathBuf,
}

impl SysRoot {
    /// The host's `/sys` and `/dev`.
    pub fn host() -> SysRoot {
        SysRoot {
            sys: PathBuf::from("/sys"),
            dev: PathBuf::from("/dev"),
        }
    }

    /// A tree at `root` with `sys/` and `dev/` under it.
    pub fn under(root: &Path) -> SysRoot {
        SysRoot {
            sys: root.join("sys"),
            dev: root.join("dev"),
        }
    }

    /// The `/sys` directory.
    pub fn sys(&self) -> &Path {
        &self.sys
    }

    /// The `/dev` directory.
    pub fn dev(&self) -> &Path {
        &self.dev
    }
}

/// An open render node.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct GpuHandle {
    /// The node.
    pub node: DrmNodeId,
    /// Its descriptor, shared by whoever renders on it.
    pub fd: Arc<OwnedFd>,
}

impl GpuHandle {
    /// A handle on `node` over `fd`.
    pub fn new(node: DrmNodeId, fd: OwnedFd) -> GpuHandle {
        GpuHandle {
            node,
            fd: Arc::new(fd),
        }
    }
}

/// Why a GPU call failed.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum GpuError {
    /// The node is not one detection found.
    #[error("no such GPU {0:?}")]
    UnknownNode(DrmNodeId),
    /// A file under the sys root could not be read.
    #[error("{}: {source}", path.display())]
    Sys {
        /// The file.
        path: PathBuf,
        /// Why it failed.
        #[source]
        source: std::io::Error,
    },
    /// The node could not be opened.
    #[error("opening {node:?}: {source}")]
    Open {
        /// The node.
        node: DrmNodeId,
        /// Why it failed.
        #[source]
        source: std::io::Error,
    },
    /// The GPU went away while in use (an unplugged eGPU).
    #[error("GPU {0:?} was unplugged")]
    Unplugged(DrmNodeId),
    /// No format table is available for the node (a recorded table nobody loaded).
    #[error("no format table for GPU {0:?}")]
    NoFormats(DrmNodeId),
    /// No plan could be made.
    #[error("plan: {0}")]
    Plan(#[from] PlanRefusal),
}

/// The machine's GPUs.
pub trait RenderNodes: std::fmt::Debug {
    /// What detection found, render GPUs only, in node order.
    fn nodes(&self) -> Vec<GpuFacts>;

    /// Opens a render node.
    fn open(&mut self, node: &DrmNodeId) -> Result<GpuHandle, GpuError>;
}
