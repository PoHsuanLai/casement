#!/usr/bin/env bash
# Acceptance script 'xwayland-clipboard' (P1-X1): the clipboard crosses in both directions through
# xwayland-satellite. An X11 client owns CLIPBOARD and a Wayland client pastes its text; a Wayland
# client copies and the X11 client pastes it. Exit: 0 pass, 1 fail, 2 could not run, 3 environment.
#
# Owner: P1-X1.
set -euo pipefail
root=$(cd "$(dirname "$0")/../.." && pwd)
# shellcheck source=dev/lib/accept.sh
source "$root/dev/lib/accept.sh"
# shellcheck source=dev/lib/x11probe.sh
source "$root/dev/lib/x11probe.sh"

X=XProbeClip
W=org.example.clip
x11_compositor --clock stepped
plug_devices
x11_start X
probe_start A
x11_do X map w "class=$X" size=300x200
probe_do A toplevel w "app=$W" size=300x200 pattern=gradient
probe_wait A '^probe: mapped w '
wait_for "mapped $W" >/dev/null
settle

# X11 to Wayland: the X client owns the selection, the Wayland client (focused by a click) pastes.
read -r wx wy _ _ <<<"$(win_rect $W)"
click $((wx + 20)) $((wy + 20))
probe_sync A
x11_do X own-clipboard "from x11 to wayland"
probe_sync A
probe_wait A '^< wl_data_device#[0-9]+\.selection\('
probe_do A paste
probe_wait A 'from x11 to wayland'
assert_ok "the Wayland client pasted the X11 text" probe_has A 'from x11 to wayland'

# Wayland to X11: the Wayland client copies, the X client (focused by a click) pastes.
read -r xx xy _ _ <<<"$(win_rect $X)"
click $((xx + 20)) $((xy + 20))
x11_do X sync
probe_do A copy "from wayland to x11"
probe_sync A
x11_do X paste
x11_wait X '^x11: clipboard-text from wayland to x11'
assert_ok "the X11 client pasted the Wayland text" x11_has X '^x11: clipboard-text from wayland to x11'
assert_summary
