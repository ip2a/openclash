#!/usr/bin/env python3
from __future__ import annotations

import json
import re
from pathlib import Path

from package_targets import TARGETS

ROOT = Path(__file__).resolve().parents[1]


def cargo_version() -> str:
    text = (ROOT / "Cargo.toml").read_text(encoding="utf-8")
    match = re.search(r'^version\s*=\s*"([^"]+)"', text, flags=re.MULTILINE)
    if not match:
        raise SystemExit("Cargo.toml version is missing")
    return match.group(1)


def replace_version(path: Path, version: str) -> None:
    text = path.read_text(encoding="utf-8")
    text = re.sub(r'(?m)^version = "[^"]+"', f'version = "{version}"', text)
    text = re.sub(r'(openclash-bin-[A-Za-z0-9-]+)==[^";]+', rf"\1=={version}", text)
    path.write_text(text, encoding="utf-8")


def sync_npm(version: str) -> None:
    npm_packages_dir = ROOT / "npm" / "packages"
    for package_json in sorted(npm_packages_dir.glob("*/package.json")):
        data = json.loads(package_json.read_text(encoding="utf-8"))
        data["version"] = version
        optional = data.get("optionalDependencies")
        if optional:
            for target in TARGETS:
                package_name = target["npm_package"]
                if package_name in optional:
                    optional[package_name] = version
        package_json.write_text(json.dumps(data, indent=2) + "\n", encoding="utf-8")


def sync_python(version: str) -> None:
    for pyproject in sorted((ROOT / "python" / "packages").glob("*/pyproject.toml")):
        replace_version(pyproject, version)


def main() -> None:
    version = cargo_version()
    sync_npm(version)
    sync_python(version)
    print(f"[ok] synced package versions to {version}")


if __name__ == "__main__":
    main()
