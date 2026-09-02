from pathlib import Path

from fastapi import FastAPI

from llm_wiki.controllers.application import create_http_app
from llm_wiki.services.fast_queue import FastQueueClient
from llm_wiki.services.runtime import build_runtime


def create_app(
    vault_path: Path,
    db_path: Path,
    *,
    fast_queue_client: FastQueueClient | None = None,
) -> FastAPI:
    runtime = build_runtime(vault_path, db_path, fast_queue_client=fast_queue_client)
    return create_http_app(runtime)


__all__ = ["create_app"]
