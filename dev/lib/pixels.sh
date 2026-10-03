# pixel_at PNG X Y: prints the RGBA of one pixel of a capture as `R G B A` (0-255 each).
# Uses python3 with PIL, else ImageMagick; exit 3 when neither is installed, 1 for a pixel
# outside the image or an unreadable file.
#
# Owner: P0-T-dev. Sourceable.

pixel_at() {
  [ $# -eq 3 ] || { echo "usage: pixel_at PNG X Y" >&2; return 64; }
  local png=$1 x=$2 y=$3
  [ -r "$png" ] || { echo "pixel_at: cannot read $png" >&2; return 1; }
  if python3 -c 'import PIL' 2>/dev/null; then
    python3 - "$png" "$x" "$y" <<'PY'
import sys
from PIL import Image
path, x, y = sys.argv[1], int(sys.argv[2]), int(sys.argv[3])
image = Image.open(path).convert("RGBA")
if not (0 <= x < image.width and 0 <= y < image.height):
    sys.exit("pixel_at: (%d,%d) is outside %dx%d" % (x, y, image.width, image.height))
print(*image.getpixel((x, y)))
PY
  elif command -v magick >/dev/null; then
    magick "$png[1x1+$x+$y]" -alpha on -depth 8 -format '%[fx:int(255*r+0.5)] %[fx:int(255*g+0.5)] %[fx:int(255*b+0.5)] %[fx:int(255*a+0.5)]' info:
  else
    echo "pixel_at: needs python3 with PIL, or ImageMagick" >&2
    return 3
  fi
}
