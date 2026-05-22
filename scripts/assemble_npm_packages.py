#!/usr/bin/env python3
from __future__ import annotations

import argparse
import shutil
import stat
from pathlib import Path

from package_targets import TARGETS

ROOT = Path(__file__).resolve().parents[1]


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--dist-root", default="dist")
    args = parser.parse_args()

    dist_root = ROOT / args.dist_root
    for target in TARGETS:
        src = dist_root / target["platform_id"] / target["artifact_binary"]
        dst = (
            ROOT
            / "npm"
            / "packages"
            / target["npm_package"]
            / "bin"
            / target["package_binary"]
        )
        if not src.exists():
            raise SystemExit(f"missing build artifact: {src}")
        dst.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(src, dst)
        dst.chmod(dst.stat().st_mode | stat.S_IXUSR | stat.S_IXGRP | stat.S_IXOTH)
        print(f"[ok] assembled {dst}")


if __name__ == "__main__":
    main()
