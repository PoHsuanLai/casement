# assert_eq NAME EXPECTED ACTUAL: prints `PASS NAME` or `FAIL NAME (expected X, got Y)` and
# records a failure; it returns 0 either way, so a `set -e` script carries on to its summary.
# assert_skip NAME WHY prints `SKIP NAME (WHY)` and records a skip.
# assert_summary prints a count line and exits with the plan's code (20-FREEZE-PLAN 6.1): 1 when
# anything failed, else 2 when anything was skipped (could not run), else 0. A script ends with it.
#
# Owner: P0-T-dev. Sourceable. Counters are shell variables, so call these in the script's own
# shell, never in a pipeline or a $( ) subshell.

ASSERT_PASSED=${ASSERT_PASSED:-0}
ASSERT_FAILED=${ASSERT_FAILED:-0}
ASSERT_SKIPPED=${ASSERT_SKIPPED:-0}

assert_eq() {
  [ $# -eq 3 ] || { echo "usage: assert_eq NAME EXPECTED ACTUAL" >&2; return 64; }
  if [ "$2" = "$3" ]; then
    ASSERT_PASSED=$((ASSERT_PASSED + 1))
    echo "PASS $1"
  else
    ASSERT_FAILED=$((ASSERT_FAILED + 1))
    echo "FAIL $1 (expected '$2', got '$3')"
  fi
}

assert_skip() {
  ASSERT_SKIPPED=$((ASSERT_SKIPPED + 1))
  echo "SKIP $1 (${2:-no reason given})"
}

assert_summary() {
  echo "assert: $ASSERT_PASSED passed, $ASSERT_FAILED failed, $ASSERT_SKIPPED skipped"
  if [ "$ASSERT_FAILED" -gt 0 ]; then exit 1; fi
  if [ "$ASSERT_SKIPPED" -gt 0 ]; then exit 2; fi
  exit 0
}
