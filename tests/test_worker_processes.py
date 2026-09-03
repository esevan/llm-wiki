from __future__ import annotations

import asyncio
from pathlib import Path

import pytest

from llm_wiki.services.job_runtime import run_async_workers


@pytest.mark.parametrize("count", [0, 33])
def test_durable_worker_count_has_safe_process_bounds(tmp_path: Path, count: int) -> None:
    with pytest.raises(ValueError, match="between one and 32"):
        asyncio.run(run_async_workers(tmp_path, tmp_path / "workers.sqlite", count, asyncio.Event()))


def test_cli_exposes_independent_spawn_safe_roles() -> None:
    source = (Path(__file__).parents[1] / "llm_wiki" / "cli.py").read_text(encoding="utf-8")
    assert 'sub.add_parser("web"' in source
    assert 'sub.add_parser("fast-worker"' in source
    assert 'sub.add_parser("async-worker"' in source
    assert 'multiprocessing.get_context("spawn")' in source
    assert source.count('name="llm-wiki-fast-worker"') == 1
    assert 'name=f"llm-wiki-async-worker-{index + 1}"' in source
    assert "for index in range(worker_count)" in source
    assert "args=(args.vault, db_path, 1)" in source
    assert "for process in processes:" in source
    assert "process.terminate()" in source
    assert "process.join(timeout=5)" in source


def test_each_concurrent_worker_owns_an_isolated_application_runtime() -> None:
    source = (Path(__file__).parents[1] / "llm_wiki" / "services" / "job_runtime.py").read_text(
        encoding="utf-8"
    )
    assert "build_job_registry(vault_path, db_path) for _ in range(worker_count)" in source
    assert "for index, (registry, _retrieval) in enumerate(runtimes)" in source
