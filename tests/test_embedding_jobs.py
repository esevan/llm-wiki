from __future__ import annotations

import asyncio
import struct
from pathlib import Path

from llm_wiki.repositories.jobs import JobRepository
from llm_wiki.services.handlers.embeddings import EmbeddingJobHandler
from llm_wiki.services.handlers.registry import HandlerRegistry
from llm_wiki.services.handlers.worker import AsyncJobWorker
from llm_wiki.services.jobs import JobStatus, TaskDescriptor
from llm_wiki.services.retrieval import RetrievalEngine
from llm_wiki.services.vault import MarkdownVaultAdapter


def test_embedding_refresh_is_durable_checkpointed_and_keeps_lexical_search_available(tmp_path: Path, monkeypatch) -> None:
    async def scenario() -> None:
        vault_path = tmp_path / "vault"
        vault_path.mkdir()
        note = vault_path / "decision.md"
        note.write_text("# Decision\nKeep data local", encoding="utf-8")
        db_path = tmp_path / "index.sqlite"
        engine = RetrievalEngine(db_path, MarkdownVaultAdapter(vault_path))
        engine.index_changed()
        constructed = 0

        class FakeEmbedder:
            def __init__(self) -> None:
                nonlocal constructed
                constructed += 1

            def embed(self, texts: list[str]) -> list[list[float]]:
                return [[1.0, 0.0] for _ in texts]

        monkeypatch.setattr("llm_wiki.services.semantic.SemanticEmbedder", FakeEmbedder)
        repository = JobRepository(db_path)
        await repository.initialize()
        descriptor = TaskDescriptor("embedding_refresh", "vault", "documents", "embedding_coverage")
        job = await repository.create(descriptor, {"manifest_hash": engine.manifest_hash()}, source_hash=engine.manifest_hash(), model="local-semantic-embedder")
        registry = HandlerRegistry()
        EmbeddingJobHandler(engine).register(registry)
        assert engine.search("local")[0].path == "decision.md"
        assert await AsyncJobWorker(repository, registry).run_job(job.id)
        completed = await repository.get(job.id)
        assert completed is not None and completed.status is JobStatus.COMPLETED
        assert completed.result["coverage"] == {"documents": 1, "semantic_ready": 1}
        assert engine.semantic_search("local")[0].path == "decision.md"
        row = engine.db.execute("SELECT dimensions,vector FROM document_embeddings").fetchone()
        assert tuple(struct.unpack(f"<{row['dimensions']}f", row["vector"])) == (1.0, 0.0)
        assert constructed == 1

    asyncio.run(scenario())
