#!/usr/bin/env bash
# Acceptance script 'xwayland-override-redirect' (P1-X1): an override-redirect X11 window (a menu
# or tooltip) is not managed: X reports the attribute, it keeps the position the client asked
# for (a managed window is placed by casement), and it takes no keyboard focus from the managed
# window below it. Exit: 0 pass, 1 fail, 2 could not run, 3 environment.
#
# Owner: P1-X1.
set -euo pipefail
root=$(cd "$(dirname "$0")/../.." && pwd)
# shellcheck source=dev/lib/accept.sh
source "$root/dev/lib/accept.sh"
# shellcheck source=dev/lib/x11probe.sh
source "$root/dev/lib/x11probe.sh"

MAIN=XProbeMain
x11_compositor --clock stepped
plug_devices
x11_start X
x11_do X map main "class=$MAIN" size=400x300
wait_for "mapped $MAIN" >/dev/null
settle
read -r x y _ _ <<<"$(win_rect $MAIN)"
click $((x + 20)) $((y + 20))
x11_wait X '^x11: event main focus-in'

x11_do X map menu class=XProbeMenu size=120x80 at=$((x + 30)),$((y + 40)) kind=override-redirect
x11_wait X '^x11: event menu mapped'
x11_do X info menu
x11_do X info main
x11_wait X '^x11: info menu '
menu=$(x11_log X | grep '^x11: info menu ' | tail -1)
assert_ok "X reports the menu override-redirect" grep -q 'override-redirect=true' <<<"$menu"
assert_ok "the menu is where the client put it" grep -q "root-at=$((x + 30)),$((y + 40))" <<<"$menu"

before=$(x11_count X '^x11: event main focus-out')
inject "key a down" "key a up" >/dev/null
x11_do X sync
assert_eq "the managed window kept the keyboard focus" "$before" "$(x11_count X '^x11: event main focus-out')"
assert_ok "the key still reached the managed window" x11_has X '^x11: event main key-press'
assert_not "the menu got no key" x11_has X '^x11: event menu key-press'
x11_do X unmap menu
x11_wait X '^x11: event menu unmapped'
assert_summary
