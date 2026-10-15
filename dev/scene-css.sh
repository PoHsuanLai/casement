#!/usr/bin/env bash
# Builds and runs the excluded casement-scene-css parity oracle (CssScene against RustScene,
# S-L6): the 500-spec test in casement-scene-css/tests/parity.rs. OUTSIDE the jail (it builds
# Blitz), into its own target directory so the oracle's heavy build never touches the
# workspace's.
#
#   dev/scene-css.sh [--no-run] [test-filter]
#
# The target directory is $CASEMENT_SCENE_CSS_TARGET, else dev/out/scene-css-target. --no-run
# builds the tests only. A filter is passed to the test binary. Exit: 0 green | 1 a parity
# failure (the output names the spec index) | 2 inside the jail (it builds). Phase-exit step of
# P3 (S-L6); never part of the per-change gate.
#
# Owner: P3-CSS.
set -euo pipefail
root=$(cd "$(dirname "$0")/.." && pwd)
cd "$root"
# shellcheck source=dev/lib/jail.sh
source "$root/dev/lib/jail.sh"

run=yes filter=()
while [ $# -gt 0 ]; do
  case "$1" in
    --no-run) run=no; shift ;;
    -h|--help) sed -n '2,12p' "$0"; exit 0 ;;
    -*) echo "scene-css.sh: unknown argument $1" >&2; exit 64 ;;
    *) filter+=("$1"); shift ;;
  esac
done
if [ -z "$(jail_violations)" ]; then
  echo "scene-css.sh: this builds with cargo: run it outside the jail" >&2
  exit 2
fi

export CARGO_TARGET_DIR="${CASEMENT_SCENE_CSS_TARGET:-$root/dev/out/scene-css-target}"
mkdir -p "$CARGO_TARGET_DIR"
manifest="$root/casement-scene-css/Cargo.toml"
if [ "$run" = no ]; then
  cargo test --manifest-path "$manifest" --locked --no-run
else
  cargo test --manifest-path "$manifest" --locked --test parity -- "${filter[@]}"
fi
