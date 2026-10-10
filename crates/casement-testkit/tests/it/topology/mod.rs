//! The soft multi-GPU topology (P1-D4): detection over a fabricated tree, the plan and its
//! override, "display on A, client buffers from B" through smithay's `GpuManager` with the copy
//! path asserted (A5), format steering, GPU unplug. Everything runs in process on the fabricated
//! tree and `/dev/udmabuf`; the jail's `--fabricate-gpus` nodes are the same shape.

mod support;

use casement_backend::GpuId;
use casement_backend::gpu::{
    CrossSupport, GpuError, GpuStack, ImportRoute, RenderNodes, SysRenderNodes, format_table,
    route_for, route_with, tag_node,
};
use casement_core::gpu::{
    ByPathId, DrmNodeId, GpuChoice, PlanRefusal, RenderPlan, TrancheUse, feedback_for, plan_for,
};
use casement_testkit::gpu::{
    ARGB8888, CrossImport, FabricatedSysRoot, GpuTopology, SoftGraphicsApi, XRGB8888,
};
use smithay::backend::allocator::Fourcc;
use smithay::backend::drm::DrmNode;
use smithay::backend::renderer::multigpu::GpuManager;
use smithay::backend::renderer::{ImportDma, ImportMem};
use smithay::utils::Size;
use support::{TestBuffer, draw};

const SIZE: (u32, u32) = (256, 128);

fn tree(topology: &GpuTopology) -> (tempfile::TempDir, SysRenderNodes) {
    let dir = tempfile::tempdir().expect("temp dir");
    let root = FabricatedSysRoot::build(topology, dir.path()).expect("tree");
    (dir, SysRenderNodes::new(root))
}

fn drm_node(id: DrmNodeId) -> DrmNode {
    DrmNode::from_dev_id(rustix::fs::makedev(id.major, id.minor)).expect("a render node")
}

fn pair(cross: CrossImport) -> GpuTopology {
    GpuTopology::pair(cross).expect("the pair topology")
}

#[test]
fn detection_over_a_fabricated_tree_reports_the_topology() {
    let topology = GpuTopology::single();
    let (_dir, mut nodes) = tree(&topology);
    let found = nodes.nodes();
    assert_eq!(found, topology.facts().expect("facts"));
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].vendor, casement_core::gpu::VendorFact::Amd);
    assert_eq!(found[0].node, DrmNodeId::new(226, 128));
    nodes.open(&found[0].node).expect("the render node opens");
    assert!(matches!(
        nodes.open(&DrmNodeId::new(226, 140)),
        Err(GpuError::UnknownNode(_))
    ));
}

#[test]
fn detection_reads_connectors_and_boot_gpu_of_a_pair() {
    let topology = pair(CrossImport::Direct);
    let (_dir, nodes) = tree(&topology);
    let found = nodes.nodes();
    assert_eq!(found, topology.facts().expect("facts"));
    let plan = plan_for(&found, &GpuChoice::Auto).expect("a plan");
    assert_eq!(plan.render, DrmNodeId::new(226, 128), "the firmware's GPU");
    assert_eq!(plan.scanout.len(), 2);
    assert_eq!(
        plan.scanout.values().copied().collect::<Vec<_>>(),
        [DrmNodeId::new(226, 129), DrmNodeId::new(226, 128)],
        "HDMI-A-1 on B, eDP-1 on A (sorted by connector name)"
    );
}

#[test]
fn the_override_picks_the_render_gpu_and_the_stack_opens_what_the_plan_names() {
    let topology = pair(CrossImport::Direct);
    let (_dir, mut nodes) = tree(&topology);
    let by_path = topology.gpus[1].by_path().expect("id");
    let stack = GpuStack::start(&mut nodes, &GpuChoice::ByPath(by_path)).expect("a stack");
    assert_eq!(stack.plan.render, DrmNodeId::new(226, 129));
    let opened: Vec<_> = stack.handles.iter().map(|handle| handle.node).collect();
    assert_eq!(opened, [DrmNodeId::new(226, 129), DrmNodeId::new(226, 128)]);

    let auto = GpuStack::start(&mut nodes, &GpuChoice::Auto).expect("a stack");
    assert_eq!(auto.plan.render, DrmNodeId::new(226, 128));

    let unknown = ByPathId::parse("0000:ff:00.0").expect("id");
    let refused = GpuStack::start(&mut nodes, &GpuChoice::ByPath(unknown.clone()));
    assert!(matches!(
        refused,
        Err(GpuError::Plan(PlanRefusal::UnknownGpu(id))) if id == unknown
    ));
}

