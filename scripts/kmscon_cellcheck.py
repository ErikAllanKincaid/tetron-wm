#!/usr/bin/env python3
"""Emulate kmscon's pango cell-width test (font_pango.c get_glyph) for chars.

kmscon renders each char as a PangoLayout on the primary font (family from
--font-name) and marks it double-width when the logical extent exceeds one cell.
Usage: kmscon_cellcheck.py [FAMILY] [PX]   (default "DejaVu Sans Mono" 16)
Prints chars whose fallback-font advance does not fit one cell.
"""
import sys
import gi
gi.require_version("Pango", "1.0"); gi.require_version("PangoCairo", "1.0")
from gi.repository import Pango, PangoCairo
import cairo

fam = sys.argv[1] if len(sys.argv) > 1 else "DejaVu Sans Mono"
px = int(sys.argv[2]) if len(sys.argv) > 2 else 16
ctx = PangoCairo.font_map_get_default().create_context()
surf = cairo.ImageSurface(cairo.FORMAT_ARGB32, 1, 1)
ctx = PangoCairo.create_context(cairo.Context(surf))
desc = Pango.FontDescription(fam); desc.set_absolute_size(px * Pango.SCALE)

def ext(s):
    l = Pango.Layout(ctx); l.set_font_description(desc); l.set_text(s, -1)
    ink, log = l.get_pixel_extents(); return log.width, l.get_baseline() // Pango.SCALE

cell = round(ext("abcdefghijklmnopqrstuvwxyz0123456789")[0] / 36 + 1)
print(f"family={fam!r} px={px} cell_w={cell}")
sets = {
 "blocks 2580-259F": range(0x2580, 0x25A0), "sextants 1FB00-3B": range(0x1FB00, 0x1FB3C),
 "braille": range(0x2800, 0x2900), "geometric": range(0x25A0, 0x2600),
 "chrome": [ord(c) for c in "✦⊞ⓘ⌨★☆✕✓▸◂▾⏎⌘⌕❐⚠⚙♪♫↔⇒‖"],
 "nerd": [0xF07B, 0xF120, 0xF0F6, 0xF1C9, 0xF2DB, 0xF240, 0xF028, 0xF293],
}
for name, cps in sets.items():
    cps = list(cps); wide = [c for c in cps if ext(chr(c))[0] > cell]
    print(f"{name:20} {len(cps)-len(wide):4}/{len(cps)} fit one cell; wide: "
          + " ".join(chr(c) for c in wide[:24]) + (" ..." if len(wide) > 24 else ""))
