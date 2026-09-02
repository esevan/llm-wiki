"""Lazy, standard OpenAI-compatible provider boundary.

Nothing outside this module knows endpoint details. It intentionally uses the standard library so
normal local use does not load a provider SDK, LangChain, or LangGraph.
"""
from __future__ import annotations

import json
import urllib.request
import threading
from collections.abc import Iterator
from dataclasses import dataclass


@dataclass(frozen=True)
class OpenAICompatibleProvider:
    base_url: str
    api_key: str
    model: str

    @property
    def api_base(self) -> str:
        base = self.base_url.rstrip("/")
        return base[:-3] if base.endswith("/v1") else base

    def _request(self, path: str, payload: dict[str, object] | None = None) -> urllib.request.Request:
        data = json.dumps(payload).encode() if payload is not None else None
        return urllib.request.Request(
            f"{self.api_base}/v1/{path.lstrip('/')}", data=data,
            headers={"Authorization": f"Bearer {self.api_key}", "Content-Type": "application/json"}, method="POST" if data else "GET",
        )

    def models(self) -> list[str]:
        with urllib.request.urlopen(self._request("models"), timeout=10) as response:  # noqa: S310 - configured local endpoint
            body = json.load(response)
        return [str(item["id"]) for item in body.get("data", [])]

    def complete_json(self, messages: list[dict[str, object]], schema_name: str, cancel_event: threading.Event | None = None) -> dict[str, object]:
        if cancel_event and cancel_event.is_set():
            raise InterruptedError(f"{schema_name} cancelled")
        payload = {"model": self.model, "messages": messages, "response_format": {"type": "json_object"}, "stream": False}
        with urllib.request.urlopen(self._request("chat/completions", payload), timeout=90) as response:  # noqa: S310
            finished = threading.Event()
            def close_on_cancel() -> None:
                while not finished.wait(0.05):
                    if cancel_event and cancel_event.is_set():
                        response.close()
                        return
            closer = threading.Thread(target=close_on_cancel, name="llm-wiki-provider-cancel", daemon=True)
            if cancel_event:
                closer.start()
            try:
                body = json.load(response)
            except Exception as error:
                if cancel_event and cancel_event.is_set():
                    raise InterruptedError(f"{schema_name} cancelled") from error
                raise
            finally:
                finished.set()
        if cancel_event and cancel_event.is_set():
            raise InterruptedError(f"{schema_name} cancelled")
        content = body["choices"][0]["message"]["content"]
        try:
            parsed = json.loads(content)
        except (TypeError, json.JSONDecodeError) as error:
            raise ValueError(f"{schema_name} provider output was not JSON") from error
        if not isinstance(parsed, dict):
            raise ValueError(f"{schema_name} provider output was not an object")
        return parsed

    def stream(self, messages: list[dict[str, str]]) -> Iterator[str]:
        payload = {"model": self.model, "messages": messages, "stream": True}
        with urllib.request.urlopen(self._request("chat/completions", payload), timeout=90) as response:  # noqa: S310
            for raw in response:
                if not raw.startswith(b"data: "):
                    continue
                data = raw[6:].strip()
                if data == b"[DONE]":
                    return
                choice = json.loads(data).get("choices", [{}])[0]
                text = choice.get("delta", {}).get("content")
                if text:
                    yield str(text)