#[test]
fn format_steering_offers_the_render_gpus_formats_first() {
    let topology = pair(CrossImport::Direct);
    let (_dir, mut nodes) = tree(&topology);
    let stack = GpuStack::start(&mut nodes, &GpuChoice::Auto).expect("a stack");
    let mut source = topology.clone();
    let table = format_table(&mut source, &nodes, &stack.plan).expect("a table");
    let feedback = feedback_for(&stack.plan, &table);
    let shape: Vec<_> = feedback
        .tranches
        .iter()
        .map(|tranche| {
            let codes: Vec<u32> = tranche.formats.iter().map(|entry| entry.format.0).collect();
            (tranche.target.minor, tranche.purpose, codes)
        })
        .collect();
    assert_eq!(feedback.main_device, DrmNodeId::new(226, 128));
    assert_eq!(
        shape,
        vec![
            (128, TrancheUse::Render, vec![XRGB8888]),
            (129, TrancheUse::Import, vec![ARGB8888]),
        ],
        "A renders XRGB8888; ARGB8888 only B offers, after a copy"
    );
    // Asking for a node detection does not know is an error, not an empty table.
    let stray = RenderPlan::new(DrmNodeId::new(226, 150), Default::default());
    assert!(matches!(
        format_table(&mut source, &nodes, &stray),
        Err(GpuError::UnknownNode(_))
    ));
}

#[test]
fn import_routes_follow_the_plan() {
    let topology = pair(CrossImport::Direct);
    let (_dir, nodes) = tree(&topology);
    let plan = plan_for(&nodes.nodes(), &GpuChoice::Auto).expect("a plan");
    let (a, b) = (DrmNodeId::new(226, 128), DrmNodeId::new(226, 129));
    assert_eq!(route_for(&a, &plan), ImportRoute::Local);
    assert_eq!(route_for(&b, &plan), ImportRoute::Direct);
    assert_eq!(
        route_with(&b, &plan, CrossSupport::CopyOnly),
        ImportRoute::Copy
    );
    assert_eq!(
        route_with(&b, &plan, CrossSupport::Refuse),
        ImportRoute::Refused
    );
    assert_eq!(
        route_for(&DrmNodeId::new(226, 150), &plan),
        ImportRoute::Refused
    );
}

/// Display on A, a client buffer allocated on B, tagged from the feedback tranche that offers
/// its format (A5); returns B's and A's counters after one commit.
fn display_on_a_client_from_b(
    cross: CrossImport,
) -> (
    casement_backend::gpu::CopyCounters,
    casement_backend::gpu::CopyCounters,
) {
    let topology = pair(cross);
    let (_dir, mut nodes) = tree(&topology);
    let stack = GpuStack::start(&mut nodes, &GpuChoice::Auto).expect("a stack");
    let mut source = topology.clone();
    let table = format_table(&mut source, &nodes, &stack.plan).expect("a table");
    let feedback = feedback_for(&stack.plan, &table);

    // The client allocates ARGB8888: the tranche that offers it is B's.
    let offering = feedback
        .tranches
        .iter()
        .find(|tranche| {
            tranche
                .formats
                .iter()
                .any(|entry| entry.format.0 == ARGB8888)
        })
        .expect("a tranche offers ARGB8888");
    let b = tag_node(&stack.plan, &offering.target);
    assert_eq!(
        b,
        DrmNodeId::new(226, 129),
        "tagged with B, not the render GPU"
    );
    let a = stack.plan.render;

    let (w, h) = SIZE;
    let client = TestBuffer::new(w, h, Some(drm_node(b))).expect("the client buffer");
    let mut out = TestBuffer::new(w, h, Some(drm_node(a))).expect("the output buffer");
    assert_eq!(
        client.dmabuf.node().map(|node| node.dev_id()),
        Some(rustix::fs::makedev(b.major, b.minor)),
        "the buffer carries its source node"
    );
    client.fill(3).expect("fill");

    let mut gpu = GpuManager::new(SoftGraphicsApi::new(topology)).expect("a GPU manager");
    {
        let mut renderer = gpu
            .renderer(&drm_node(a), &drm_node(a), Fourcc::Argb8888)
            .expect("a renderer");
        let texture = renderer
            .import_dmabuf(&client.dmabuf, None)
            .expect("the foreign dma-buf imports");
        draw(&mut renderer, &mut out, &texture).expect("the frame draws");
    }
    assert_eq!(
        out.mismatches(3).expect("read back"),
        0,
        "byte-exact across the copy"
    );
    let counters_of = |gpu: &mut GpuManager<SoftGraphicsApi>, id: u32| {
        gpu.devices()
            .expect("devices")
            .find(|device| device.id() == GpuId(id))
            .map(|device| device.counters().snapshot())
            .expect("the device is enumerated")
    };
    (counters_of(&mut gpu, 1), counters_of(&mut gpu, 0))
}

#[test]
fn display_on_a_client_buffers_from_b_take_the_dma_shadow_copy() {
    let (b, a) = display_on_a_client_from_b(CrossImport::Direct);
    assert_eq!(b.bind_dmabuf, 1, "B rendered the shadow buffer once");
    assert_eq!(b.copy_texture, 0, "no CPU copy");
    assert_eq!(a.import_memory, 0, "no CPU copy");
    assert!(a.import_dmabuf >= 1, "A imported the shadow buffer");
}

