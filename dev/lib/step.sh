#!/usr/bin/env bash
# step.sh RUN NAME CMD...: one gate step (20-FREEZE-PLAN §6.1, §6.2).
#
# Runs CMD with its combined output written to dev/out/RUN/NAME.log and shown on the terminal,
# times it, applies dev/gate/budgets.toml ([$GATE_TIER] NAME = seconds; over 2x the budget is
# exit 4), maps exit 2 (a SKIP) to a failure unless NAME is listed in
# dev/gate/expected-skips.toml, and records the step in dev/out/RUN/summary.json (rewritten
# whole after every step from steps.jsonl, so it is always valid JSON).
#
# Exit codes: 0 pass | 1 a test failed | 2 could not run safely | 3 environment incomplete |
# 4 budget overrun | 5 jail breach (printed and returned at once). The script exits with the
# step's final code, so `set -e` in gate.sh stops on a failure.
set -euo pipefail
[ $# -ge 3 ] || { echo "usage: step.sh RUN NAME CMD..." >&2; exit 64; }
run=$1 name=$2; shift 2
root=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
dir="$root/dev/out/$run"
mkdir -p "$dir"
log="$dir/$name.log"
tier="${GATE_TIER:-fast}"

# budget_of NAME: the step's budget in seconds for this tier, empty when none is set.
budget_of() {
  local file="$root/dev/gate/budgets.toml" section="" line key value
  [ -f "$file" ] || return 0
  while IFS= read -r line || [ -n "$line" ]; do
    line="${line%%#*}"
    line="${line#"${line%%[![:space:]]*}"}"; line="${line%"${line##*[![:space:]]}"}"
    [ -n "$line" ] || continue
    case "$line" in
      \[*\]) section="${line#[}"; section="${section%]}"; continue ;;
    esac
    [ "$section" = "$tier" ] || continue
    key="${line%%=*}"; value="${line#*=}"
    key="${key//[[:space:]]/}"; key="${key//\"/}"; value="${value//[[:space:]]/}"
    if [ "$key" = "$1" ]; then echo "$value"; return 0; fi
  done <"$file"
}

# skip_expected NAME: 0 when expected-skips.toml lists `step = "NAME"`.
skip_expected() {
  local file="$root/dev/gate/expected-skips.toml"
  [ -f "$file" ] || return 1
  grep -Eq "^[[:space:]]*step[[:space:]]*=[[:space:]]*\"$1\"" "$file"
}

start=$(date +%s)
# The pipe only shows the output: the step's code is the command's own (PIPESTATUS[0]).
set +e
"$@" 2>&1 | tee "$log"
codes=("${PIPESTATUS[@]}")
set -e
rc=${codes[0]}
if [ "${codes[1]}" != 0 ]; then echo "step $name: could not write $log" >&2; [ "$rc" != 0 ] || rc=3; fi
end=$(date +%s)
elapsed=$(( end - start ))

verdict=""
budget=$(budget_of "$name")
if [ "$rc" = 0 ] && [ -n "$budget" ] && [ "$elapsed" -gt $(( 2 * budget )) ]; then
  echo "step $name: ${elapsed}s is over twice its budget of ${budget}s" >&2
  rc=4
fi
if [ "$rc" = 2 ]; then
  if skip_expected "$name"; then verdict=SKIP; rc=0; else
    echo "step $name: SKIP is not listed in dev/gate/expected-skips.toml" >&2
    verdict=FAIL rc=2
  fi
fi
case "$rc" in
  0) [ -n "$verdict" ] || verdict=PASS ;;
  *) verdict=FAIL ;;
esac

printf '{"step":"%s","tier":"%s","verdict":"%s","exit":%d,"seconds":%d,"budget":%s,"log":"%s"}\n' \
  "$name" "$tier" "$verdict" "$rc" "$elapsed" "${budget:-null}" "$log" >>"$dir/steps.jsonl"
{
  printf '{"run":"%s","tier":"%s","steps":[\n' "$run" "$tier"
  first=1
  while IFS= read -r rec; do
    if [ "$first" = 1 ]; then first=0; else printf ',\n'; fi
    printf '  %s' "$rec"
  done <"$dir/steps.jsonl"
  printf '\n]}\n'
} >"$dir/summary.json.tmp"
mv "$dir/summary.json.tmp" "$dir/summary.json"

if [ "$verdict" = FAIL ]; then
  echo "FAIL $name (exit $rc, ${elapsed}s) log: $log"
else
  echo "$verdict $name (${elapsed}s)"
fi
if [ "$rc" = 5 ]; then echo "JAIL BREACH in $name: stopping at once" >&2; exit 5; fi
exit "$rc"
