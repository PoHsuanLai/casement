#!/usr/bin/env bash
# Every backticked repo path in the repo docs (ARCHITECTURE.md, FINDINGS.md, CONVENTIONS.md,
# QUIRKS.md) that starts with crates/, dev/, scripts/, wlcs-casement/ or casement-scene-css/
# exists. A trailing `:line` or `#anchor` is stripped; paths holding `<`, `*`, `{` or an ellipsis
# are patterns, not paths, and are skipped.
set -euo pipefail
cd "$(dirname "$0")/.."
fail=0
for doc in ARCHITECTURE.md FINDINGS.md CONVENTIONS.md QUIRKS.md; do
  [ -f "$doc" ] || continue
  while IFS= read -r path; do
    case "$path" in *'<'*|*'*'*|*'{'*|*'…'*) continue ;; esac
    path="${path%%#*}"
    path=$(printf '%s' "$path" | sed -E 's/:[0-9]+(-[0-9]+)?$//')
    if [ ! -e "$path" ]; then echo "MISSING: $doc names \`$path\`"; fail=1; fi
  done < <(grep -oE '`(crates|dev|scripts|wlcs-casement|casement-scene-css)/[^` ]*`' "$doc" | tr -d '`' | sort -u)
done
[ "$fail" = 0 ] && echo "holds: every doc path exists"
exit "$fail"
