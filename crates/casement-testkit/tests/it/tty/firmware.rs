//! A firmware framebuffer (simpledrm) beside a real GPU is not driven, and a virtual test display
//! (a configfs vkms device on the faux bus) never is (the first real login drew a phantom
//! `Virtual-1` on the leftover vkms device): sysfs fixtures for the detection, the backend over
//! fakes for the rule at start and at hotplug.

use casement_backend::gpu::SysRoot;
use casement_backend::tty::{
    DeviceEvent, DevicePath, NoGpus, SysDeviceKinds, TtyBackend, TtySeams,
};
use casement_core::gpu::{DisplayKind, GpuChoice};
use casement_testkit::fakes::{
    FakeKms, FakeKmsOpener, FakeSession, FakeUdev, SharedSession, SyntheticTopology,
};
use std::path::{Path, PathBuf};

fn card(n: u32) -> DevicePath {
    DevicePath(PathBuf::from(format!("/dev/dri/card{n}")))
}

/// `sys/class/drm/card<n>/device -> .../<device>` and `device/driver -> .../<driver>`: the
/// device directory is real and holds the driver link, as the kernel shows it.
fn fabricate(dir: &Path, n: u32, device: &str, driver: &str) {
    fabricate_on(dir, n, device, driver, "platform");
}

/// The same on bus `bus`: `device/subsystem -> .../bus/<bus>`.
fn fabricate_on(dir: &Path, n: u32, device: &str, driver: &str, bus: &str) {
    let real = dir.join("sys/devices").join(device);
    std::fs::create_dir_all(&real).expect("device dir");
    std::os::unix::fs::symlink(format!("../../bus/drivers/{driver}"), real.join("driver"))
        .expect("driver link");
    std::os::unix::fs::symlink(format!("../../bus/{bus}"), real.join("subsystem"))
        .expect("subsystem link");
    let class = dir.join(format!("sys/class/drm/card{n}"));
    std::fs::create_dir_all(&class).expect("class dir");
    std::os::unix::fs::symlink(&real, class.join("device")).expect("device link");
}

fn machine() -> (tempfile::TempDir, SysRoot) {
    let dir = tempfile::tempdir().expect("temp dir");
    fabricate(dir.path(), 0, "simple-framebuffer.0", "simple-framebuffer");
    fabricate_on(dir.path(), 1, "0000:10:00.0", "amdgpu", "pci");
    fabricate_on(dir.path(), 2, "0000:01:00.0", "nvidia", "pci");
    fabricate_on(dir.path(), 3, "faux/casement", "faux_driver", "faux");
    let root = SysRoot::under(dir.path());
    (dir, root)
}

#[test]
fn sysfs_names_the_framebuffer_and_the_gpus() {
    let (_dir, root) = machine();
    let kinds = SysDeviceKinds::new(root);
    assert_eq!(kinds.kind(&card(0)), DisplayKind::Firmware);
    assert_eq!(kinds.kind(&card(1)), DisplayKind::Gpu);
    assert_eq!(kinds.kind(&card(2)), DisplayKind::Gpu);
    assert_eq!(kinds.kind(&card(3)), DisplayKind::Virtual);
    assert_eq!(
        kinds.kind(&card(7)),
        DisplayKind::Gpu,
        "a device sysfs does not show is driven"
    );
}

fn backend(root: SysRoot, present: Vec<DevicePath>, udev: FakeUdev) -> TtyBackend {
    let session = SharedSession::new(FakeSession::default());
    let mut opener = FakeKmsOpener::default();
    for path in (0..4).map(card) {
        opener.devices.insert(
            path,
            FakeKms::with(SyntheticTopology::LaptopPanel.resources()),
        );
    }
    let seams = TtySeams::new(
        Box::new(session),
        Box::new(udev.with_present(present)),
        Box::new(opener),
        Box::new(NoGpus),
        GpuChoice::Auto,
    )
    .with_kinds(SysDeviceKinds::new(root));
    TtyBackend::start(seams).expect("start")
}

fn open(backend: &TtyBackend) -> Vec<DevicePath> {
    backend.devices.devices.keys().cloned().collect()
}

#[test]
fn the_framebuffer_is_not_opened_beside_real_gpus() {
    let (_dir, root) = machine();
    let backend = backend(
        root,
        vec![card(0), card(1), card(2), card(3)],
        FakeUdev::scripted(Vec::new()),
    );
    assert_eq!(open(&backend), [card(1), card(2)]);
}

#[test]
fn a_virtual_display_is_not_opened_even_alone() {
    let dir = tempfile::tempdir().expect("temp dir");
    fabricate_on(dir.path(), 0, "faux/casement", "faux_driver", "faux");
    let backend = backend(
        SysRoot::under(dir.path()),
        vec![card(0)],
        FakeUdev::scripted(Vec::new()),
    );
    assert!(open(&backend).is_empty());
}

#[test]
fn the_framebuffer_alone_is_opened() {
    let dir = tempfile::tempdir().expect("temp dir");
    fabricate(dir.path(), 0, "simple-framebuffer.0", "simple-framebuffer");
    let backend = backend(
        SysRoot::under(dir.path()),
        vec![card(0)],
        FakeUdev::scripted(Vec::new()),
    );
    assert_eq!(open(&backend), [card(0)]);
}

#[test]
fn a_framebuffer_or_virtual_display_added_beside_an_open_gpu_is_ignored() {
    let (_dir, root) = machine();
    let udev = FakeUdev::scripted(vec![
        DeviceEvent::Added(card(0)),
        DeviceEvent::Added(card(3)),
    ]);
    let mut backend = backend(root, vec![card(2)], udev);
    let _ = backend.dispatch();
    assert_eq!(open(&backend), [card(2)]);
}

#[test]
fn a_framebuffer_opened_before_a_gpu_arrived_is_closed_once_the_gpu_is_added() {
    let (_dir, root) = machine();
    let udev = FakeUdev::scripted(vec![DeviceEvent::Added(card(1))]);
    let mut backend = backend(root, vec![card(0)], udev);
    assert_eq!(
        open(&backend),
        [card(0)],
        "alone, the framebuffer is driven"
    );
    let _ = backend.dispatch();
    assert_eq!(open(&backend), [card(1)]);
}
