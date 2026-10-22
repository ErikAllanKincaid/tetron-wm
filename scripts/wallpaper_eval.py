#!/usr/bin/env python3
"""Score chafa wallpaper settings: fidelity vs the source image and glyph usage.

Usage: wallpaper_eval.py IMAGE [WxH]     (default 160x44)
For each chafa configuration it renders cells, rebuilds a 2x2-subpixel image
from the glyph masks + fg/bg colours, and reports mean abs error against the
source resized to the same grid, the glyphs emitted, and whether every glyph
exists in the kernel console fonts (Lat15-Fixed16) and DejaVu Sans Mono.
"""
import re, subprocess, sys, collections
import numpy as np
from PIL import Image

ROOT = __import__("os").path.dirname(__import__("os").path.dirname(__import__("os").path.abspath(__file__)))
sys.path.insert(0, ROOT + "/scripts")
exec(open(ROOT + "/scripts/fontcov.py").read().split("def main")[0])

MASK = {" ": "0000", "█": "1111", "▀": "1100", "▄": "0011", "▌": "1010", "▐": "0101",
        "▘": "1000", "▝": "0100", "▖": "0010", "▗": "0001", "▚": "1001", "▞": "0110",
        "▛": "1110", "▜": "1101", "▙": "1011", "▟": "0111"}

SHADE = {"░": 0.25, "▒": 0.5, "▓": 0.75}

