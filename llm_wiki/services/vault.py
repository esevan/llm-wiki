from __future__ import annotations

from pathlib import Path, PurePosixPath
import os
import tempfile
import time

from llm_wiki.core.models import ParsedDocument
from llm_wiki.services.markdown import parse_markdown


class MarkdownVaultAdapter:
    """The only component permitted to address vault paths."""

    def __init__(self, root: Path):
        self.root = root.expanduser().resolve()

    def discover(self) -> list[Path]:
        if not self.root.exists():
            return []
        return sorted(
            p
            for p in self.root.rglob("*.md")
            if p.is_file()
            and ".obsidian" not in p.parts
            and not self.is_korean_translation_path(self.relative_path(p))
        )

    @staticmethod
    def is_korean_translation_path(relative_path: str) -> bool:
        parts = PurePosixPath(relative_path).parts
        return len(parts) >= 2 and parts[:2] == ("Translations", "ko")

    @staticmethod
    def korean_translation_path(canonical_path: str) -> str:
        candidate = PurePosixPath(canonical_path)
        if candidate.is_absolute() or not candidate.parts or ".." in candidate.parts:
            raise ValueError("Canonical path must stay inside the vault")
        if candidate.parts[0] == "Translations":
            raise ValueError("A derived translation cannot be its own canonical source")
        return (PurePosixPath("Translations") / "ko" / candidate).as_posix()

    def discover_korean_translations(self) -> list[str]:
        root = self.root / "Translations" / "ko"
        if not root.exists():
            return []
        return sorted(self.relative_path(path) for path in root.rglob("*.md") if path.is_file())

    def relative_path(self, path: Path) -> str:
        return path.relative_to(self.root).as_posix()

    def read(self, path: Path) -> ParsedDocument:
        return parse_markdown(self.relative_path(path), path.read_text(encoding="utf-8"))

    def read_text(self, relative_path: str) -> str:
        target = (self.root / relative_path).resolve()
        if target != self.root and self.root not in target.parents:
            raise ValueError("Path must stay inside the vault")
        return target.read_text(encoding="utf-8")

    def atomic_write(self, relative_path: str, content: str) -> None:
        self.atomic_write_bytes(relative_path, content.encode("utf-8"))

    def atomic_write_bytes(self, relative_path: str, content: bytes) -> None:
        target = self.root / relative_path
        target.parent.mkdir(parents=True, exist_ok=True)
        descriptor, temporary = tempfile.mkstemp(prefix=f".{target.name}.", suffix=".tmp", dir=target.parent)
        try:
            with os.fdopen(descriptor, "wb") as handle:
                handle.write(content)
                handle.flush()
                os.fsync(handle.fileno())
            for attempt in range(3):
                try:
                    os.replace(temporary, target)
                    break
                except PermissionError:
                    if attempt == 2:
                        raise
                    time.sleep(.05 * (attempt + 1))  # bounded Windows sharing-violation retry
        finally:
            if os.path.exists(temporary):
                os.unlink(temporary)

    def move(self, source_relative: str, destination_relative: str) -> None:
        source = self.root / source_relative
        destination = self.root / destination_relative
        destination.parent.mkdir(parents=True, exist_ok=True)
        os.replace(source, destination)

    def remove(self, relative_path: str) -> None:
        """Remove one vault-relative generated file after an explicit UI confirmation."""
        target = (self.root / relative_path).resolve()
        if self.root.resolve() not in target.parents:
            raise ValueError("Path must stay inside the vault")
        try:
            target.unlink()
        except FileNotFoundError:
            return
