from __future__ import annotations

import asyncio
import json
import uuid
from collections.abc import AsyncIterator, Callable
from dataclasses import dataclass, field

from llm_wiki.adapters.provider import AsyncOpenAICompatibleProvider

FastOperation = Callable[[asyncio.Event], AsyncIterator[str]]


@dataclass
class FastRequest:
    operation: FastOperation
    id: str = field(default_factory=lambda: str(uuid.uuid4()))
    output: asyncio.Queue[str | BaseException | None] = field(default_factory=asyncio.Queue)
    cancel_event: asyncio.Event = field(default_factory=asyncio.Event)


class FastQueue:
    """One ephemeral FIFO consumer used only for interaction throttling."""

    def __init__(self, *, max_pending: int = 100):
        self._pending: asyncio.Queue[FastRequest | None] = asyncio.Queue(maxsize=max_pending)
        self._consumer: asyncio.Task[None] | None = None

    async def start(self) -> None:
        if self._consumer is None:
            self._consumer = asyncio.create_task(self._consume(), name="llm-wiki-fast-queue")

    async def stop(self) -> None:
        if self._consumer is None:
            return
        await self._pending.put(None)
        await self._consumer
        self._consumer = None

    async def submit(self, operation: FastOperation) -> FastRequest:
        if self._consumer is None:
            await self.start()
        request = FastRequest(operation)
        await self._pending.put(request)
        return request

    async def stream(self, request: FastRequest) -> AsyncIterator[str]:
        try:
            while True:
                value = await request.output.get()
                if value is None:
                    return
                if isinstance(value, BaseException):
                    raise value
                yield value
        finally:
            request.cancel_event.set()

    async def _consume(self) -> None:
        while True:
            request = await self._pending.get()
            if request is None:
                self._pending.task_done()
                return
            try:
                if not request.cancel_event.is_set():
                    async for text in request.operation(request.cancel_event):
                        if request.cancel_event.is_set():
                            break
                        await request.output.put(text)
            except BaseException as error:
                await request.output.put(error)
            finally:
                await request.output.put(None)
                self._pending.task_done()


class FastQueueServer:
    """Loopback stream service with exactly one ephemeral provider consumer."""

    def __init__(self, host: str = "127.0.0.1", port: int = 8766):
        if host not in {"127.0.0.1", "::1", "localhost"}:
            raise ValueError("Fast Queue transport must remain loopback-only")
        self.host = host
        self.port = port
        self.queue = FastQueue()

    async def serve(self) -> None:
        await self.queue.start()
        server = await asyncio.start_server(self._connection, self.host, self.port)
        async with server:
            await server.serve_forever()

    async def _connection(self, reader: asyncio.StreamReader, writer: asyncio.StreamWriter) -> None:
        request: FastRequest | None = None
        try:
            raw = await asyncio.wait_for(reader.readline(), timeout=10)
            payload = json.loads(raw)

            async def operation(cancelled: asyncio.Event) -> AsyncIterator[str]:
                provider = AsyncOpenAICompatibleProvider.with_client(
                    str(payload["base_url"]), str(payload["api_key"]), str(payload["model"])
                )
                try:
                    mode = str(payload.get("mode", "stream"))
                    if mode == "json":
                        result = await provider.complete_json(
                            list(payload["messages"]), str(payload.get("schema_name", "response"))
                        )
                        yield json.dumps(result, ensure_ascii=False)
                        return
                    if mode == "models":
                        yield json.dumps(await provider.models(), ensure_ascii=False)
                        return
                    async for chunk in provider.stream(list(payload["messages"])):
                        if cancelled.is_set():
                            return
                        yield chunk
                finally:
                    await provider.aclose()

            request = await self.queue.submit(operation)
            async for chunk in self.queue.stream(request):
                writer.write((json.dumps({"chunk": chunk}, ensure_ascii=False) + "\n").encode())
                await writer.drain()
            writer.write(b'{"done":true}\n')
            await writer.drain()
        except (BrokenPipeError, ConnectionResetError, asyncio.CancelledError):
            if request:
                request.cancel_event.set()
        except Exception as error:
            try:
                writer.write((json.dumps({"error": str(error)}) + "\n").encode())
                await writer.drain()
            except (BrokenPipeError, ConnectionResetError):
                pass
        finally:
            writer.close()
            await writer.wait_closed()


class FastQueueClient:
    def __init__(self, host: str = "127.0.0.1", port: int = 8766):
        self.host = host
        self.port = port

    async def stream(
        self, *, base_url: str, api_key: str, model: str, messages: list[dict[str, object]]
    ) -> AsyncIterator[str]:
        async for chunk in self._request(
            {"mode": "stream", "base_url": base_url, "api_key": api_key, "model": model, "messages": messages}
        ):
            yield chunk

    async def complete_json(
        self, *, base_url: str, api_key: str, model: str, messages: list[dict[str, object]], schema_name: str
    ) -> dict[str, object]:
        chunks = [
            chunk
            async for chunk in self._request(
                {
                    "mode": "json",
                    "base_url": base_url,
                    "api_key": api_key,
                    "model": model,
                    "messages": messages,
                    "schema_name": schema_name,
                }
            )
        ]
        value = json.loads("".join(chunks))
        if not isinstance(value, dict):
            raise ValueError("Fast Queue JSON result was not an object")
        return value

    async def models(self, *, base_url: str, api_key: str, model: str) -> list[str]:
        chunks = [
            chunk
            async for chunk in self._request(
                {"mode": "models", "base_url": base_url, "api_key": api_key, "model": model, "messages": []}
            )
        ]
        value = json.loads("".join(chunks))
        return [str(item) for item in value]

    async def _request(self, payload: dict[str, object]) -> AsyncIterator[str]:
        reader, writer = await asyncio.open_connection(self.host, self.port)
        writer.write((json.dumps(payload, ensure_ascii=False) + "\n").encode())
        await writer.drain()
        try:
            while raw := await reader.readline():
                event = json.loads(raw)
                if event.get("done"):
                    return
                if event.get("error"):
                    raise OSError(str(event["error"]))
                if "chunk" in event:
                    yield str(event["chunk"])
        finally:
            writer.close()
            await writer.wait_closed()
