#!/usr/bin/env bash
# New global, new test file (CONVENTIONS addendum 11): every protocol module
# crates/casement-wayland/src/<p>.rs has crates/casement-testkit/tests/it/protocol/<p>.rs and a
# `mod <p>;` line in that directory's mod.rs; every module named in
# crates/casement-testkit/tests/golden/globals.txt (`<interface> <version> <module>` per line)
# is one of those covered modules.
set -euo pipefail
shopt -s nullglob
cd "$(dirname "$0")/.."
src=crates/casement-wayland/src
tests=crates/casement-testkit/tests/it/protocol
globals=crates/casement-testkit/tests/golden/globals.txt
fail=0

if [ ! -d "$src" ]; then
  echo "holds: casement-wayland is not a member yet, nothing to cover"
  exit 0
fi

# covered <module>: 0 when the module has its test file and its mod line.
covered() {
  [ -f "$tests/$1.rs" ] && [ -f "$tests/mod.rs" ] && grep -Eq "^[[:space:]]*(pub[[:space:]]+)?mod[[:space:]]+$1[[:space:]]*;" "$tests/mod.rs"
}

for file in "$src"/*.rs; do
  p=$(basename "$file" .rs)
  case "$p" in lib|states|serial|convert|client) continue ;; esac
  if covered "$p"; then echo "covered: $p"; else echo "MISSING: $p (needs $tests/$p.rs and 'mod $p;' in $tests/mod.rs)"; fail=1; fi
done

if [ -f "$globals" ]; then
  while read -r interface version module rest; do
    case "$interface" in ''|\#*) continue ;; esac
    if [ -z "${module:-}" ]; then echo "MISSING: $globals line '$interface $version' names no module"; fail=1; continue; fi
    if ! covered "$module"; then echo "MISSING: $interface ($globals) names module '$module' with no test file"; fail=1; fi
  done <"$globals"
else
  echo "MISSING: $globals"
  fail=1
fi
exit "$fail"
