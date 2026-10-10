//! `casement-x11probe`: an X11 client for the XWayland suite (P1-X1). It sets `WM_CLASS`, maps a
//! window, reads input and selections, reports override-redirect facts and the clipboard, as JSON
//! lines like `casement-probe`, so `dev/accept/xwayland-*.sh` and the suite assert what an X11
//! client sees through xwayland-satellite. P1-X1 writes it (with the `x11rb` dependency); until
//! then it says so and exits 2 ("could not run"), which no gate lists as a pass.

fn main() -> std::process::ExitCode {
    eprintln!("casement-x11probe: P1-X1: not built");
    std::process::ExitCode::from(2)
}
