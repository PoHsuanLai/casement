#!/usr/bin/env bash
# Takes a screenshot of a running jailed casement: dev/shot.sh [--output NAME | --window ID |
# --region X,Y,W,H] OUT.png, through casementctl shot (ext-image-copy-capture). Calls require_jail first.
#
# Owner: P0-T-dev. Frozen at P0-Z2 as a stub: the name and the command line are fixed, the body is not
# written yet. Exit 2 (could not run) until it is.
set -euo pipefail
root=$(cd "$(dirname "$0")" && pwd)
while [ ! -f "$root/dev/lib/jail.sh" ]; do root=$(dirname "$root"); [ "$root" != / ] || { echo "no dev/lib/jail.sh above $0" >&2; exit 3; }; done
# shellcheck source=dev/lib/jail.sh
source "$root/dev/lib/jail.sh"
require_jail
echo "shot.sh: not written yet (P0-T-dev)" >&2
exit 2
