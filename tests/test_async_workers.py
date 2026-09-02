from __future__ import annotations

import asyncio
from pathlib import Path

import httpx

from llm_wiki.core.jobs import JobStatus, TaskDescriptor
from llm_wiki.repositories.jobs import JobRepository
from llm_wiki.services.handlers.registry import HandlerRegistry
from llm_wiki.services.handlers.worker import AsyncJobWorker


def test_worker_executes_registered_handler(tmp_path: Path) -> None:
    async def scenario() -> None:
        repository = JobRepository(tmp_path / "worker.sqlite")
        await repository.initialize()
        registry = HandlerRegistry()

        async def handler(context):
            return {"echo": context.payload["value"]}

        descriptor = TaskDescriptor("echo", "tests", "one")
        registry.register(descriptor, handler)
        job = await repository.create(descriptor, {"value": 42})
        assert await AsyncJobWorker(repository, registry).run_once() is True
        completed = await repository.get(job.id)
        assert completed is not None
        assert completed.status is JobStatus.COMPLETED
        assert completed.result == {"echo": 42}

    asyncio.run(scenario())


def test_registry_rejects_duplicate_implementation() -> None:
    registry = HandlerRegistry()

    async def handler(_context):
        return {}

    descriptor = TaskDescriptor("only_once")
    registry.register(descriptor, handler)
    try:
        registry.register(descriptor, handler)
    except ValueError as error:
        assert "Duplicate" in str(error)
    else:
        raise AssertionError("duplicate durable handler was accepted")


def test_worker_retries_transient_provider_failures_without_leaking_response(tmp_path: Path) -> None:
    async def scenario() -> None:
        repository = JobRepository(tmp_path / "retry.sqlite")
        await repository.initialize()
        registry = HandlerRegistry()

        async def handler(_context):
            request = httpx.Request("POST", "http://provider.invalid/chat")
            response = httpx.Response(503, request=request, text="private provider response")
            raise httpx.HTTPStatusError("unavailable", request=request, response=response)

        descriptor = TaskDescriptor("transient")
        registry.register(descriptor, handler)
        job = await repository.create(descriptor, {})
        assert await AsyncJobWorker(repository, registry).run_once()
        retried = await repository.get(job.id)
        assert retried is not None
        assert retried.status is JobStatus.RETRYABLE
        assert retried.error_code == "http_503"
        assert "private provider response" not in retried.error_message

    asyncio.run(scenario())
