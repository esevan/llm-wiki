from __future__ import annotations

import asyncio
import json

import httpx

from llm_wiki.adapters.provider import AsyncOpenAICompatibleProvider


def test_async_provider_parses_json_and_streams() -> None:
    def handler(request: httpx.Request) -> httpx.Response:
        payload = json.loads(request.content)
        if payload["stream"]:
            return httpx.Response(200, text='data: {"choices":[{"delta":{"content":"Hi"}}]}\n\ndata: [DONE]\n\n')
        return httpx.Response(200, json={"choices": [{"message": {"content": '{"ok":true}'}}]})

    async def scenario() -> None:
        client = httpx.AsyncClient(base_url="http://provider.test/v1", transport=httpx.MockTransport(handler))
        provider = AsyncOpenAICompatibleProvider.with_client("http://provider.test/v1", "key", "model", client)
        assert await provider.complete_json([], "schema") == {"ok": True}
        assert [part async for part in provider.stream([])] == ["Hi"]
        await provider.aclose()

    asyncio.run(scenario())
