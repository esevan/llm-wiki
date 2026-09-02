from __future__ import annotations

import ast
from pathlib import Path


ROOT = Path(__file__).parents[1]
EXPECTED_DURABLE_TASKS = {
    "workflow_draft",
    "workflow_refinement",
    "image_summary",
    "completion_review",
    "knowledge_translation",
    "derived_translation",
    "embedding_refresh",
    "conflict_review",
    "workbench_organization",
    "lineage_inference",
    "completion_report",
}


def test_every_durable_task_has_one_registry_descriptor() -> None:
    counts: dict[str, int] = {}
    for path in (ROOT / "llm_wiki" / "services" / "handlers").glob("*.py"):
        tree = ast.parse(path.read_text(encoding="utf-8"))
        for node in ast.walk(tree):
            if not isinstance(node, ast.Call):
                continue
            function = node.func
            if not (isinstance(function, ast.Name) and function.id == "TaskDescriptor"):
                continue
            if node.args and isinstance(node.args[0], ast.Constant) and isinstance(node.args[0].value, str):
                task = node.args[0].value
                counts[task] = counts.get(task, 0) + 1
    assert set(counts) == EXPECTED_DURABLE_TASKS
    assert all(count == 1 for count in counts.values())


def test_provider_io_is_confined_to_adapter_backed_queue_services() -> None:
    allowed = {"llm_wiki/adapters/provider.py", "llm_wiki/services/fast_queue.py"}
    allowed.update(str(path.relative_to(ROOT)) for path in (ROOT / "llm_wiki" / "services" / "handlers").glob("*.py"))
    offenders: list[str] = []
    for path in (ROOT / "llm_wiki").rglob("*.py"):
        relative = str(path.relative_to(ROOT))
        source = path.read_text(encoding="utf-8")
        if "AsyncOpenAICompatibleProvider" in source and relative not in allowed:
            offenders.append(relative)
    assert offenders == []
