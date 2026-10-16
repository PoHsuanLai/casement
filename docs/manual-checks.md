# Manual checks

Checks only a person at a real screen can make. Agents build the code and never run it.

## winit backend (nested window)

- [ ] `casement --backend winit --clock wall` in KDE Plasma Wayland at 2x: the picture is
  upright, fills the whole window (a 1440x900 logical window is a 2880x1800 output at scale 2),
  follows a resize of the window (the output changes mode, the picture refills it), clicks and
  typing reach a terminal, the scroll wheel scrolls it, and clicking away from the window with
  a key held does not leave the key repeating inside.

## Capture on a real seat (KMS)

- [ ] **Eyedropper**: on the tty seat, in an app that asks for a colour from the screen (a colour well's eyedropper, a portal
  `PickColor` caller), the screen holds still, a pick returns the colour of the pixel under the pointer (compare with a
  `grim` shot of the same spot) and Escape returns none. Before cas34 the sill manual check said the tty had no read-back; it has
  had one since P1-T and the display GPU's scenes (capgpu, dmabuf), so this is the first run on hardware.
- [ ] **Cursor in captures**: `grim -c shot.png` shows the pointer, `grim shot.png` does not (and neither does the eyedropper's
  picture); same with a dma-buf client (OBS screen capture with and without "capture cursor"). Try an app cursor (a text I-beam)
  as well as the arrow.
- [ ] **A region**: `grim -g "100,100 1x1" p.png` is one pixel of the right colour at scale 1 and at 1.5x (it covers the one
  rounded physical pixel); on the display GPU only that pixel crosses to the CPU (no stall to measure, but no failure either).
