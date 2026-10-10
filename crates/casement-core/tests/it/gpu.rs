//! GPU planning (P1-D4): which GPU renders and scans out, and the dma-buf feedback. The tables
//! include the shape of the owner's machine (an integrated AMD GPU driving the panel, a discrete
//! NVIDIA one beside it), written as data: nothing in the planner knows it.

use casement_core::OutputName;
use casement_core::gpu::{
    BootVga, ByPathId, DrmNodeId, FormatEntry, FormatTable, Fourcc, GpuChoice, GpuFacts, Modifier,
    PlanRefusal, RenderPlan, TrancheUse, VendorFact, feedback_for, plan_for,
};
use std::collections::BTreeMap;

fn gpu(minor: u32, vendor: VendorFact, boot: BootVga, drives: &[&str], by_path: &str) -> GpuFacts {
    GpuFacts::new(
        DrmNodeId::new(226, minor),
        vendor,
        boot,
        drives
            .iter()
            .map(|name| OutputName::parse(name).expect("output name"))
            .collect(),
        ByPathId::parse(by_path).expect("by-path id"),
    )
}

const IGPU: &str = "0000:65:00.0";
const DGPU: &str = "0000:01:00.0";

fn node(minor: u32) -> DrmNodeId {
    DrmNodeId::new(226, minor)
}

fn name(text: &str) -> OutputName {
    OutputName::parse(text).expect("output name")
}

/// This machine's shape: the NVIDIA card enumerates first (render128), the AMD iGPU second.
fn laptop(dgpu_drives: &[&str]) -> [GpuFacts; 2] {
    [
        gpu(
            128,
            VendorFact::Nvidia,
            BootVga::Secondary,
            dgpu_drives,
            DGPU,
        ),
        gpu(129, VendorFact::Amd, BootVga::Primary, &["eDP-1"], IGPU),
    ]
}

#[test]
fn the_gpu_driving_the_outputs_renders() {
    let plan = plan_for(&laptop(&[]), &GpuChoice::Auto).expect("a plan");
    assert_eq!(plan.render, node(129));
    assert_eq!(plan.scanout.get(&name("eDP-1")), Some(&node(129)));
}

#[test]
fn several_gpus_driving_outputs_fall_to_the_firmware_gpu() {
    let plan = plan_for(&laptop(&["HDMI-A-1"]), &GpuChoice::Auto).expect("a plan");
    assert_eq!(plan.render, node(129), "the boot GPU renders");
    assert_eq!(plan.scanout.get(&name("HDMI-A-1")), Some(&node(128)));
    assert_eq!(plan.scanout.get(&name("eDP-1")), Some(&node(129)));
}

#[test]
fn a_dgpu_only_machine_renders_on_the_dgpu_whatever_its_boot_flag() {
    let facts = [
        gpu(
            130,
            VendorFact::Intel,
            BootVga::Secondary,
            &[],
            "0000:00:02.0",
        ),
        gpu(128, VendorFact::Nvidia, BootVga::Secondary, &["DP-1"], DGPU),
    ];
    let plan = plan_for(&facts, &GpuChoice::Auto).expect("a plan");
    assert_eq!(plan.render, node(128), "the GPU that drives a monitor");
}

#[test]
fn with_no_monitor_attached_the_boot_gpu_then_the_lowest_node_render() {
    let facts = [
        gpu(128, VendorFact::Nvidia, BootVga::Secondary, &[], DGPU),
        gpu(129, VendorFact::Amd, BootVga::Primary, &[], IGPU),
    ];
    assert_eq!(
        plan_for(&facts, &GpuChoice::Auto).expect("a plan").render,
        node(129)
    );
    let unflagged = [
        gpu(129, VendorFact::Amd, BootVga::Secondary, &[], IGPU),
        gpu(128, VendorFact::Nvidia, BootVga::Secondary, &[], DGPU),
    ];
    let plan = plan_for(&unflagged, &GpuChoice::Auto).expect("a plan");
    assert_eq!(plan.render, node(128));
    assert!(plan.scanout.is_empty());
}

#[test]
fn the_setting_overrides_detection() {
    let choice = GpuChoice::ByPath(ByPathId::parse(DGPU).expect("by-path id"));
    let plan = plan_for(&laptop(&[]), &choice).expect("a plan");
    assert_eq!(plan.render, node(128));
    assert_eq!(
        plan.scanout.get(&name("eDP-1")),
        Some(&node(129)),
        "scanout stays on the GPU that drives the panel"
    );
}

#[test]
fn the_setting_naming_the_gpu_detection_picks_changes_nothing() {
    let choice = GpuChoice::ByPath(ByPathId::parse(IGPU).expect("by-path id"));
    assert_eq!(
        plan_for(&laptop(&[]), &choice),
        plan_for(&laptop(&[]), &GpuChoice::Auto)
    );
}

#[test]
fn no_gpu_and_an_unknown_override_are_refused() {
    assert_eq!(plan_for(&[], &GpuChoice::Auto), Err(PlanRefusal::NoGpu));
    let unknown = ByPathId::parse("0000:ff:00.0").expect("by-path id");
    assert_eq!(
        plan_for(&[], &GpuChoice::ByPath(unknown.clone())),
        Err(PlanRefusal::UnknownGpu(unknown.clone()))
    );
    assert_eq!(
        plan_for(&laptop(&[]), &GpuChoice::ByPath(unknown.clone())),
        Err(PlanRefusal::UnknownGpu(unknown)),
        "a GPU that is not there is refused even when others are"
    );
}

