"""Download and verify the pinned offline desktop embedding model."""

from __future__ import annotations

import hashlib
import json
import shutil
import urllib.request
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
MODEL_DIR = ROOT / "src-tauri" / "resources" / "embedding-model"
MANIFEST = MODEL_DIR / "manifest.json"


def digest(path: Path) -> str:
    hasher = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            hasher.update(chunk)
    return hasher.hexdigest()


def valid(path: Path, size: int, expected: str) -> bool:
    return path.is_file() and path.stat().st_size == size and digest(path) == expected


def main() -> None:
    manifest = json.loads(MANIFEST.read_text(encoding="utf-8"))
    repository = manifest["repository"]
    revision = manifest["revision"]
    for item in manifest["files"]:
        target = MODEL_DIR / item["name"]
        if valid(target, item["size"], item["sha256"]):
            print(f"embedding asset verified: {item['name']}")
            continue
        partial = target.with_suffix(target.suffix + ".part")
        partial.unlink(missing_ok=True)
        url = f"https://huggingface.co/{repository}/resolve/{revision}/{item['source']}"
        print(f"downloading embedding asset: {item['name']}")
        request = urllib.request.Request(url, headers={"User-Agent": "llm-wiki-build/0.1"})
        with urllib.request.urlopen(request) as response, partial.open("wb") as output:
            shutil.copyfileobj(response, output, length=1024 * 1024)
        if not valid(partial, item["size"], item["sha256"]):
            partial.unlink(missing_ok=True)
            raise RuntimeError(f"Embedding asset verification failed: {item['name']}")
        partial.replace(target)


if __name__ == "__main__":
    main()
