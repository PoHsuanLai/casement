#!/usr/bin/env python3
"""Pixel checks for dev/accept/scale-crisp.sh (python3 with PIL and numpy, both on the host).

  crisp.py checker PNG X0 Y0 W H CELL [INSET]
      The window's buffer starts at physical pixel (X0, Y0) and is W x H pixels, drawn as a
      checkerboard of CELL-pixel squares, white first. Every pixel of the buffer inside INSET
      pixels (default 18: a multiple of two cells for CELL 3) of its edge must be exactly that
      checkerboard: opaque black and white only, squares exactly CELL pixels wide. Prints the
      count of wrong pixels and, for the first few, where; exit 0 when none.
  crisp.py same A.png B.png
      Exit 0 when the two images are the same size and byte-identical.
  crisp.py black PNG CX CY REACH
      Every pixel at least REACH pixels from (CX, CY) (on either axis) must be opaque black: what a
      session lock's cover shows, bar the cursor.
  crisp.py size PNG
      Prints `WxH`.
"""
import sys

import numpy as np
from PIL import Image


def load(path):
    return np.asarray(Image.open(path).convert("RGBA"))


def checker(argv):
    png, x0, y0, w, h, cell = argv[0], *map(int, argv[1:6])
    inset = int(argv[6]) if len(argv) > 6 else 18
    image = load(png)
    ys, xs = np.mgrid[inset:h - inset, inset:w - inset]
    white = ((xs // cell + ys // cell) % 2) == 0
    want = np.where(white[..., None], 255, 0).astype(np.uint8)
    want = np.concatenate([want, want, want, np.full(white.shape + (1,), 255, np.uint8)], axis=2)
    got = image[y0 + inset:y0 + h - inset, x0 + inset:x0 + w - inset]
    if got.shape != want.shape:
        print(f"region {x0},{y0} {w}x{h} lies outside the {image.shape[1]}x{image.shape[0]} capture")
        return 1
    wrong = np.argwhere((got != want).any(axis=2))
    print(f"checked {want.shape[1]}x{want.shape[0]} pixels at {x0 + inset},{y0 + inset}: {len(wrong)} wrong")
    for (dy, dx) in wrong[:5]:
        print(f"  ({x0 + inset + dx},{y0 + inset + dy}) is {tuple(got[dy, dx])}, expected {tuple(want[dy, dx])}")
    return 0 if len(wrong) == 0 else 1


def same(argv):
    a, b = load(argv[0]), load(argv[1])
    if a.shape != b.shape:
        print(f"sizes differ: {a.shape[1]}x{a.shape[0]} against {b.shape[1]}x{b.shape[0]}")
        return 1
    wrong = np.argwhere((a != b).any(axis=2))
    print(f"{len(wrong)} of {a.shape[0] * a.shape[1]} pixels differ")
    for (y, x) in wrong[:5]:
        print(f"  ({x},{y}) {tuple(a[y, x])} against {tuple(b[y, x])}")
    return 0 if len(wrong) == 0 else 1


def black(argv):
    image = load(argv[0])
    cx, cy, reach = map(int, argv[1:4])
    ys, xs = np.mgrid[0:image.shape[0], 0:image.shape[1]]
    away = (np.abs(xs - cx) >= reach) | (np.abs(ys - cy) >= reach)
    lit = away & (image != np.array([0, 0, 0, 255], np.uint8)).any(axis=2)
    print(f"{int(lit.sum())} of {int(away.sum())} pixels away from the cursor are not black")
    for (y, x) in np.argwhere(lit)[:5]:
        print(f"  ({x},{y}) is {tuple(image[y, x])}")
    return 0 if not lit.any() else 1


def size(argv):
    image = load(argv[0])
    print(f"{image.shape[1]}x{image.shape[0]}")
    return 0


if __name__ == "__main__":
    verbs = {"checker": checker, "same": same, "black": black, "size": size}
    if len(sys.argv) < 3 or sys.argv[1] not in verbs:
        sys.exit(__doc__)
    sys.exit(verbs[sys.argv[1]](sys.argv[2:]))
