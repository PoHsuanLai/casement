#!/usr/bin/env bash
# Builds, OUTSIDE the jail (it needs the network once, A7), the two artefacts dev/wlcs/run.sh
# runs inside it:
#   dev/out/tools/wlcs/wlcs            wlcs at the SHA smithay's compile_wlcs.sh pins (SP5: no
#                                      sudo, about 26 s with make -j8). wlcs is GPL-3.0 (A6): a
#                                      separate test program, never shipped, never linked into
#                                      anything we ship.
#   dev/out/tools/libwlcs_casement.so  our MIT shim (wlcs-casement, an excluded crate), the
#                                      cdylib wlcs dlopens.
# Plain cargo (the shim serialises builds); CARGO_TARGET_DIR is honoured, else the shim builds
# into dev/out/tools/shim-target.
set -euo pipefail
cd "$(dirname "$0")/../.."
sha=2b385328366de5db19521b9b3eef81861430d984
dir=dev/out/tools/wlcs
mkdir -p dev/out/tools
if [ ! -d "$dir/.git" ]; then
  git clone https://github.com/MirServer/wlcs.git "$dir"
fi
if [ "$(git -C "$dir" rev-parse HEAD)" != "$sha" ]; then
  git -C "$dir" fetch --quiet origin
  git -C "$dir" reset --hard "$sha"
fi
if [ ! -x "$dir/wlcs" ]; then
  (
    cd "$dir"
    cmake -DWLCS_BUILD_ASAN=False -DWLCS_BUILD_TSAN=False -DWLCS_BUILD_UBSAN=False .
    make -j8
  )
fi
[ -x "$dir/wlcs" ] || { echo "wlcs: build finished but $dir/wlcs is missing" >&2; exit 1; }
echo "wlcs built at $sha: $dir/wlcs"

target="${CARGO_TARGET_DIR:-$PWD/dev/out/tools/shim-target}"
CARGO_TARGET_DIR="$target" cargo build --manifest-path wlcs-casement/Cargo.toml
cp "$target/debug/libwlcs_casement.so" dev/out/tools/libwlcs_casement.so
echo "shim built: dev/out/tools/libwlcs_casement.so"
