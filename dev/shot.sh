#!/usr/bin/env bash
# Takes a screenshot of a running jailed casement, through `casementctl shot`
# (ext-image-copy-capture):
#
#   dev/shot.sh [--output NAME | --window ID | --region X,Y,W,H] [--masked] OUT.png
#
# A bare file name goes to the run's shots/ directory ($JAIL_SCRATCH/casement/<run>/shots/), where
# the review gates look; a path with a directory is used as given. Calls require_jail first.
# Exit 2 until the compositor serves the capture protocols (P1-C1).
#
# Owner: P0-T-dev.
set -euo pipefail
root=$(cd "$(dirname "$0")/.." && pwd)
# shellcheck source=dev/lib/shot.sh
source "$root/dev/lib/shot.sh"
require_jail
target=output out="" masked=""
while [ $# -gt 0 ]; do
  case "$1" in
    --output) target="output:${2:?--output needs a value}"; shift 2 ;;
    --window) target="window:${2:?--window needs a value}"; shift 2 ;;
    --region) target="region:${2:?--region needs a value}"; shift 2 ;;
    --masked) masked="masked:"; shift ;;
    -*) echo "shot.sh: unknown flag $1" >&2; exit 64 ;;
    *) out=$1; shift ;;
  esac
done
[ -n "$out" ] || { echo "usage: dev/shot.sh [--output NAME | --window ID | --region X,Y,W,H] [--masked] OUT.png" >&2; exit 64; }
case "$out" in */*) ;; *) out="$(casement_state_dir)/shots/$out" ;; esac
shot "$masked$target" "$out"
echo "shot: $out"
