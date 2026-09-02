from __future__ import annotations

import json
from collections.abc import AsyncIterator
from dataclasses import dataclass

import httpx


@dataclass
class AsyncOpenAICompatibleProvider:
    base_url: str
    api_key: str
    model: str
    client: httpx.AsyncClient

    @classmethod
    def with_client(cls, base_url: str, api_key: str, model: str, client: httpx.AsyncClient | None = None) -> "AsyncOpenAICompatibleProvider":
        base = base_url.rstrip("/")
        api_base = base[:-3] if base.endswith("/v1") else base
        owned = client or httpx.AsyncClient(base_url=f"{api_base}/v1", headers={"Authorization": f"Bearer {api_key}"}, timeout=90)
        return cls(base_url, api_key, model, owned)

    async def complete_json(self, messages: list[dict[str, object]], schema_name: str) -> dict[str, object]:
        response = await self.client.post(
            "/chat/completions",
            json={"model": self.model, "messages": messages, "response_format": {"type": "json_object"}, "stream": False},
        )
        response.raise_for_status()
        content = response.json()["choices"][0]["message"]["content"]
        try:
            value = json.loads(content)
        except (TypeError, json.JSONDecodeError) as error:
            raise ValueError(f"{schema_name} provider output was not JSON") from error
        if not isinstance(value, dict):
            raise ValueError(f"{schema_name} provider output was not an object")
        return value

    async def stream(self, messages: list[dict[str, object]]) -> AsyncIterator[str]:
        async with self.client.stream(
            "POST",
            "/chat/completions",
            json={"model": self.model, "messages": messages, "stream": True},
        ) as response:
            response.raise_for_status()
            async for line in response.aiter_lines():
                if not line.startswith("data: "):
                    continue
                data = line[6:].strip()
                if data == "[DONE]":
                    return
                text = json.loads(data).get("choices", [{}])[0].get("delta", {}).get("content")
                if text:
                    yield str(text)

    async def models(self) -> list[str]:
        response = await self.client.get("/models")
        response.raise_for_status()
        return [str(item["id"]) for item in response.json().get("data", []) if item.get("id")]

    async def aclose(self) -> None:
        await self.client.aclose()
