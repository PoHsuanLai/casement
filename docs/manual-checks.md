# Manual checks

Checks only a person at a real screen can make. Agents build the code and never run it.

## winit backend (nested window)

- [ ] `casement --backend winit --clock wall` in KDE Plasma Wayland at 2x: the picture is
  upright, fills the whole window (a 1440x900 logical window is a 2880x1800 output at scale 2),
  follows a resize of the window (the output changes mode, the picture refills it), clicks and
  typing reach a terminal, the scroll wheel scrolls it, and clicking away from the window with
  a key held does not leave the key repeating inside.
