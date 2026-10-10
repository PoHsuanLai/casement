#!/usr/bin/env bash
# Phase tier: every recorded KMS fixture has the shape of `KmsResources` (the FakeKms suite
# replays them in full: tests/it/tty/kms.rs). Never touches hardware.
set -euo pipefail
dir="$(cd "$(dirname "$0")/.." && pwd)/crates/casement-testkit/fixtures/kms"
count=0
for file in "$dir"/*.json; do
  [ -e "$file" ] || continue
  python3 -I - "$file" <<'PY'
import json, sys
path = sys.argv[1]
doc = json.load(open(path))
for key in ("connectors", "crtcs", "planes"):
    if not isinstance(doc.get(key), list):
        sys.exit(f"{path}: missing list '{key}'")
for c in doc["connectors"]:
    for key in ("id", "name", "status", "modes", "physical", "edid", "vrr", "possible_crtcs"):
        if key not in c:
            sys.exit(f"{path}: connector without '{key}'")
PY
  count=$((count + 1))
done
echo "kms fixtures: $count parse"
