//! `fake-logind`: the logind subset on the jail's private bus (P1-F2). Not built at P0: the
//! headless session needs no logind (no seat, no devices), so the binary says so and exits 0
//! and `dev/lib/compositor.sh` carries on without it. P1-F2 fills `FakeLogind::serve`.

fn main() -> std::process::ExitCode {
    eprintln!("fake-logind: P1-F2: not built (the headless backend needs no logind)");
    std::process::ExitCode::SUCCESS
}
