from __future__ import annotations

import os
import platform
import sys
from importlib.resources import files


def _binary_path() -> str:
    system = platform.system()
    machine = platform.machine().lower()
    normalized_machine = {
        "amd64": "x86_64",
        "arm64": "aarch64",
    }.get(machine, machine)

    packages = {
        ("Linux", "x86_64"): "openclash_bin_linux_x64_musl",
        ("Linux", "aarch64"): "openclash_bin_linux_arm64_musl",
        ("Darwin", "aarch64"): "openclash_bin_darwin_arm64",
        ("Windows", "x86"): "openclash_bin_win32_ia32",
        ("Windows", "i386"): "openclash_bin_win32_ia32",
        ("Windows", "i686"): "openclash_bin_win32_ia32",
    }

    module = packages.get((system, normalized_machine))
    if module is None:
        raise RuntimeError(
            f"openclash PyPI package currently supports Linux x86_64, Linux arm64, macOS arm64, and Windows x86; got {system}-{machine}"
        )

    binary = "openclash.exe" if system == "Windows" else "openclash"
    return str(files(module).joinpath(f"bin/{binary}"))


def main() -> None:
    binary = _binary_path()
    os.chmod(binary, os.stat(binary).st_mode | 0o755)
    os.execv(binary, [binary, *sys.argv[1:]])


if __name__ == "__main__":
    main()