def xterm(n):
    if n < 16:
        b = [(0,0,0),(205,0,0),(0,205,0),(205,205,0),(0,0,238),(205,0,205),(0,205,205),(229,229,229),
             (127,127,127),(255,0,0),(0,255,0),(255,255,0),(92,92,255),(255,0,255),(0,255,255),(255,255,255)]
        return b[n]
    if n >= 232:
        v = 8 + 10 * (n - 232); return (v, v, v)
    n -= 16; lv = [0, 95, 135, 175, 215, 255]
    return (lv[n // 36], lv[n // 6 % 6], lv[n % 6])

def parse(out, w, h):
    fg = bg = (255, 255, 255); cells = []
    out = out.replace("\x1b[?25l", "").replace("\x1b[?25h", "")
    for line in out.split("\n"):
        if not line.strip("\x1b[0m"): continue
        row = []; i = 0
        while i < len(line):
            m = re.match(r"\x1b\[([0-9;]*)m", line[i:])
            if m:
                p = [int(x) if x else 0 for x in m.group(1).split(";")]; k = 0
                while k < len(p):
                    c = p[k]
                    if c == 0: fg = bg = (0, 0, 0) if False else fg
                    if c == 38 and p[k+1] == 2: fg = tuple(p[k+2:k+5]); k += 4
                    elif c == 48 and p[k+1] == 2: bg = tuple(p[k+2:k+5]); k += 4
                    elif c == 38 and p[k+1] == 5: fg = xterm(p[k+2]); k += 2
                    elif c == 48 and p[k+1] == 5: bg = xterm(p[k+2]); k += 2
                    elif 30 <= c <= 37: fg = xterm(c - 30)
                    elif 90 <= c <= 97: fg = xterm(c - 90 + 8)
                    elif 40 <= c <= 47: bg = xterm(c - 40)
                    elif 100 <= c <= 107: bg = xterm(c - 100 + 8)
                    k += 1
                i += m.end(); continue
            row.append((line[i], fg, bg)); i += 1
        if row: cells.append(row)
    return cells

def recon(cells, w, h):
    img = np.zeros((h * 2, w * 2, 3), float)
    for y, row in enumerate(cells[:h]):
        for x, (ch, fg, bg) in enumerate(row[:w]):
            m = MASK.get(ch); sh = SHADE.get(ch, 0.5)
            for s in range(4):
                c = (fg if m[s] == "1" else bg) if m else tuple(b + (a - b) * sh for a, b in zip(fg, bg))
                img[y*2 + s // 2, x*2 + s % 2] = c
    return img

CFG = [
 ("current all-wide, full, dither(no-op)", ["--symbols", "all-wide", "--dither", "ordered"]),
 ("all-wide, full", ["--symbols", "all-wide"]),
 ("half+quad+space+solid", ["--symbols", "half+quad+space+solid"]),
 ("block+space+solid", ["--symbols", "block+space+solid"]),
 ("vhalf+space+solid (kernel VT)", ["--symbols", "vhalf+space+solid"]),
 ("quad only", ["--symbols", "quad+space+solid"]),
 ("sextant+half+quad", ["--symbols", "half+quad+sextant+space+solid"]),
 ("half+quad, work=9", ["--symbols", "half+quad+space+solid", "-w", "9"]),
 ("half+quad, din99d", ["--symbols", "half+quad+space+solid", "--color-space", "din99d"]),
 ("half+quad, median", ["--symbols", "half+quad+space+solid", "--color-extractor", "median"]),
 ("half+quad, -c 256", ["--symbols", "half+quad+space+solid", "-c", "256"]),
 ("half+quad, -c 256 ordered", ["--symbols", "half+quad+space+solid", "-c", "256", "--dither", "ordered"]),
 ("half+quad, -c 256 diffusion", ["--symbols", "half+quad+space+solid", "-c", "256", "--dither", "diffusion"]),
 ("half+quad, -c 16", ["--symbols", "half+quad+space+solid", "-c", "16"]),
 ("half+quad, -c 16 diffusion", ["--symbols", "half+quad+space+solid", "-c", "16", "--dither", "diffusion"]),
 ("VT: space+solid+shades, full", ["--symbols", "space+solid+0x2591..0x2592"]),
 ("VT: space+solid+shades, -c 16", ["--symbols", "space+solid+0x2591..0x2592", "-c", "16"]),
 ("VT: space+solid+shades, -c 16 diffusion", ["--symbols", "space+solid+0x2591..0x2592", "-c", "16", "--dither", "diffusion"]),
 ("VT: space+solid (no shades), -c 16", ["--symbols", "space+solid", "-c", "16"]),
 ("vhalf, -c 16 diffusion", ["--symbols", "vhalf+space+solid", "-c", "16", "--dither", "diffusion"]),
]

def main():
    path = sys.argv[1]; w, h = map(int, (sys.argv[2] if len(sys.argv) > 2 else "160x44").split("x"))
    src = np.asarray(Image.open(path).convert("RGB").resize((w * 2, h * 2), Image.LANCZOS), float)
    fonts = {n: c for n, c in [load(p) for p in
             ["/usr/share/consolefonts/Lat15-Fixed16.psf.gz", "/usr/share/consolefonts/Uni2-Fixed16.psf.gz",
              "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf"]]}
    print(f"{path} {w}x{h}\n| config | MAE | glyphs | not in Lat15 | not in Uni2 | not in DejaVu |\n|---|---|---|---|---|---|")
    for name, a in CFG:
        out = subprocess.run(["chafa", "-f", "symbols", "-c", "full", "--stretch", "-s", f"{w}x{h}", *a, path],
                             capture_output=True, text=True).stdout
        cells = parse(out, w, h)
        g = collections.Counter(ch for r in cells for ch, _, _ in r)
        mae = np.abs(recon(cells, w, h) - src).mean()
        miss = lambda k: sum(n for ch, n in g.items() if ord(ch) not in fonts[k])
        ks = list(fonts)
        if len(sys.argv) > 3:
            Image.fromarray(recon(cells, w, h).clip(0, 255).astype("uint8")).resize((w * 4, h * 4), Image.NEAREST).save(
                f"{sys.argv[3]}/{re.sub('[^a-z0-9]+', '_', name.lower())}.png")
        print(f"| {name} | {mae:.2f} | {len(g)} | {miss(ks[0])} | {miss(ks[1])} | {miss(ks[2])} |")
if __name__ == "__main__":
    main()