#[test]
fn by_path_ids_are_lowercase_pci_addresses() {
    for good in ["0000:01:00.0", "0000:65:00.0", "0001:0a:1f.7"] {
        assert_eq!(ByPathId::parse(good).expect(good).as_str(), good);
    }
    for bad in [
        "",
        "auto",
        "0000:01:00",
        "0000:01:00.8",
        "0000:0A:00.0",
        "000:01:00.0",
        "pci-0000:01:00.0-render",
        "0000:01:00.0 ",
    ] {
        assert!(ByPathId::parse(bad).is_err(), "{bad:?} is not an id");
    }
}

fn entry(format: u32, modifier: u64) -> FormatEntry {
    FormatEntry {
        format: Fourcc(format),
        modifier: Modifier(modifier),
    }
}

const ARGB: u32 = 0x3432_5241;
const XRGB: u32 = 0x3432_5258;

#[test]
fn feedback_lists_render_formats_before_import_formats() {
    let (render, other) = (node(129), node(128));
    let scanout = BTreeMap::from([(name("eDP-1"), render), (name("HDMI-A-1"), other)]);
    let plan = RenderPlan::new(render, scanout);
    let table = FormatTable::new(BTreeMap::from([
        (render, vec![entry(ARGB, 0), entry(XRGB, 0)]),
        // ARGB linear is already offered by the render GPU; the tiled one is new.
        (
            other,
            vec![entry(ARGB, 0), entry(ARGB, 0x0100_0000_0000_0001)],
        ),
    ]));
    let feedback = feedback_for(&plan, &table);
    assert_eq!(feedback.main_device, render);
    let shape: Vec<_> = feedback
        .tranches
        .iter()
        .map(|tranche| (tranche.target, tranche.purpose, tranche.formats.clone()))
        .collect();
    assert_eq!(
        shape,
        vec![
            (
                render,
                TrancheUse::Render,
                vec![entry(ARGB, 0), entry(XRGB, 0)]
            ),
            (
                other,
                TrancheUse::Import,
                vec![entry(ARGB, 0x0100_0000_0000_0001)]
            ),
        ]
    );
}

#[test]
fn feedback_leaves_out_gpus_that_scan_nothing_out_and_empty_tranches() {
    let (render, idle, same) = (node(129), node(128), node(130));
    let plan = RenderPlan::new(
        render,
        BTreeMap::from([(name("eDP-1"), render), (name("DP-1"), same)]),
    );
    let table = FormatTable::new(BTreeMap::from([
        (render, vec![entry(ARGB, 0)]),
        (idle, vec![entry(XRGB, 0)]),
        (same, vec![entry(ARGB, 0)]),
    ]));
    let feedback = feedback_for(&plan, &table);
    assert_eq!(
        feedback.tranches.len(),
        1,
        "the idle GPU is not offered; the other scanout GPU adds nothing new"
    );
    assert_eq!(feedback.tranches[0].purpose, TrancheUse::Render);
}

#[test]
fn feedback_for_a_render_gpu_without_formats_is_empty_but_names_the_main_device() {
    let render = node(128);
    let plan = RenderPlan::new(render, BTreeMap::new());
    let feedback = feedback_for(&plan, &FormatTable::default());
    assert_eq!(feedback.main_device, render);
    assert!(feedback.tranches.is_empty());
}

#[test]
fn feedback_offers_the_formats_a_plane_takes_first_as_a_scanout_tranche() {
    let render = node(129);
    let plan = RenderPlan::new(render, BTreeMap::from([(name("eDP-1"), render)]));
    let table = FormatTable::new(BTreeMap::from([(
        render,
        vec![entry(ARGB, 0), entry(XRGB, 0)],
    )]))
    .with_scanout(BTreeMap::from([(
        render,
        // The plane takes XRGB linear, and a tiled layout the GPU does not draw into.
        vec![entry(XRGB, 0), entry(XRGB, 0x0100_0000_0000_0001)],
    )]));
    let feedback = feedback_for(&plan, &table);
    let shape: Vec<_> = feedback
        .tranches
        .iter()
        .map(|tranche| (tranche.target, tranche.purpose, tranche.formats.clone()))
        .collect();
    assert_eq!(
        shape,
        vec![
            (render, TrancheUse::Scanout, vec![entry(XRGB, 0)]),
            (render, TrancheUse::Render, vec![entry(ARGB, 0)]),
        ],
        "a pair is offered once, at its first tranche; a plane format the GPU cannot draw is not offered"
    );
}

#[test]
fn feedback_has_no_scanout_tranche_for_planes_of_another_gpu() {
    let (render, other) = (node(129), node(128));
    let plan = RenderPlan::new(render, BTreeMap::from([(name("DP-1"), other)]));
    let table = FormatTable::new(BTreeMap::from([
        (render, vec![entry(ARGB, 0)]),
        (other, vec![entry(XRGB, 0)]),
    ]))
    .with_scanout(BTreeMap::from([(other, vec![entry(XRGB, 0)])]));
    let feedback = feedback_for(&plan, &table);
    let purposes: Vec<_> = feedback.tranches.iter().map(|t| t.purpose).collect();
    assert_eq!(purposes, vec![TrancheUse::Render, TrancheUse::Import]);
}
