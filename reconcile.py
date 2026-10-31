#!/usr/bin/env python3
# reconcile.py -- run from the tetron-wm repo root.
# Checks the automatable packaging constraints (CON-WD*) in spec/constraints.py.
# Requirements (WMDEB-*) are verified by cargo test, review and the VM passes.
import json
import sys
import tomllib
from pathlib import Path

BUILT = {"target/release/tetron-wm", "tetron-wm"}


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
            if src not in BUILT and not Path(src).exists():
                problems.append(f"{p}: src {src} missing")
            if not dst.startswith("usr/"):
                problems.append(f"{p}: dst {dst} not under usr/")
    return {"valid": not problems, "problems": problems}


def check_example_config() -> dict:
    hits = [str(p) for p in Path("src").rglob("*.rs") if 'include_str!("../config.example.toml")' in p.read_text()]
    return {"compiled_in": bool(hits), "files": hits}


if __name__ == "__main__":
    ctx = {"packaging": check_packaging(), "example_config": check_example_config()}
    print(json.dumps(ctx, indent=2))
    sys.exit(0 if ctx["packaging"]["valid"] and ctx["example_config"]["compiled_in"] else 1)
