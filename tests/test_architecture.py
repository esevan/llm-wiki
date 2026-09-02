from __future__ import annotations

import ast
import re
from pathlib import Path


ROOT = Path(__file__).parents[1]


def imports(path: Path) -> set[str]:
    tree = ast.parse(path.read_text(encoding="utf-8"))
    found: set[str] = set()
    for node in ast.walk(tree):
        if isinstance(node, ast.Import):
            found.update(alias.name for alias in node.names)
        elif isinstance(node, ast.ImportFrom) and node.module:
            found.add(node.module)
    return found


def test_layer_dependencies_point_inward() -> None:
    for path in (ROOT / "llm_wiki" / "repositories").glob("*.py"):
        assert not any(name.startswith(("llm_wiki.controllers", "llm_wiki.web")) for name in imports(path))
    for path in (ROOT / "llm_wiki" / "services").rglob("*.py"):
        assert not any(name.startswith(("llm_wiki.controllers", "llm_wiki.web")) for name in imports(path))
    for path in (ROOT / "llm_wiki" / "controllers").glob("*.py"):
        assert not any(name.startswith("llm_wiki.adapters") for name in imports(path))


def test_web_app_factory_composes_controllers() -> None:
    web_source = (ROOT / "llm_wiki" / "web" / "app.py").read_text(encoding="utf-8")
    assert "from llm_wiki.controllers.application import create_app" in web_source
    assert not (ROOT / "llm_wiki" / "api" / "app.py").exists()


def test_queue_browser_handlers_have_one_authoritative_implementation() -> None:
    source = (ROOT / "llm_wiki" / "static" / "index.html").read_text(encoding="utf-8")
    for name in (
        "loadBoard",
        "runConflictReview",
        "reviewCompletion",
        "startBackgroundRefinement",
        "draftWithAI",
        "cancelKnowledgeTranslation",
        "streamKnowledgeTranslation",
    ):
        declarations = len(re.findall(rf"(?:async\s+)?function\s+{name}\s*\(", source))
        assignments = len(re.findall(rf"(?m)^{name}\s*=\s*(?:async\s+)?function", source))
        assert declarations + assignments == 1, name


def test_superseded_ai_modules_are_removed() -> None:
    removed = (
        "llm_wiki/services/ai.py",
        "llm_wiki/services/provider.py",
        "llm_wiki/services/graphs.py",
        "llm_wiki/services/conflict_review.py",
        "llm_wiki/services/synchronous/job_executor.py",
    )
    assert not [path for value in removed if (path := ROOT / value).exists()]
