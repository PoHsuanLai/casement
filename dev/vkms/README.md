# dev/vkms

`accept.sh` is P1's exit criterion 1: casement's tty backend on a **vkms** device (the kernel's
virtual display driver), with no real display involved. It is yours to run: no agent runs it,
no gate runs it, and nothing in it uses sudo (20-FREEZE-PLAN C3). The one-time setup below is
the only part that needs root, and you type it.

The script picks the DRM node by driver name (`vkms`), refuses any other card, refuses to run
unless a terminal is attached and you type `vkms only`, and writes only into a scratch directory
it creates (kept when a run fails) plus the status of one vkms connector.

## One-time setup (you type these)

The device is made through configfs with its own name (`casement`), so the module's default
device is not created. Check first that your kernel has vkms and configfs support:
`modinfo vkms | grep -E 'enable_writeback|create_default_dev'`.

```bash
# 1. Load vkms with writeback and without the default device.
sudo modprobe vkms enable_writeback=1 create_default_dev=0

# 2. Build the device: one primary plane, one cursor plane, one CRTC with a writeback
#    connector, one encoder, and two connectors (connector0 is the screen; hotplug1 starts
#    disconnected and is what the hotplug step connects and disconnects).
sudo bash -c '
set -euo pipefail
d=/sys/kernel/config/vkms/casement
mkdir "$d"
mkdir "$d/planes/plane0" "$d/planes/cursor0" "$d/crtcs/crtc0" "$d/encoders/encoder0"
mkdir "$d/connectors/connector0" "$d/connectors/hotplug1"
echo 1 > "$d/planes/plane0/type"      # 1 = primary
echo 2 > "$d/planes/cursor0/type"     # 2 = cursor
echo 1 > "$d/crtcs/crtc0/writeback"
ln -s "$d/crtcs/crtc0" "$d/planes/plane0/possible_crtcs/crtc0"
ln -s "$d/crtcs/crtc0" "$d/planes/cursor0/possible_crtcs/crtc0"
ln -s "$d/crtcs/crtc0" "$d/encoders/encoder0/possible_crtcs/crtc0"
ln -s "$d/encoders/encoder0" "$d/connectors/connector0/possible_encoders/encoder0"
ln -s "$d/encoders/encoder0" "$d/connectors/hotplug1/possible_encoders/encoder0"
echo 2 > "$d/connectors/hotplug1/status"   # 2 = disconnected
echo 1 > "$d/enabled"
chown -R '"$USER"' "$d"
'

# 3. Let your user open the card node, and keep your desktop from adopting it as a screen.
sudo tee /etc/udev/rules.d/99-casement-vkms.rules >/dev/null <<'RULE'
SUBSYSTEM=="drm", KERNEL=="card[0-9]*", SUBSYSTEMS=="platform", DRIVERS=="vkms", GROUP="video", MODE="0660", TAG-="uaccess", TAG-="seat", TAG-="master-of-seat"
SUBSYSTEM=="drm", KERNEL=="renderD[0-9]*", SUBSYSTEMS=="platform", DRIVERS=="vkms", GROUP="video", MODE="0660", TAG-="uaccess"
RULE
sudo udevadm control --reload
sudo udevadm trigger --subsystem-match=drm --action=change
id -nG | grep -qw video || { sudo usermod -aG video "$USER"; echo "log out and in again for the video group"; }
```

Check: `ls -l /dev/dri/` shows a `card*` with group `video` mode `crw-rw----` for the vkms one,
and `readlink -f /sys/class/drm/card*/device/driver` lists `.../vkms` for exactly one card.

To undo: `sudo rmmod vkms` (remove the configfs device first with `sudo rmdir` of its items, or
reboot) and `sudo rm /etc/udev/rules.d/99-casement-vkms.rules`.

## Run

```bash
dev/bootstrap.sh          # outside the jail: builds casement and writeback-compare into dev/out/tools/bin
dev/vkms/accept.sh        # a terminal; type "vkms only"; takes a little over 10 minutes
```

Needs `jq` and `wlopm` (`sudo dnf install jq wlopm`). `CASEMENT_VKMS_IDLE_SECONDS=30` shortens
the idle step for a try-out; such a run ends with exit 5 (`VKMS PARTIAL`) even if every step
passed. `CASEMENT_VKMS_DEVICE` names the configfs device if you did not call it `casement`.

Exit codes: 0 all passed, 1 a step failed (scratch dir kept), 2 refused, 3 vkms or a tool is
absent (a gate script prints `SKIP-VKMS` for this one), 4 the casement binary has no vkms
hooks yet, 5 a short run.

## What each step asserts

| Step | Asserts |
|---|---|
| modeset | casement, started with `--backend tty --drm-node <vkms card>`, reports one lit output with a mode and a flip count above zero within 30 s |
| idle | for 600 s with no clients: casement stays up, **missed vblanks do not increase**, and flips advance at least 90% of `seconds x refresh` |
| hotplug | connecting the `hotplug1` connector (configfs `status` = 1) gives two outputs within 15 s, disconnecting it (2) gives one again, casement alive |
| writeback | SIGHUP makes casement read one frame back through the vkms writeback connector; `writeback-compare` holds it to `crates/casement-testkit/tests/golden/scenes/empty-desktop-1024x768.png` (pixman, empty desktop at 1024x768) within 3 steps per channel on all but 2000 pixels per million |
| dpms | `wlopm --off` reports the output `off` and the flips stop for 3 s; `wlopm --on` reports `on` and the flips restart |
| vt | SIGUSR1 (session paused) reports the seat `lent` and the flips stop; SIGUSR2 (activated) reports the seat `held`, the one output back on, flips running |

## casement's side (what this script needs from the binary)

The script talks to casement through four tty-backend options that are not part of the normal
CLI: `--drm-node`, `--kms-report`, `--writeback-raw`, and SIGUSR1/SIGUSR2 as the simulated
session. The contract is spelled out at the top of `accept.sh`. A casement without them makes
the script exit 4 before it starts anything. The writeback golden has to be blessed once with
`CASEMENT_BLESS=1` (the scene test `empty-desktop-1024x768`).
