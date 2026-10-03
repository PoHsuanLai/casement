#!/usr/bin/env bash
# The acceptance runner: dev/accept.sh [--jobs N] [--build] [--out DIR] [script...]. --build builds
# every artefact outside the jail first (A7: nothing inside the jail calls cargo or the network);
# each dev/accept/*.sh then runs in its own jail (parallel, N at a time, state under $JAIL_SCRATCH,
# A8) and the runner prints one PASS|SKIP|FAIL line per script with the plan's exit codes.
#
#   dev/accept.sh [--jobs N] [--build] [--out DIR] [--timeout SECONDS] [NAME...]
#
# NAME is a script's name (`globals`, `globals.sh` or its path); with none, every dev/accept/*.sh.
# Each script's jail gets its own scratch dir, DIR/<name>/ (default dev/out/<run>/accept/), so its
# runtime dir, bus, outputs and screenshots never meet another script's; its combined output is
# DIR/<name>.log. The checkout is bound read-only: a script cannot dirty it. Inside an outer jail
# (the gate's) each script still gets its own nested jail.
#
# Exit: 0 every script passed | 1 one failed | 2 none failed but one could not run (a SKIP) |
# 3 environment incomplete (an artefact is missing: run dev/bootstrap.sh) | 4 a script overran its
# bound | 5 a script reported a jail breach (the rest are stopped at once). A script's own exit code
# is its verdict, with the same meanings (20-FREEZE-PLAN 6.1).
#
# Owner: P0-T-dev.
set -euo pipefail
root=$(cd "$(dirname "$0")/.." && pwd)
# shellcheck source=dev/lib/jail.sh
source "$root/dev/lib/jail.sh"

parallel=4 build=no out="" bound=${ACCEPT_SCRIPT_TIMEOUT:-240} names=()
while [ $# -gt 0 ]; do
  case "$1" in
    --jobs) parallel=${2:?--jobs needs a value}; shift 2 ;;
    --build) build=yes; shift ;;
    --out) out=${2:?--out needs a value}; shift 2 ;;
    --timeout) bound=${2:?--timeout needs a value}; shift 2 ;;
    -h|--help) sed -n '2,20p' "$0"; exit 0 ;;
    -*) echo "accept.sh: unknown flag $1" >&2; exit 64 ;;
    *) names+=("$1"); shift ;;
  esac
done
case "$parallel" in ''|*[!0-9]*|0) echo "accept.sh: --jobs wants a positive number" >&2; exit 64 ;; esac

inside_jail=no
[ -z "$(jail_violations)" ] && inside_jail=yes

if [ "$build" = yes ]; then
  if [ "$inside_jail" = yes ]; then
    echo "accept.sh: --build runs cargo and the network, which never happen inside the jail: run it outside" >&2
    exit 3
  fi
  "$root/dev/bootstrap.sh" --build-only
fi

tools="${CASEMENT_TOOLS:-$root/dev/out/tools}"
for tool in casement casementctl fake-logind; do
  if [ ! -x "$tools/bin/$tool" ]; then
    echo "accept.sh: artefact $tools/bin/$tool is missing" >&2
    echo "accept.sh: fix: run dev/bootstrap.sh (or dev/accept.sh --build) outside the jail" >&2
    exit 3
  fi
done

jail="${CASEMENT_JAIL:-$_casement_desktop/harness/jail.sh}"
[ -x "$jail" ] || { echo "accept.sh: the jail script $jail is missing" >&2; exit 3; }

# The scripts to run, as names.
if [ ${#names[@]} -eq 0 ]; then
  for path in "$root"/dev/accept/*.sh; do names+=("$(basename "$path" .sh)"); done
else
  for i in "${!names[@]}"; do
    names[i]=$(basename "${names[i]}" .sh)
    [ -f "$root/dev/accept/${names[i]}.sh" ] || { echo "accept.sh: no script dev/accept/${names[i]}.sh" >&2; exit 64; }
  done
fi
[ ${#names[@]} -gt 0 ] || { echo "accept.sh: no scripts in dev/accept" >&2; exit 64; }
[ "$parallel" -le ${#names[@]} ] || parallel=${#names[@]}

if [ -z "$out" ]; then out="$root/dev/out/$("$root/dev/lib/run-id.sh")/accept"; fi
mkdir -p "$out"
out=$(cd "$out" && pwd)

jail_env=()
for var in CASEMENT_TOOLS CASEMENT_START_TENTHS; do
  [ -z "${!var:-}" ] || jail_env+=(--env "$var=${!var}")
done
# More K=V pairs for the jails (the environment is otherwise cleared), space separated.
for pair in ${CASEMENT_JAIL_ENV:-}; do jail_env+=(--env "$pair"); done

# run_one NAME: one script in its own jail; leaves "<exit> <seconds>" in OUT/NAME.result.
run_one() {
  local name=$1 start end rc
  start=$(date +%s)
  set +e
  timeout --kill-after=10 "$bound" "$jail" --scratch "$out/$name" "${jail_env[@]}" -- \
    "$root/dev/accept/$name.sh" >"$out/$name.log" 2>&1 </dev/null
  rc=$?
  set -e
  end=$(date +%s)
  # timeout's own code for "the bound passed" is a budget overrun (4).
  [ "$rc" != 124 ] || rc=4
  echo "$rc $((end - start))" >"$out/$name.result"
}

passed=0 failed=0 skipped=0 worst=0 reported=()
# verdict RC: the word and the rank that decides the runner's exit code.
report_done() {
  local name rc secs seen
  for name in "${names[@]}"; do
    [ -f "$out/$name.result" ] || continue
    for seen in "${reported[@]:-}"; do [ "$seen" != "$name" ] || continue 2; done
    read -r rc secs <"$out/$name.result"
    reported+=("$name")
    case "$rc" in
      0) passed=$((passed + 1)); echo "PASS $name (${secs}s)" ;;
      2) skipped=$((skipped + 1)); echo "SKIP $name (${secs}s)"; [ "$worst" != 0 ] || worst=2 ;;
      *)
        failed=$((failed + 1))
        echo "FAIL $name (exit $rc, ${secs}s) log: $out/$name.log"
        tail -15 "$out/$name.log" | sed 's/^/    /'
        # 1 beats 3 beats 4 beats a skip; 5 stops everything.
        case "$rc" in
          5) worst=5 ;;
          1) [ "$worst" = 5 ] || worst=1 ;;
          3) [ "$worst" = 5 ] || [ "$worst" = 1 ] || worst=3 ;;
          *) [ "$worst" = 5 ] || [ "$worst" = 1 ] || [ "$worst" = 3 ] || worst=4 ;;
        esac ;;
    esac
  done
}

started=$(date +%s)
pids=()
for name in "${names[@]}"; do
  while [ "$(jobs -rp | wc -l)" -ge "$parallel" ]; do
    wait -n || true
    report_done
    if [ "$worst" = 5 ]; then break 2; fi
  done
  run_one "$name" &
  pids+=("$!")
done

if [ "$worst" != 5 ]; then
  while [ "$(jobs -rp | wc -l)" -gt 0 ]; do
    wait -n || true
    report_done
    if [ "$worst" = 5 ]; then break; fi
  done
fi
if [ "$worst" = 5 ]; then
  echo "JAIL BREACH: stopping the remaining scripts at once" >&2
  for pid in "${pids[@]}"; do kill "$pid" 2>/dev/null || true; done
fi
wait || true
report_done
echo "accept: $passed passed, $skipped skipped, $failed failed in $(( $(date +%s) - started ))s (logs: $out)"
exit "$worst"
