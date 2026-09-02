from __future__ import annotations

import asyncio
from pathlib import Path

from fastapi.testclient import TestClient

from llm_wiki.core.jobs import JobStatus
from llm_wiki.services.handlers.catalog import register_workflow_handlers
from llm_wiki.services.handlers.registry import HandlerRegistry
from llm_wiki.services.handlers.worker import AsyncJobWorker
from llm_wiki.web.app import create_app
from tests.fakes.ai_provider import AsyncJSONProvider


def test_draft_contract_is_202_then_preserves_the_characterized_result(tmp_path: Path, monkeypatch) -> None:
    app = create_app(tmp_path, tmp_path / "compat.sqlite")
    app.state.provider_settings.save("http://provider.test/v1", "model", None)
    monkeypatch.setattr(app.state.provider_settings, "_secret", lambda: "key")
    provider = AsyncJSONProvider(
        {"ko": {"title": "문제", "detail": "맥락"}, "en": {"title": "Problem", "detail": "Context"}}
    )

    with TestClient(app) as client:
        capture = client.post("/api/captures", json={"text": "원문"}).json()
        response = client.post(f"/api/captures/{capture['id']}/draft", headers={"X-LLM-Wiki-Locale": "ko"})
        assert response.status_code == 202
        job_id = response.json()["id"]
        registry = HandlerRegistry()
        register_workflow_handlers(registry, app.state.workflow, app.state.provider_settings, lambda _task: provider)
        assert asyncio.run(AsyncJobWorker(app.state.job_repository, registry).run_job(job_id))
        assert client.get(f"/api/jobs/{job_id}/result").json()["result"]["title"] == "문제"

    job = asyncio.run(app.state.job_repository.get(job_id))
    assert job is not None and job.status is JobStatus.COMPLETED
    assert job.execution_mode == "asynchronous"


def test_failed_async_draft_records_safe_terminal_error(tmp_path: Path, monkeypatch) -> None:
    class FailingProvider(AsyncJSONProvider):
        async def complete_json(self, _messages, _schema):
            raise OSError("offline")

    app = create_app(tmp_path, tmp_path / "failed.sqlite")
    app.state.provider_settings.save("http://provider.test/v1", "model", None)
    monkeypatch.setattr(app.state.provider_settings, "_secret", lambda: "key")
    with TestClient(app) as client:
        capture = client.post("/api/captures", json={"text": "original"}).json()
        response = client.post(f"/api/captures/{capture['id']}/draft")
        job_id = response.json()["id"]
        registry = HandlerRegistry()
        register_workflow_handlers(
            registry,
            app.state.workflow,
            app.state.provider_settings,
            lambda _task: FailingProvider({}),
        )
        asyncio.run(AsyncJobWorker(app.state.job_repository, registry).run_job(job_id))

    job = asyncio.run(app.state.job_repository.get(job_id))
    assert job is not None and job.status is JobStatus.FAILED
    assert job.error_code == "OSError"
