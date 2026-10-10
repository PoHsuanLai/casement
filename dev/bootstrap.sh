#!/usr/bin/env bash
# Fetches and builds every artefact the jailed scripts need, OUTSIDE the jail (A7: nothing inside
# the jail calls cargo or the network), into dev/out/tools (no root):
#   bin/casement bin/casementctl bin/casement-probe bin/fake-logind
#                the compositor (headless backend, `debug-inject`), its control tool, the test
#                client's probe and the fake logind, built with plain cargo (CARGO_TARGET_DIR
#                is honoured, else ./target; --release builds the release profile)
#   bin/foot     a terminal for the toolkit scripts: the one on PATH, else the Fedora packages
#                foot and fcft through `dnf download` (no root) and unpacked
#   bin/xwayland-satellite
#                cargo install --git https://github.com/Supreeeme/xwayland-satellite at the pinned
#                rev (it is not on crates.io, SP6). Run with `-glamor none` (A10: `off` panics it)
#   wlcs/        wlcs and its shim, through dev/wlcs/build.sh (the P0-T-wlcs lane's script), into
                $CASEMENT_TOOLS too: wlcs/wlcs and libwlcs_casement.so
#   bin/cargo-fuzz   only with --fuzz
#
#   dev/bootstrap.sh [--build-only | --tools-only | --foot-only] [--release] [--fuzz] [--force]
#
# --build-only builds just the four casement binaries (what dev/accept.sh --build runs);
# --tools-only the rest; --foot-only just foot (no cargo, no network beyond dnf). Anything already present is kept unless --force. $CASEMENT_TOOLS moves
# the tools directory. Refuses inside the jail (exit 2). Exit 3: a host tool is missing.
#
# Owner: P0-T-dev.
set -euo pipefail
root=$(cd "$(dirname "$0")/.." && pwd)
cd "$root"
# shellcheck source=dev/lib/jail.sh
source "$root/dev/lib/jail.sh"

# The xwayland-satellite revision (v0.8.3, SP6): a full SHA, so a moved tag cannot change the build.
SATELLITE_REPO=https://github.com/Supreeeme/xwayland-satellite
SATELLITE_REV=b5690b56d749526a05db9b9268d58bf8f700957f

want_build=yes want_tools=yes want_satellite=yes profile=debug fuzz=no force=no
while [ $# -gt 0 ]; do
  case "$1" in
    --build-only) want_tools=no; shift ;;
    --tools-only) want_build=no; shift ;;
    --foot-only) want_build=no; want_satellite=no; shift ;;
    --release) profile=release; shift ;;
    --fuzz) fuzz=yes; shift ;;
    --force) force=yes; shift ;;
    -h|--help) sed -n '2,22p' "$0"; exit 0 ;;
    *) echo "bootstrap.sh: unknown argument $1" >&2; exit 64 ;;
  esac
done

if [ -z "$(jail_violations)" ]; then
  echo "bootstrap.sh: this builds and fetches (cargo, dnf, git, the network): run it outside the jail" >&2
  exit 2
fi

tools="${CASEMENT_TOOLS:-$root/dev/out/tools}"
mkdir -p "$tools/bin"
target="${CARGO_TARGET_DIR:-$root/target}"
have() { [ "$force" = no ] && [ -e "$1" ]; }
need() { command -v "$1" >/dev/null || { echo "bootstrap.sh: $1 is not installed: $2" >&2; exit 3; }; }

build_casement() {
  local flag=()
  [ "$profile" = debug ] || flag+=(--release)
  # One invocation, so the features unify: the test kit builds casement with `testing` and the
  # binary gets `debug-inject` besides. Both refuse to run outside the jail.
  cargo build "${flag[@]}" --bins -p casement -p casementctl -p casement-testclient -p casement-testkit \
    --features casement/debug-inject
  local bin
  for bin in casement casementctl casement-probe fake-logind writeback-compare; do
    # A copy, not a link: a later `cargo clean` must not leave the jail a dangling tool.
    cp -f "$target/$profile/$bin" "$tools/bin/$bin.new"
    mv -f "$tools/bin/$bin.new" "$tools/bin/$bin"
  done
  echo "bootstrap: casement, casementctl, casement-probe, fake-logind, writeback-compare -> $tools/bin ($profile)"
}

fetch_foot() {
  if have "$tools/bin/foot"; then echo "bootstrap: foot present"; return 0; fi
  if command -v foot >/dev/null && [ "$force" = no ]; then
    ln -sf "$(command -v foot)" "$tools/bin/foot"
    echo "bootstrap: foot is $(command -v foot)"
    return 0
  fi
  need dnf "this needs Fedora's dnf (the packages foot and fcft are fetched, not installed)"
  need rpm2cpio "sudo dnf install rpm"
  need cpio "sudo dnf install cpio"
  local dir="$tools/foot-root" rpms="$tools/foot-rpms"
  rm -rf "$dir" "$rpms"
  mkdir -p "$dir" "$rpms"
  (cd "$rpms" && timeout 300 dnf download --arch x86_64 foot fcft)
  for rpm in "$rpms"/*.rpm; do (cd "$dir" && rpm2cpio "$rpm" | cpio -idm 2>/dev/null); done
  [ -x "$dir/usr/bin/foot" ] || { echo "bootstrap.sh: the foot package held no usr/bin/foot" >&2; exit 1; }
  # A wrapper, so the unpacked fcft is found without touching the system.
  cat >"$tools/bin/foot" <<WRAP
#!/usr/bin/env bash
export LD_LIBRARY_PATH="$dir/usr/lib64\${LD_LIBRARY_PATH:+:\$LD_LIBRARY_PATH}"
exec "$dir/usr/bin/foot" "\$@"
WRAP
  chmod +x "$tools/bin/foot"
  echo "bootstrap: foot -> $tools/bin/foot"
}

install_satellite() {
  if have "$tools/bin/xwayland-satellite"; then echo "bootstrap: xwayland-satellite present"; return 0; fi
  local again=()
  [ "$force" = no ] || again+=(--force)
  cargo install --locked --git "$SATELLITE_REPO" --rev "$SATELLITE_REV" --root "$tools" "${again[@]}" xwayland-satellite
  echo "bootstrap: xwayland-satellite $SATELLITE_REV -> $tools/bin"
}

build_wlcs() {
  if [ ! -x "$root/dev/wlcs/build.sh" ]; then echo "bootstrap: dev/wlcs/build.sh is missing: wlcs skipped" >&2; return 0; fi
  # The wlcs lane's own script; a frozen stub exits 2, which is a warning here, not a failure.
  local rc=0
  CASEMENT_TOOLS="$tools" "$root/dev/wlcs/build.sh" || rc=$?
  [ "$rc" = 0 ] || echo "bootstrap: dev/wlcs/build.sh exited $rc: wlcs is not built" >&2
}

install_fuzz() {
  if have "$tools/bin/cargo-fuzz"; then echo "bootstrap: cargo-fuzz present"; return 0; fi
  cargo install --locked cargo-fuzz --root "$tools"
}

if [ "$want_build" = yes ]; then build_casement; fi
if [ "$want_tools" = yes ]; then
  fetch_foot
  if [ "$want_satellite" = yes ]; then
    install_satellite
    build_wlcs
    if [ "$fuzz" = yes ]; then install_fuzz; fi
  fi
fi
echo "bootstrap: done ($tools)"
