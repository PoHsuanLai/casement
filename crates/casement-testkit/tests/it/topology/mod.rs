//! The soft multi-GPU topology (P1-D4): "display on A, client buffers from B", format steering,
//! GPU unplug. The jail fabricates the nodes (`--fabricate-gpus`); in process,
//! `FabricatedSysRoot` writes the same tree.

use casement_backend::DmabufMode;
use casement_testkit::gpu::{FabricatedSysRoot, GpuTopology, SoftGraphicsApi};
use casement_testkit::{Fixture, FixtureSpec};

#[test]
#[ignore = "P1-D4"]
fn detection_over_a_fabricated_tree_reports_the_topology() {
    let dir = tempfile::tempdir().expect("temp dir");
    let root = FabricatedSysRoot::build(&GpuTopology::single(), dir.path()).expect("tree");
    let _ = root;
    todo!("P1-D4: SysRenderNodes over the root reports one AMD GPU");
}

#[test]
#[ignore = "P1-D4"]
fn display_on_a_client_buffers_from_b() {
    let spec = FixtureSpec::desktop().with_dmabuf(DmabufMode::Soft);
    let _fixture = Fixture::start(spec).expect("fixture");
    let _api = SoftGraphicsApi::new(GpuTopology::single());
    todo!(
        "P1-D4: two soft GPUs; the copy path counters (A5) say which path ran; tagged from feedback"
    );
}

#[test]
#[ignore = "P1-D4"]
fn format_steering_offers_the_render_gpus_formats_first() {
    todo!("P1-D4: the feedback tranches of a two-GPU topology");
}

#[test]
#[ignore = "P1-D4"]
fn unplugging_a_gpu_falls_back_to_shm() {
    todo!("P1-D4: DeviceMissing on the dma-buf, the next commit as shm renders exactly");
}
