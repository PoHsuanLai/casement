# Sourceable: provides `require_jail` (20-FREEZE-PLAN §6.1, C40) by sourcing the desktop
# harness's harness/lib/jail.sh, found as a sibling checkout: $CASEMENT_DESKTOP, then
# <repo>/../desktop, then $HOME/desktop. Missing: prints the fix and exits 3.
_casement_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
_casement_desktop=""
for _d in "${CASEMENT_DESKTOP:-}" "$_casement_root/../desktop" "$HOME/desktop"; do
  if [ -n "$_d" ] && [ -f "$_d/harness/lib/jail.sh" ]; then _casement_desktop=$_d; break; fi
done
if [ -z "$_casement_desktop" ]; then
  echo "dev/lib/jail.sh: the desktop harness (harness/lib/jail.sh) was not found" >&2
  echo "fix: check out ~/desktop beside casement, or set CASEMENT_DESKTOP" >&2
  exit 3
fi
# shellcheck source=/dev/null
source "$_casement_desktop/harness/lib/jail.sh"
unset _d
