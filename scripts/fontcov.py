#!/usr/bin/env python3
"""Glyph coverage matrix: which fonts can draw the characters tetron-wm emits.

Usage: fontcov.py [--md] [FONT ...]   FONT = .ttf/.otf/.ttc or .psf/.psf.gz path.
No args: a default set (kernel console PSFs + common monospace TTFs found locally).
Glyph inventory = every non-ASCII char in src/*.rs (minus comments/tests noise is
not filtered: extra chars only add rows) plus the chafa wallpaper symbol blocks.
"""
import glob, gzip, os, re, subprocess, sys, tempfile, unicodedata
from fontTools.ttLib import TTFont

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

def inventory():
    chars = {}
    for p in glob.glob(os.path.join(ROOT, "src", "**", "*.rs"), recursive=True):
        t = open(p, encoding="utf-8").read()
        t = re.sub(r"\\u\{([0-9A-Fa-f]+)\}", lambda m: chr(int(m.group(1), 16)), t)
        for c in t:
            if ord(c) > 127:
                chars.setdefault(c, set()).add(os.path.basename(p))
    return chars

# chafa wallpaper blocks: (name, lo, hi)
WALL = [("block-elements", 0x2580, 0x259F), ("sextant", 0x1FB00, 0x1FB3B),
        ("braille", 0x2800, 0x28FF), ("geometric", 0x25A0, 0x25FF)]

def psf_cov(path):
    with tempfile.TemporaryDirectory() as d:
        raw = os.path.join(d, "f.psf")
        data = open(path, "rb").read()
        if path.endswith(".gz"): data = gzip.decompress(data)
        open(raw, "wb").write(data)
        out = subprocess.run(["psfgettable", raw, "/dev/stdout"],
                             capture_output=True, text=True).stdout
    cov = set()
    for ln in out.splitlines():
        if ln.startswith("#") or not ln.strip(): continue
        for tok in ln.split()[1:]:
            m = re.fullmatch(r"U\+([0-9A-Fa-f]+)", tok)
            if m: cov.add(int(m.group(1), 16))
    return cov

def ttf_cov(path):
    f = TTFont(path, fontNumber=0, lazy=True)
    return set(f.getBestCmap().keys())

def load(path):
    n = os.path.basename(path).replace(".psf.gz", "").replace(".ttf", "")
    return n, (psf_cov(path) if ".psf" in path else ttf_cov(path))

DEFAULT = [
    "/usr/share/consolefonts/Lat15-Fixed16.psf.gz",
    "/usr/share/consolefonts/Lat15-Terminus16.psf.gz",
    "/usr/share/consolefonts/Uni2-Fixed16.psf.gz",
    "/usr/share/consolefonts/Uni3-Terminus16.psf.gz",
    "/usr/share/consolefonts/Uni2-VGA16.psf.gz",
    "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf",
    "/usr/share/fonts/truetype/liberation/LiberationMono-Regular.ttf",
    "/usr/share/fonts/truetype/noto/NotoSansMono-Regular.ttf",
    "/usr/share/fonts/truetype/terminus/TerminusTTF-4.46.0.ttf",
    os.path.expanduser("~/.local/share/fonts/SymbolsNerdFont-Regular.ttf"),
]

def main():
    a = [x for x in sys.argv[1:] if x != "--md"]
    fonts = [load(p) for p in (a or DEFAULT) if os.path.exists(p)]
    inv = inventory()
    names = [n for n, _ in fonts]
    print("| char | cp | name | files | " + " | ".join(names) + " |")
    print("|" + "---|" * (4 + len(names)))
    for c in sorted(inv, key=ord):
        if ord(c) in (0xFE0F, 0x200D): continue
        try: nm = unicodedata.name(c)
        except ValueError: nm = "?"
        fl = ",".join(sorted(inv[c]))[:30]
        row = ["Y" if ord(c) in cov else "-" for _, cov in fonts]
        print(f"| {c} | U+{ord(c):04X} | {nm} | {fl} | " + " | ".join(row) + " |")
    print()
    print("| wallpaper symbols | " + " | ".join(names) + " |")
    print("|" + "---|" * (1 + len(names)))
    for nm, lo, hi in WALL:
        tot = hi - lo + 1
        print(f"| {nm} U+{lo:04X}-{hi:04X} | " +
              " | ".join(f"{sum(1 for cp in range(lo, hi+1) if cp in cov)}/{tot}" for _, cov in fonts) + " |")

main()
