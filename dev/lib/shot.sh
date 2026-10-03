# shot TARGET OUT.png: captures an output, a window or a region through `casementctl shot`
# (an ext-image-copy-capture client, P1-C1). TARGET is `output`, `output:NAME`, `window:ID` or
# `region:X,Y,W,H`; a TARGET starting `masked:` (`masked:output`) masks sensitive regions. Until
# the compositor has the capture protocols casementctl exits 2 (could not run), and so does this.
#
# Owner: P0-T-dev. Sourceable.

# shellcheck source=dev/lib/inject.sh
source "$(dirname "${BASH_SOURCE[0]}")/inject.sh"

shot() {
  require_jail
  [ $# -eq 2 ] || { echo "usage: shot TARGET OUT.png   (output[:NAME] | window:ID | region:X,Y,W,H)" >&2; return 64; }
  _session || return 1
  local ctl target=$1 out=$2 flags=()
  ctl=$(casement_tool casementctl) || return 3
  case "$target" in masked:*) flags+=(--masked); target=${target#masked:} ;; esac
  case "$target" in
    output) ;;
    output:*) flags+=(--output "${target#output:}") ;;
    window:*) flags+=(--window "${target#window:}") ;;
    region:*) flags+=(--region "${target#region:}") ;;
    *) echo "shot: unknown target '$target'" >&2; return 64 ;;
  esac
  mkdir -p "$(dirname "$out")"
  "$ctl" shot "${flags[@]}" "$out"
}
