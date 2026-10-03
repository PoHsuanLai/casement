#!/usr/bin/env bash
# Runs the layout and motion benches (A30) on the release profile, OUTSIDE the jail (it builds), and
# judges them (20-FREEZE-PLAN 6.4): `arrange` p95 at most 200 us and `sample` p95 at most 50 us at 50
# windows, and no row's p95 more than twice (and 5 us above) the last green run's.
#
#   dev/perf/run.sh [--iters N] [--record] [--no-build]
#
# Rows go to dev/out/perf/<stamp>.tsv (`label p50 p95 p99 max`, microseconds). --record stores a
# green run as dev/out/perf/last-green.tsv, the trend's baseline. Exit: 0 green | 1 over a budget |
# 4 a p95 doubled against last green | 2 inside the jail. Timings are CPU-load sensitive: run on a
# quiet machine, and treat a lone failure as a reason to run again.
#
# Owner: P0-T-dev.
set -euo pipefail
root=$(cd "$(dirname "$0")/../.." && pwd)
cd "$root"
# shellcheck source=dev/lib/jail.sh
source "$root/dev/lib/jail.sh"

iters=20000 record=no build=yes
while [ $# -gt 0 ]; do
  case "$1" in
    --iters) iters=${2:?--iters needs a value}; shift 2 ;;
    --record) record=yes; shift ;;
    --no-build) build=no; shift ;;
    -h|--help) sed -n '2,14p' "$0"; exit 0 ;;
    *) echo "perf/run.sh: unknown argument $1" >&2; exit 64 ;;
  esac
done
if [ -z "$(jail_violations)" ]; then
  echo "perf/run.sh: this builds with cargo: run it outside the jail" >&2
  exit 2
fi

target="${CARGO_TARGET_DIR:-$root/target}"
manifest="$root/dev/perf/Cargo.toml"
if [ "$build" = yes ]; then
  CARGO_TARGET_DIR="$target" cargo build --release --manifest-path "$manifest" --bins
fi
out="$root/dev/out/perf"
mkdir -p "$out"
stamp=$(date +%Y%m%dT%H%M%S)
rows="$out/$stamp.tsv"
: >"$rows"
for bench in layout motion; do
  "$target/release/$bench" "$iters" --tsv >>"$rows"
done
column -t -s "$(printf '\t')" "$rows" 2>/dev/null || cat "$rows"

# Ceilings: rows by label prefix, p95 in column 3.
over=$(awk -F'\t' '
  /^arrange:/ && $3 > 200 { printf "  %s: p95 %s us > 200 us\n", $1, $3 }
  /^sample/   && $3 > 50  { printf "  %s: p95 %s us > 50 us\n", $1, $3 }' "$rows")
if [ -n "$over" ]; then
  echo "perf: over budget:"; echo "$over"
  exit 1
fi

baseline="$out/last-green.tsv"
if [ -f "$baseline" ]; then
  slow=$(awk -F'\t' 'NR == FNR { old[$1] = $3; next }
    ($1 in old) && $3 > 2 * old[$1] && $3 - old[$1] > 5 { printf "  %s: p95 %s us, was %s us\n", $1, $3, old[$1] }' "$baseline" "$rows")
  if [ -n "$slow" ]; then
    echo "perf: p95 more than doubled against last green:"; echo "$slow"
    exit 4
  fi
fi
if [ "$record" = yes ]; then cp "$rows" "$baseline"; echo "perf: recorded $baseline"; fi
echo "perf: green ($rows)"
