from __future__ import annotations

import asyncio

import pytest

from llm_wiki.services.fast_queue import FastQueue, FastQueueServer


def test_fast_queue_has_one_consumer_and_no_durable_state() -> None:
    async def scenario() -> None:
        queue = FastQueue()
        active = 0
        maximum = 0

        def operation(label: str):
            async def run(_cancel):
                nonlocal active, maximum
                active += 1
                maximum = max(maximum, active)
                await asyncio.sleep(0.01)
                yield label
                active -= 1
            return run

        first = await queue.submit(operation("first"))
        second = await queue.submit(operation("second"))
        assert [item async for item in queue.stream(first)] == ["first"]
        assert [item async for item in queue.stream(second)] == ["second"]
        assert maximum == 1
        assert not hasattr(queue, "repository")
        await queue.stop()

    asyncio.run(scenario())


def test_closing_fast_interaction_discards_queued_output() -> None:
    async def scenario() -> None:
        queue = FastQueue()

        async def operation(cancel):
            await asyncio.sleep(0.01)
            if not cancel.is_set():
                yield "late"

        request = await queue.submit(operation)
        request.cancel_event.set()
        assert [item async for item in queue.stream(request)] == []
        await queue.stop()

    asyncio.run(scenario())


def test_fast_queue_transport_rejects_non_loopback_binding() -> None:
    with pytest.raises(ValueError, match="loopback-only"):
        FastQueueServer("0.0.0.0")
