//! GPU planning (P1-D4): which GPU renders and scans out, and the dma-buf feedback. Ignored
//! until the lane fills `plan_for` and `feedback_for`; the tables are the lane's to extend
//! with this machine's shape (an integrated GPU driving the panel, a discrete one the rest).

use casement_core::OutputName;
use casement_core::gpu::{
    BootVga, ByPathId, DrmNodeId, FormatEntry, FormatTable, Fourcc, GpuChoice, GpuFacts, Modifier,
    PlanRefusal, TrancheUse, VendorFact, feedback_for, plan_for,
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

#[test]
#[ignore = "P1-D4"]
fn the_gpu_driving_the_outputs_renders() {
    let facts = [
        gpu(
            128,
            VendorFact::Nvidia,
            BootVga::Secondary,
            &[],
            "0000:01:00.0",
        ),
        gpu(
            129,
            VendorFact::Amd,
            BootVga::Primary,
            &["eDP-1"],
            "0000:65:00.0",
        ),
    ];
    let plan = plan_for(&facts, &GpuChoice::Auto).expect("a plan");
    assert_eq!(plan.render, DrmNodeId::new(226, 129));
}

#[test]
#[ignore = "P1-D4"]
fn the_setting_overrides_detection() {
    let facts = [
        gpu(
            128,
            VendorFact::Nvidia,
            BootVga::Secondary,
            &[],
            "0000:01:00.0",
        ),
        gpu(
            129,
            VendorFact::Amd,
            BootVga::Primary,
            &["eDP-1"],
            "0000:65:00.0",
        ),
    ];
    let choice = GpuChoice::ByPath(ByPathId::parse("0000:01:00.0").expect("by-path id"));
    let plan = plan_for(&facts, &choice).expect("a plan");
    assert_eq!(plan.render, DrmNodeId::new(226, 128));
    assert_eq!(
        plan.scanout.get(&OutputName::parse("eDP-1").expect("name")),
        Some(&DrmNodeId::new(226, 129)),
        "scanout stays on the GPU that drives the panel"
    );
}

#[test]
#[ignore = "P1-D4"]
fn no_gpu_and_an_unknown_override_are_refused() {
    assert_eq!(plan_for(&[], &GpuChoice::Auto), Err(PlanRefusal::NoGpu));
    let unknown = ByPathId::parse("0000:ff:00.0").expect("by-path id");
    assert_eq!(
        plan_for(&[], &GpuChoice::ByPath(unknown.clone())),
        Err(PlanRefusal::UnknownGpu(unknown))
    );
}

#[test]
#[ignore = "P1-D4"]
fn feedback_lists_render_formats_before_import_formats() {
    let render = DrmNodeId::new(226, 128);
    let entry = FormatEntry {
        format: Fourcc(0x3432_5241),
        modifier: Modifier(0),
    };
    let plan = casement_core::gpu::RenderPlan::new(render, BTreeMap::new());
    let table = FormatTable::new(BTreeMap::from([(render, vec![entry])]));
    let feedback = feedback_for(&plan, &table);
    assert_eq!(feedback.main_device, render);
    assert_eq!(
        feedback.tranches.first().map(|tranche| tranche.purpose),
        Some(TrancheUse::Render)
    );
}
