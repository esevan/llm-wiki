from __future__ import annotations

import hashlib
from dataclasses import dataclass

from llm_wiki.services.vault import MarkdownVaultAdapter


class PatchConflict(ValueError):
    pass


@dataclass(frozen=True)
class SectionPatch:
    operation: str
    heading: str
    content: str
    base_hash: str
    before: str
    proposed: str


def digest(text: str) -> str:
    return hashlib.sha256(text.encode()).hexdigest()


def propose_section_patch(current: str, operation: str, heading: str, content: str) -> SectionPatch:
    if operation not in {"append_section", "replace_section", "insert_after_heading"}:
        raise ValueError("Unsupported structured patch operation")
    marker = f"# {heading}" if heading else ""
    if operation == "append_section":
        proposed = current.rstrip() + f"\n\n# {heading}\n\n{content.rstrip()}\n"
    else:
        index = current.find(marker)
        if index < 0:
            raise ValueError(f"Heading not found: {heading}")
        next_heading = current.find("\n# ", index + len(marker))
        end = len(current) if next_heading < 0 else next_heading
        if operation == "replace_section":
            proposed = current[:index] + f"# {heading}\n\n{content.rstrip()}\n" + current[end:]
        else:
            line_end = current.find("\n", index)
            proposed = current[: line_end + 1] + f"\n{content.rstrip()}\n" + current[line_end + 1 :]
    return SectionPatch(operation, heading, content, digest(current), current, proposed)


def apply_reviewed_patch(adapter: MarkdownVaultAdapter, path: str, patch: SectionPatch) -> str:
    """Re-read before adapter-owned atomic write; block rather than overwrite changed context."""
    current = adapter.read_text(path)
    if digest(current) != patch.base_hash:
        raise PatchConflict("The source document changed after review; create a new patch")
    adapter.atomic_write(path, patch.proposed)
    return digest(patch.proposed)
