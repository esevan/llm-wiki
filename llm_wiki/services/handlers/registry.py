from __future__ import annotations

from collections.abc import Awaitable, Callable
from dataclasses import dataclass
from typing import Any, Protocol

from llm_wiki.services.jobs import TaskDescriptor
from llm_wiki.services.jobs import JobCheckpoint


@dataclass(frozen=True)
class HandlerContext:
    job_id: str
    payload: dict[str, Any]
    source_hash: str
    model: str
    cancelled: Callable[[], Awaitable[bool]]
    progress: Callable[[int, int], Awaitable[None]]
    save_checkpoint: Callable[[str, str, str, int, dict[str, Any]], Awaitable[None]]
    checkpoints: Callable[[str, str], Awaitable[list[JobCheckpoint]]]


class AsyncJobHandler(Protocol):
    async def __call__(self, context: HandlerContext) -> dict[str, Any]: ...


class HandlerRegistry:
    """One authoritative durable handler for every registered task kind."""

    def __init__(self) -> None:
        self._handlers: dict[str, AsyncJobHandler] = {}
        self._descriptors: dict[str, TaskDescriptor] = {}

    def register(self, descriptor: TaskDescriptor, handler: AsyncJobHandler) -> None:
        if descriptor.task_kind in self._handlers:
            raise ValueError(f"Duplicate AI task handler: {descriptor.task_kind}")
        self._descriptors[descriptor.task_kind] = descriptor
        self._handlers[descriptor.task_kind] = handler

    def handler(self, task_kind: str) -> AsyncJobHandler:
        try:
            return self._handlers[task_kind]
        except KeyError as error:
            raise LookupError(f"No AI task handler registered for {task_kind}") from error

    @property
    def task_kinds(self) -> frozenset[str]:
        return frozenset(self._handlers)
