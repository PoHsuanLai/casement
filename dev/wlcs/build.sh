#!/usr/bin/env bash
# Builds wlcs at the SHA smithay's compile_wlcs.sh pins into dev/out/tools/wlcs (SP5: no sudo,
# about 26 s with make -j8). Runs OUTSIDE the jail: it needs the network once (A7). wlcs is
# GPL-3.0 (A6): a separate test program, never shipped, never linked into anything we ship; our
# MIT shim (wlcs-casement) is what it dlopens.
set -euo pipefail
cd "$(dirname "$0")/../.."
sha=2b385328366de5db19521b9b3eef81861430d984
dir=dev/out/tools/wlcs
mkdir -p dev/out/tools
if [ ! -d "$dir/.git" ]; then
  git clone https://github.com/MirServer/wlcs.git "$dir"
fi
git -C "$dir" fetch --quiet origin
git -C "$dir" reset --hard "$sha"
(
  cd "$dir"
  cmake -DWLCS_BUILD_ASAN=False -DWLCS_BUILD_TSAN=False -DWLCS_BUILD_UBSAN=False .
  make -j8
)
[ -x "$dir/wlcs" ] || { echo "wlcs: build finished but $dir/wlcs is missing" >&2; exit 1; }
echo "wlcs built at $sha: $dir/wlcs"
