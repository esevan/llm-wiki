from pathlib import Path

import pytest

from llm_wiki.services.patches import PatchConflict, apply_reviewed_patch, propose_section_patch
from llm_wiki.services.vault import MarkdownVaultAdapter


def test_reviewed_patch_is_atomic_and_blocks_external_change(tmp_path: Path) -> None:
    note = tmp_path / "decision.md"
    note.write_text("# Decision\n\nKeep this local.\n", encoding="utf-8")
    adapter = MarkdownVaultAdapter(tmp_path)
    patch = propose_section_patch(adapter.read_text("decision.md"), "insert_after_heading", "Decision", "Reviewed context.")
    apply_reviewed_patch(adapter, "decision.md", patch)
    assert "Reviewed context." in adapter.read_text("decision.md")
    changed = propose_section_patch(adapter.read_text("decision.md"), "append_section", "Evidence", "Facts.")
    note.write_text("External edit", encoding="utf-8")
    with pytest.raises(PatchConflict):
        apply_reviewed_patch(adapter, "decision.md", changed)
