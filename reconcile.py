#!/usr/bin/env python3
# reconcile.py -- run from the tetron-wm repo root.
# Checks the automatable packaging constraints (CON-WD*) in spec/constraints.py.
# Requirements (WMDEB-*) are verified by cargo test, review and the VM passes.
import json
import sys
import tomllib
from pathlib import Path

# mkdeb's word for the binary passed to it.
BUILT = {"binary", "asset"}


def check_packaging() -> dict:
    problems = []
    for p in sorted(Path("packaging").glob("*.toml")):
        try:
            d = tomllib.loads(p.read_text())
        except tomllib.TOMLDecodeError as e:
            problems.append(f"{p}: {e}")
            continue
        problems += [f"{p}: has {k}" for k in ("scripts", "maintainer") if k in d]
        for f in d.get("files", []):
            src, dst = f.get("src", ""), f.get("dst", "")
            # mkdeb reads paths relative to the toml file.
            if src not in BUILT and not (p.parent / src).is_file():
                problems.append(f"{p}: src {src} missing")
            if not dst.startswith("usr/"):
                problems.append(f"{p}: dst {dst} not under usr/")
    return {"valid": not problems, "problems": problems}


def check_lean_deb() -> dict:
    p = Path("packaging/deb.toml")
    if not p.exists():
        return {"ok": False, "problems": ["packaging/deb.toml missing"]}
    d = tomllib.loads(p.read_text())
    problems = [f"{p}: declares {k}" for k in ("depends", "recommends") if d.get(k)]
    return {"ok": not problems, "problems": problems}


def check_desktop_deb() -> dict:
    p = Path("packaging/desktop.toml")
    if not p.exists():
        return {"ok": False, "problems": ["packaging/desktop.toml missing"]}
    d = tomllib.loads(p.read_text())
    problems = [f"{p}: has a binary file entry" for f in d.get("files", []) if f.get("src") in BUILT]
    if not any(dep.split()[0] == "tetron-wm" for dep in d.get("depends", [])):
        problems.append(f"{p}: depends does not name tetron-wm")
    return {"ok": not problems, "problems": problems}


def check_example_config() -> dict:
    hits = [str(p) for p in Path("src").rglob("*.rs") if 'include_str!("../config.example.toml")' in p.read_text()]
    return {"compiled_in": bool(hits), "files": hits}


if __name__ == "__main__":
    ctx = {"packaging": check_packaging(), "lean_deb": check_lean_deb(), "desktop_deb": check_desktop_deb(), "example_config": check_example_config()}
    print(json.dumps(ctx, indent=2))
    sys.exit(0 if ctx["packaging"]["valid"] and ctx["lean_deb"]["ok"] and ctx["desktop_deb"]["ok"] and ctx["example_config"]["compiled_in"] else 1)
