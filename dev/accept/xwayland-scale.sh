#!/usr/bin/env bash
# Acceptance script 'xwayland-scale' (P1-X1): X11 windows at output scale 1 and 2. The facts
# recorded per window are the compositor's logical frame, the X size of the window, the screen's
# size and Xft.dpi; the assertion is that X sizes and the compositor's frame agree through the
# dpi scale (X size = frame * dpi / 96) on each output a window lands on. Exit: 0 pass, 1 fail,
# 2 could not run (a window did not reach the second output), 3 environment.
#
# Owner: P1-X1.
set -euo pipefail
root=$(cd "$(dirname "$0")/../.." && pwd)
# shellcheck source=dev/lib/accept.sh
source "$root/dev/lib/accept.sh"
# shellcheck source=dev/lib/x11probe.sh
source "$root/dev/lib/x11probe.sh"

x11_compositor --clock stepped \
  --outputs "HEADLESS-1:1920x1080@60000:120:0,0;HEADLESS-2:3840x2160@60000:240:1920,0"
plug_devices
x11_start X

# check NAME: the facts of one mapped window and the dpi-scaled agreement.
check() {
  local name=$1 class=$2 fx fy fw fh xw xh dpi
  wait_for "mapped $class" >/dev/null
  settle
  read -r fx fy fw fh <<<"$(win_rect "$class")"
  x11_do X info "$name"
  xw=$(x11_log X | sed -n "s/^x11: info $name size=\([0-9]*\)x.*/\1/p" | tail -1)
  xh=$(x11_log X | sed -n "s/^x11: info $name size=[0-9]*x\([0-9]*\) .*/\1/p" | tail -1)
  dpi=$(x11_log X | sed -n 's/^x11: xft-dpi //p' | tail -1)
  echo "fact: $name frame=${fx},${fy} ${fw}x${fh} x-size=${xw}x${xh} xft-dpi=$dpi"
  assert_ok "$name: Xft.dpi is published ($dpi)" test "$dpi" != none
  [ "$dpi" != none ] || return 0
  assert_eq "$name: X width = frame width * dpi / 96" "$fw" "$(awk -v x="$xw" -v d="$dpi" 'BEGIN { printf "%d", x * 96 / d + 0.5 }')"
  assert_eq "$name: X height = frame height * dpi / 96" "$fh" "$(awk -v x="$xh" -v d="$dpi" 'BEGIN { printf "%d", x * 96 / d + 0.5 }')"
}

x11_do X map one class=XProbeScaleOne size=300x200
x11_wait X '^x11: event one mapped'
check one XProbeScaleOne
read -r fx _ _ _ <<<"$(win_rect XProbeScaleOne)"
first=left; [ "$fx" -lt 1920 ] || first=right
echo "fact: the first window is on the $first output"

# A second window: casement places it by its own rules; the check runs on whichever output it
# lands on, and the script says so when both landed on one.
x11_do X map two class=XProbeScaleTwo size=300x200
x11_wait X '^x11: event two mapped'
check two XProbeScaleTwo
read -r sx _ _ _ <<<"$(win_rect XProbeScaleTwo)"
second=left; [ "$sx" -lt 1920 ] || second=right
if [ "$first" = "$second" ]; then
  assert_skip "windows on both outputs" "both windows landed on the $first output; run on hardware or place one"
fi
assert_summary
