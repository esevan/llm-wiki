from __future__ import annotations

from collections.abc import AsyncIterator
from typing import Any

from llm_wiki.services.completion_archive import CompletionArchivePublisher
from llm_wiki.services.handlers.catalog import (
    register_translation_handlers,
    register_workflow_handlers,
)
from llm_wiki.services.handlers.completion_report import CompletionReportHandler
from llm_wiki.services.handlers.lineage import LineageInferenceHandler
from llm_wiki.services.handlers.registry import HandlerRegistry
from llm_wiki.services.handlers.worker import AsyncJobWorker


class AsyncJSONProvider:
    def __init__(self, result: dict[str, Any], *, model: str = "test-model"):
        self.result = result
        self.model = model
        self.messages: list[dict[str, object]] = []
        self.closed = False

    async def complete_json(self, messages: list[dict[str, object]], _schema: str) -> dict[str, Any]:
        self.messages = messages
        return self.result

    async def aclose(self) -> None:
        self.closed = True


class FakeFastQueueClient:
    def __init__(self, *, chunks: list[str] | None = None, result: dict[str, Any] | None = None):
        self.chunks = chunks or []
        self.result = result or {}
        self.messages: list[dict[str, object]] = []

    async def stream(self, *, messages: list[dict[str, object]], **_options: object) -> AsyncIterator[str]:
        self.messages = messages
        for chunk in self.chunks:
            yield chunk

    async def complete_json(self, *, messages: list[dict[str, object]], **_options: object) -> dict[str, Any]:
        self.messages = messages
        return self.result

    async def models(self, **_options: object) -> list[str]:
        return ["test-model"]


class AsyncAdapter:
    def __init__(self, provider: object):
        self.provider = provider
        self.model = "test-model"

    async def complete_json(self, messages, schema):
        return self.provider.complete_json(messages, schema)  # type: ignore[attr-defined]

    async def aclose(self) -> None:
        pass


class FastAdapter:
    def __init__(self, provider: object):
        self.provider = provider

    async def stream(self, *, messages, **_options):
        for chunk in self.provider.stream(messages):  # type: ignore[attr-defined]
            yield chunk

    async def complete_json(self, *, messages, schema_name, **_options):
        return self.provider.complete_json(messages, schema_name)  # type: ignore[attr-defined]

    async def models(self, **_options):
        return ["test-model"]


async def run_workflow_job(app: object, job_id: str, provider: object) -> None:
    registry = HandlerRegistry()
    register_workflow_handlers(
        registry,
        app.state.workflow,  # type: ignore[attr-defined]
        app.state.provider_settings,  # type: ignore[attr-defined]
        lambda _task: AsyncAdapter(provider),
    )
    assert await AsyncJobWorker(app.state.job_repository, registry).run_job(job_id)  # type: ignore[attr-defined]


async def run_localization_job(app: object, job_id: str, provider: object) -> None:
    registry = HandlerRegistry()
    register_translation_handlers(
        registry,
        app.state.workflow,  # type: ignore[attr-defined]
        app.state.provider_settings,  # type: ignore[attr-defined]
        app.state.vault,  # type: ignore[attr-defined]
        lambda _task: AsyncAdapter(provider),
    )
    assert await AsyncJobWorker(app.state.job_repository, registry).run_job(job_id)  # type: ignore[attr-defined]


async def run_completion_report_job(app: object, job_id: str, provider: object) -> None:
    registry = HandlerRegistry()
    publisher = CompletionArchivePublisher(
        app.state.workflow, app.state.retrieval, app.state.vault, app.state.knowledge_cache
    )  # type: ignore[attr-defined]
    handler = CompletionReportHandler(publisher, app.state.provider_settings, lambda: AsyncAdapter(provider))  # type: ignore[attr-defined]
    handler.register(registry)
    assert await AsyncJobWorker(app.state.job_repository, registry).run_job(job_id)  # type: ignore[attr-defined]


async def run_lineage_job(app: object, job_id: str, provider: object) -> None:
    registry = HandlerRegistry()
    handler = LineageInferenceHandler(app.state.workflow, app.state.provider_settings, lambda: AsyncAdapter(provider))  # type: ignore[attr-defined]
    handler.register(registry)
    assert await AsyncJobWorker(app.state.job_repository, registry).run_job(job_id)  # type: ignore[attr-defined]
