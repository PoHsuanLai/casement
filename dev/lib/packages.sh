# shellcheck shell=bash
# Sourced by dev/preflight.sh, dev/bootstrap.sh, dev/lib/accept.sh and dev/vkms/accept.sh: the
# package names a missing tool or library comes from, per distribution family, so a hint prints
# the line that works on the machine it is read on.
#
#   pkg_family          fedora, debian or arch (from /etc/os-release ID and ID_LIKE); unknown otherwise
#   pkg_hint KEY...     the full install line (dnf, apt or pacman); on an unknown family all three
#
# Keys: bubblewrap pkgconf jq wlopm cpio rpm foot smithay-libs. $OS_RELEASE names another
# os-release file (a test's). The names come from the audit in ~/rs-wt/distro-audit/REPORT.md.

# pkg_family [OS_RELEASE_FILE]
pkg_family() {
  local file="${1:-${OS_RELEASE:-/etc/os-release}}" ID="" ID_LIKE=""
  if [ -r "$file" ]; then
    # shellcheck disable=SC1090
    ID=$(. "$file" && printf '%s' "${ID:-}") ID_LIKE=$(. "$file" && printf '%s' "${ID_LIKE:-}")
  fi
  case " $ID $ID_LIKE " in
    *" fedora "*|*" rhel "*|*" centos "*) echo fedora ;;
    *" debian "*|*" ubuntu "*) echo debian ;;
    *" arch "*) echo arch ;;
    *) echo unknown ;;
  esac
}

# _pkg_names FAMILY KEY: the names for one key; nothing when the family has no such package.
_pkg_names() {
  case "$2" in
    bubblewrap|jq|foot|cpio) echo "$2" ;;
    pkgconf)
      case "$1" in
        fedora) echo pkgconf-pkg-config ;;
        debian) echo pkg-config ;;
        arch) echo pkgconf ;;
      esac ;;
    wlopm) echo wlopm ;; # Ubuntu 24.04: wlopm 0.1.0-1 in universe; Debian trixie: wlopm (packages.ubuntu.com, read 2026-10-16)
    rpm) [ "$1" != fedora ] || echo rpm ;;
    smithay-libs)
      case "$1" in
        fedora) echo libseat-devel libinput-devel systemd-devel mesa-libgbm-devel libdrm-devel libxkbcommon-devel pipewire-devel wayland-devel pixman-devel libglvnd-devel ;;
        debian) echo libseat-dev libinput-dev libudev-dev libgbm-dev libdrm-dev libxkbcommon-dev libpipewire-0.3-dev libwayland-dev libpixman-1-dev libegl-dev libgles-dev ;;
        arch) echo seatd libinput systemd-libs libdrm libxkbcommon pipewire wayland pixman mesa libglvnd ;;
      esac ;;
    *) echo "packages.sh: no package list for $2" >&2; return 1 ;;
  esac
}

# _pkg_line FAMILY KEY...: one install line.
_pkg_line() {
  local family="$1" key names=() tool
  shift
  for key in "$@"; do
    # shellcheck disable=SC2207
    names+=($(_pkg_names "$family" "$key"))
  done
  case "$family" in
    fedora) tool="sudo dnf install" ;;
    debian) tool="sudo apt install" ;;
    arch) tool="sudo pacman -S" ;;
  esac
  echo "$tool ${names[*]}"
}

# pkg_hint KEY...: the install line for this machine; on an unknown family one line per family.
pkg_hint() {
  local family
  family=$(pkg_family)
  if [ "$family" != unknown ]; then
    _pkg_line "$family" "$@"
    return 0
  fi
  echo "on Fedora: $(_pkg_line fedora "$@")"
  echo "on Debian or Ubuntu: $(_pkg_line debian "$@")"
  echo "on Arch: $(_pkg_line arch "$@")"
}
