//! The probe CLI against its usage rules (the one test that needs no compositor).

/// casement-probe's exit code for the arguments `args`, run with nothing from this environment
/// (no compositor to connect to: a usage error must be found before it tries).
fn probe_exit(args: &[&str]) -> Option<i32> {
    std::process::Command::new(env!("CARGO_BIN_EXE_casement-probe"))
        .env_clear()
        .args(args)
        .output()
        .expect("casement-probe runs")
        .status
        .code()
}

#[test]
fn the_probe_rejects_unknown_flags_with_the_usage_code() {
    assert_eq!(probe_exit(&["toplevel", "--maximized"]), Some(64));
    assert_eq!(probe_exit(&["layer", "--maximised"]), Some(64));
    assert_eq!(probe_exit(&["toplevel", "--title"]), Some(64));
    assert_eq!(probe_exit(&["frobnicate"]), Some(64));
}
