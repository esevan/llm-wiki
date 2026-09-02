import sqlite3
from pathlib import Path

from llm_wiki.services.patches import digest
from llm_wiki.services.vault import MarkdownVaultAdapter
from llm_wiki.services.workflow import WorkflowEngine


def test_projection_is_obsidian_markdown_and_mirrored(tmp_path: Path) -> None:
    db = sqlite3.connect(":memory:"); db.row_factory = sqlite3.Row
    workflow = WorkflowEngine(db)
    problem = workflow.promote_capture(workflow.capture("Keep local knowledge clear"))
    path, content = workflow.projection("problems", problem["id"])
    adapter = MarkdownVaultAdapter(tmp_path)
    adapter.atomic_write(path, content)
    workflow.mirror("problems", problem["id"], path, digest(content))
    assert "llm_wiki_id" in adapter.read_text(path)
    assert path.endswith(".md")


def test_projection_uses_english_stored_version_as_canonical() -> None:
    db = sqlite3.connect(":memory:"); db.row_factory = sqlite3.Row
    workflow = WorkflowEngine(db)
    capture = workflow.capture("한글 캡처")
    problem = workflow.promote_capture(
        capture,
        "한글 문제",
        "한글 맥락",
        {
            "ko": {"statement": "한글 문제", "detail": "한글 맥락"},
            "en": {"statement": "English problem", "detail": "English context"},
        },
    )

    path, content = workflow.projection("problems", problem["id"])

    assert path.endswith("English problem.md")
    assert "canonical_locale: en" in content
    assert "# English problem" in content
    assert "English context" in content
    assert "한글 문제" not in content
