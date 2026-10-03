#!/usr/bin/env bash
# Fetches and builds every artefact the jailed scripts need, OUTSIDE the jail (A7), into
# dev/out/tools, without root: foot (dnf download + rpm2cpio unpack), xwayland-satellite
# (cargo install --git https://github.com/Supreeeme/xwayland-satellite --rev <pin> --root dev/out/tools;
# it is not on crates.io, SP6), wlcs (dev/wlcs/build.sh), cargo-fuzz with --fuzz.
#
# Owner: P0-T-dev. Frozen at P0-Z2 as a stub: the name and the command line are fixed, the body is not
# written yet. Exit 2 (could not run) until it is.
set -euo pipefail
echo "bootstrap.sh: not written yet (P0-T-dev)" >&2
exit 2
