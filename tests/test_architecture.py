from __future__ import annotations

import ast
import re
from pathlib import Path

ROOT = Path(__file__).parents[1]
PACKAGE = ROOT / "llm_wiki"


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
    for path in (PACKAGE / "core").rglob("*.py"):
        assert not any(name.startswith("llm_wiki.") for name in imports(path))
    for path in (ROOT / "llm_wiki" / "repositories").glob("*.py"):
        assert not any(
            name.startswith(("llm_wiki.adapters", "llm_wiki.controllers", "llm_wiki.services", "llm_wiki.web"))
            for name in imports(path)
        )
    for path in (ROOT / "llm_wiki" / "services").rglob("*.py"):
        assert not any(name.startswith(("llm_wiki.controllers", "llm_wiki.web")) for name in imports(path))
    for path in (ROOT / "llm_wiki" / "controllers").glob("*.py"):
        assert not any(
            name.startswith(("llm_wiki.adapters", "llm_wiki.repositories", "llm_wiki.web")) for name in imports(path)
        )


def test_backend_modules_have_no_import_cycles() -> None:
    module_paths = {_module_name(path): path for path in PACKAGE.rglob("*.py")}
    graph = {
        module: {dependency for dependency in imports(path) if dependency in module_paths}
        for module, path in module_paths.items()
    }
    visited: set[str] = set()
    active: list[str] = []

    def visit(module: str) -> None:
        if module in active:
            cycle = active[active.index(module) :] + [module]
            raise AssertionError(" -> ".join(cycle))
        if module in visited:
            return
        active.append(module)
        for dependency in graph[module]:
            visit(dependency)
        active.pop()
        visited.add(module)

    for module in graph:
        visit(module)


def _module_name(path: Path) -> str:
    relative = path.relative_to(ROOT).with_suffix("")
    parts = relative.parts[:-1] if relative.name == "__init__" else relative.parts
    return ".".join(parts)


def test_web_app_factory_composes_controllers() -> None:
    web_source = (ROOT / "llm_wiki" / "web" / "app.py").read_text(encoding="utf-8")
    controller_source = (ROOT / "llm_wiki" / "controllers" / "application.py").read_text(encoding="utf-8")
    assert "from llm_wiki.controllers.application import create_http_app" in web_source
    assert "build_runtime" in web_source
    assert "def create_http_app(runtime: ApplicationRuntime)" in controller_source
    assert not (ROOT / "llm_wiki" / "api" / "app.py").exists()


def test_queue_functions_keep_branching_bounded() -> None:
    paths = list((PACKAGE / "services" / "handlers").glob("*.py"))
    paths.extend(
        (
            PACKAGE / "services" / "job_submission.py",
            PACKAGE / "services" / "runtime.py",
            PACKAGE / "controllers" / "jobs.py",
            PACKAGE / "repositories" / "jobs.py",
        )
    )
    offenders: list[str] = []
    for path in paths:
        tree = ast.parse(path.read_text(encoding="utf-8"))
        for node in ast.walk(tree):
            if not isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef)):
                continue
            branches = sum(_branch_weight(child) for child in ast.walk(node))
            if branches > 10:
                offenders.append(f"{path.relative_to(ROOT)}:{node.lineno} {node.name} ({branches})")
    assert offenders == []


def _branch_weight(node: ast.AST) -> int:
    if isinstance(node, ast.BoolOp):
        return len(node.values) - 1
    return int(isinstance(node, (ast.If, ast.For, ast.AsyncFor, ast.While, ast.Try, ast.Match, ast.IfExp)))


def test_queue_browser_handlers_have_one_authoritative_implementation() -> None:
    source = (ROOT / "llm_wiki" / "static" / "index.html").read_text(encoding="utf-8")
    for name in (
        "loadBoard",
        "runConflictReview",
        "reviewCompletion",
        "startBackgroundRefinement",
        "draftWithAI",
        "detachKnowledgeTranslationReader",
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
        "llm_wiki/services/jobs.py",
        "llm_wiki/services/handlers/workflow.py",
        "llm_wiki/services/handlers/localization.py",
    )
    assert not [path for value in removed if (path := ROOT / value).exists()]
