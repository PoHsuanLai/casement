#!/usr/bin/env bash
# The gate's first step (20-FREEZE-PLAN §4.3, §6.1): is this machine able to run the gate?
# Every failed check exits 3 (environment incomplete) with the fix printed. Never builds.
set -euo pipefail
cd "$(dirname "$0")/.."
tier="${GATE_TIER:-fast}"
need() { echo "preflight: $1" >&2; echo "preflight: fix: $2" >&2; exit 3; }

jail="${CASEMENT_JAIL:-../desktop/harness/jail.sh}"
[ -x "$jail" ] || need "the jail script $jail is missing or not executable" \
  "check out ~/desktop beside casement (git clone https://github.com/PoHsuanLai/desktop ../desktop) or set CASEMENT_JAIL"
command -v bwrap >/dev/null || need "bubblewrap (bwrap) is not on PATH" "sudo dnf install bubblewrap"
[ -e /dev/udmabuf ] || need "/dev/udmabuf is missing" "log in locally (Q-T1), or: sudo modprobe udmabuf"
[ -r /dev/udmabuf ] || need "/dev/udmabuf is not readable" "log in locally (Q-T1): the uaccess ACL comes with a local seat session"
if ! command -v cargo-nextest >/dev/null && ! cargo nextest --version >/dev/null 2>&1; then
  need "cargo-nextest is not installed" "cargo install --locked cargo-nextest"
fi
case "$tier" in
  merge|phase)
    cargo deny --version >/dev/null 2>&1 || need "cargo-deny is not installed" "cargo install --locked cargo-deny" ;;
esac

command -v pkg-config >/dev/null || need "pkg-config is not on PATH" "sudo dnf install pkgconf-pkg-config"
missing=()
for lib in libseat libinput libudev gbm libdrm xkbcommon libpipewire-0.3 wayland-server pixman-1 egl glesv2; do
  pkg-config --exists "$lib" || missing+=("$lib")
done
if [ ${#missing[@]} -gt 0 ]; then
  need "development libraries smithay links are missing: ${missing[*]}" \
    "sudo dnf install libseat-devel libinput-devel systemd-devel mesa-libgbm-devel libdrm-devel libxkbcommon-devel pipewire-devel wayland-devel pixman-devel libglvnd-devel"
fi

case "$tier" in
  merge|phase)
    tools="${CASEMENT_TOOLS:-dev/out/tools}"
    for tool in foot xwayland-satellite wlcs; do
      [ -e "$tools/$tool" ] || [ -e "$tools/bin/$tool" ] \
        || need "bootstrap artefact $tools/$tool is missing" "run dev/bootstrap.sh (outside the jail)"
    done ;;
esac
echo "preflight ok"