#[test]
fn display_on_a_client_buffers_from_b_take_the_cpu_copy_when_a_cannot_import() {
    let (b, a) = display_on_a_client_from_b(CrossImport::CopyOnly);
    assert_eq!(b.copy_texture, 1, "B's texture was read back once");
    assert_eq!(a.import_memory, 1, "A uploaded it once");
    assert_eq!(b.bind_dmabuf, 0, "no shadow buffer");
}

#[test]
fn a_shm_client_renders_exactly_beside_a_foreign_gpu() {
    let (w, h) = SIZE;
    let mut gpu =
        GpuManager::new(SoftGraphicsApi::new(pair(CrossImport::Direct))).expect("manager");
    let a = drm_node(DrmNodeId::new(226, 128));
    let mut out = TestBuffer::new(w, h, Some(a)).expect("the output buffer");
    let mut renderer = gpu.renderer(&a, &a, Fourcc::Argb8888).expect("a renderer");
    let bytes = TestBuffer::shm_bytes(w, h, 5);
    let texture = renderer
        .import_memory(
            &bytes,
            Fourcc::Argb8888,
            Size::from((w as i32, h as i32)),
            false,
        )
        .expect("shm imports");
    draw(&mut renderer, &mut out, &texture).expect("the frame draws");
    drop(renderer);
    assert_eq!(out.mismatches(5).expect("read back"), 0);
}

#[test]
fn unplugging_a_gpu_falls_back_to_shm() {
    let topology = pair(CrossImport::Direct);
    let (w, h) = SIZE;
    let (a, b) = (
        drm_node(DrmNodeId::new(226, 128)),
        drm_node(DrmNodeId::new(226, 129)),
    );
    let client = TestBuffer::new(w, h, Some(b)).expect("the client buffer");
    let mut out = TestBuffer::new(w, h, Some(a)).expect("the output buffer");
    client.fill(1).expect("fill");
    let mut gpu = GpuManager::new(SoftGraphicsApi::new(topology)).expect("manager");
    {
        let mut renderer = gpu.renderer(&a, &a, Fourcc::Argb8888).expect("a renderer");
        let texture = renderer
            .import_dmabuf(&client.dmabuf, None)
            .expect("imports");
        draw(&mut renderer, &mut out, &texture).expect("draws");
    }
    assert_eq!(
        out.mismatches(1).expect("read back"),
        0,
        "before the unplug"
    );

    gpu.as_mut().unplug(GpuId(1));
    let mut renderer = gpu
        .renderer(&a, &a, Fourcc::Argb8888)
        .expect("A is still there");
    let error = renderer
        .import_dmabuf(&client.dmabuf, None)
        .err()
        .expect("B's buffer is gone with B");
    assert!(format!("{error:?}").contains("DeviceMissing"), "{error:?}");

    // The next commit, as shm, renders exactly on A.
    let bytes = TestBuffer::shm_bytes(w, h, 2);
    let texture = renderer
        .import_memory(
            &bytes,
            Fourcc::Argb8888,
            Size::from((w as i32, h as i32)),
            false,
        )
        .expect("shm imports");
    draw(&mut renderer, &mut out, &texture).expect("draws");
    drop(renderer);
    assert_eq!(out.mismatches(2).expect("read back"), 0, "after the unplug");
}

#[test]
fn the_stack_replans_without_an_unplugged_gpu() {
    let topology = pair(CrossImport::Direct);
    let (dir, mut nodes) = tree(&topology);
    let by_path = topology.gpus[1].by_path().expect("id");
    let choice = GpuChoice::ByPath(by_path.clone());
    let stack = GpuStack::start(&mut nodes, &choice).expect("a stack");
    assert_eq!(stack.plan.render, DrmNodeId::new(226, 129));

    // B leaves the tree; the override now names a GPU that is not there.
    std::fs::remove_dir_all(dir.path().join("sys/dev/char/226:129")).expect("unplug");
    let gone = DrmNodeId::new(226, 129);
    let refused = stack.unplugged(&gone, &mut nodes, &choice);
    assert!(matches!(
        refused,
        Err(GpuError::Plan(PlanRefusal::UnknownGpu(id))) if id == by_path
    ));

    let stack = GpuStack::start(&mut nodes, &GpuChoice::Auto).expect("auto still plans");
    assert_eq!(stack.plan.render, DrmNodeId::new(226, 128));
    assert_eq!(stack.plan.scanout.len(), 1, "HDMI-A-1 went with B");
    let again = stack
        .unplugged(&gone, &mut nodes, &GpuChoice::Auto)
        .expect("replanned");
    assert_eq!(again.plan.render, DrmNodeId::new(226, 128));
}
