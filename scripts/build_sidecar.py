from __future__ import annotations

import os
import platform
import shutil
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
BUILD = ROOT / ".tmp" / "sidecar-build"
BINARIES = ROOT / "src-tauri" / "binaries"


def target_triple() -> str:
    output = subprocess.run(["rustc", "-vV"], check=True, capture_output=True, text=True).stdout
    return next(line.split(": ", 1)[1] for line in output.splitlines() if line.startswith("host: "))


def main() -> None:
    BUILD.mkdir(parents=True, exist_ok=True)
    BINARIES.mkdir(parents=True, exist_ok=True)
    name = "llm-wiki-sidecar"
    subprocess.run(
        [
            "pyinstaller",
            "--noconfirm",
            "--clean",
            "--onefile",
            "--name",
            name,
            "--collect-data",
            "llm_wiki",
            "--distpath",
            str(BUILD / "dist"),
            "--workpath",
            str(BUILD / "work"),
            "--specpath",
            str(BUILD),
            str(ROOT / "scripts" / "sidecar_entry.py"),
        ],
        cwd=ROOT,
        check=True,
    )
    suffix = ".exe" if platform.system() == "Windows" else ""
    source = BUILD / "dist" / f"{name}{suffix}"
    destination = BINARIES / f"{name}-{target_triple()}{suffix}"
    staged = destination.with_suffix(destination.suffix + ".new")
    shutil.copy2(source, staged)
    os.replace(staged, destination)
    destination.chmod(0o755)
    print(f"packaged sidecar: {destination.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
