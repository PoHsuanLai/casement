//! What a fixture starts with.

use crate::fakes::{MemoryLockMarker, MemorySpaceStore};
use crate::gpu::GpuTopology;
use casement::icons::IconSearch;
use casement::privacy::PrivacyFiles;
use casement::relax::AckPolicy;
use casement::time::WallClock;
use casement_backend::{RenderCostModel, SoftRenderer, VirtualOutputSpec};
use casement_core::settings::CasementSettings;
use std::time::Duration;

/// A fixture's outputs, renderer, GPUs, settings and frame cost.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixtureSpec {
    /// The outputs plugged at start (C27: the backend's own spec type).
    pub outputs: Vec<VirtualOutputSpec>,
    /// The renderer.
    pub renderer: SoftRenderer,
    /// The fake GPUs (bodies at P1).
    pub gpus: GpuTopology,
    /// The settings.
    pub settings: CasementSettings,
    /// What a frame costs.
    pub cost: RenderCostModel,
    /// Whether `zwp_linux_dmabuf_v1` is offered (a fake main device over `/dev/udmabuf`).
    pub dmabuf: bool,
    /// Whether a buffer attached before the first ack is an error (A37; wlcs needs it lenient).
    pub ack_policy: AckPolicy,
    /// The Spaces file the compositor starts from and stores into (A1). A second fixture started
    /// with a clone of the same store is a restart.
    pub space_store: MemorySpaceStore,
    /// The session lock's marker the compositor starts from and keeps (CASB-06). A second
    /// fixture started with a clone of the same marker is a restart while locked.
    pub lock_marker: MemoryLockMarker,
    /// Where the privacy book and the administrator's floor are kept, as real files in a
    /// scratch folder (none: nothing is read or kept, and answers last the run). A second
    /// fixture started on the same files is a restart.
    pub privacy: Option<PrivacyFiles>,
    /// The date kept answers are stamped with. It never moves.
    pub wall: WallClock,
    /// Whether Mission Control looks for app icons (off: none are drawn).
    pub icons: Option<IconSearch>,
}

impl FixtureSpec {
    /// One 1920x1080 output at 60 Hz and scale 1, pixman, one soft GPU, the default settings,
    /// every frame costing 4 ms.
    pub fn desktop() -> FixtureSpec {
        FixtureSpec {
            outputs: vec![
                VirtualOutputSpec::parse("HEADLESS-1:1920x1080@60000:120:0,0")
                    .expect("a literal spec in the grammar"),
            ],
            renderer: SoftRenderer::Pixman,
            gpus: GpuTopology::single(),
            settings: CasementSettings::defaults(),
            cost: RenderCostModel::Fixed(Duration::from_millis(4)),
            dmabuf: false,
            ack_policy: AckPolicy::Strict,
            space_store: MemorySpaceStore::empty(),
            lock_marker: MemoryLockMarker::absent(),
            privacy: None,
            wall: WallClock::epoch(),
            icons: None,
        }
    }

    /// The same, keeping the privacy book and reading the floor in `files`.
    pub fn with_privacy(self, files: PrivacyFiles) -> FixtureSpec {
        FixtureSpec {
            privacy: Some(files),
            ..self
        }
    }

    /// The same, with the date fixed at `wall`.
    pub fn with_wall(self, wall: WallClock) -> FixtureSpec {
        FixtureSpec { wall, ..self }
    }

    /// The same, looking for app icons as `icons` says.
    pub fn with_icons(self, icons: IconSearch) -> FixtureSpec {
        FixtureSpec {
            icons: Some(icons),
            ..self
        }
    }

    /// The same, with dma-buf offered to clients (or not).
    pub fn with_dmabuf(self, dmabuf: bool) -> FixtureSpec {
        FixtureSpec { dmabuf, ..self }
    }

    /// The same, starting from (and storing into) this Spaces file.
    pub fn with_space_store(self, space_store: MemorySpaceStore) -> FixtureSpec {
        FixtureSpec {
            space_store,
            ..self
        }
    }

    /// The same, starting from (and keeping) this lock marker.
    pub fn with_lock_marker(self, lock_marker: MemoryLockMarker) -> FixtureSpec {
        FixtureSpec {
            lock_marker,
            ..self
        }
    }

    /// The same, with this ack policy.
    pub fn with_ack_policy(self, ack_policy: AckPolicy) -> FixtureSpec {
        FixtureSpec { ack_policy, ..self }
    }

    /// The same, with these outputs.
    pub fn with_outputs(self, specs: &[&str]) -> FixtureSpec {
        FixtureSpec {
            outputs: specs
                .iter()
                .map(|spec| {
                    VirtualOutputSpec::parse(spec)
                        .unwrap_or_else(|error| panic!("output spec {spec}: {error}"))
                })
                .collect(),
            ..self
        }
    }

    /// The same, with this renderer.
    pub fn with_renderer(self, renderer: SoftRenderer) -> FixtureSpec {
        FixtureSpec { renderer, ..self }
    }
}
