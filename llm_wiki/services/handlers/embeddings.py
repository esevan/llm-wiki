from __future__ import annotations

import asyncio
import struct
from typing import Any

from llm_wiki.core.jobs import TaskDescriptor
from llm_wiki.services.handlers.registry import HandlerContext, HandlerRegistry
from llm_wiki.services.retrieval import RetrievalEngine
from llm_wiki.services.semantic import SemanticUnavailable


class EmbeddingJobHandler:
    def __init__(self, retrieval: RetrievalEngine):
        self.retrieval = retrieval

    def register(self, registry: HandlerRegistry) -> None:
        registry.register(TaskDescriptor("embedding_refresh", result_interface="embedding_coverage"), self.__call__)

    async def __call__(self, context: HandlerContext) -> dict[str, Any]:
        rows = self.retrieval.db.execute(
            """SELECT d.path,d.source_hash,d.title || '\n' || d.headings || '\n' || substr(d.body,1,4000) text
               FROM documents d LEFT JOIN document_embeddings e ON e.path=d.path
               WHERE e.source_hash IS NULL OR e.source_hash != d.source_hash ORDER BY d.path"""
        ).fetchall()
        model = context.model or "local-semantic-embedder"
        saved = {item.unit_key: item for item in await context.checkpoints(context.source_hash, model)}
        completed = 0
        for ordinal, row in enumerate(rows):
            if await context.cancelled():
                raise InterruptedError("Embedding refresh cancelled")
            path, source_hash = str(row["path"]), str(row["source_hash"])
            checkpoint = saved.get(path)
            current = self.retrieval.db.execute("SELECT source_hash FROM documents WHERE path=?", (path,)).fetchone()
            if not current or str(current[0]) != source_hash:
                continue
            if not checkpoint or checkpoint.result.get("source_hash") != source_hash:
                try:
                    vector = (await asyncio.to_thread(self.retrieval.embed_texts, [str(row["text"])]))[0]
                except SemanticUnavailable:
                    return {
                        "updated": completed,
                        "coverage": self.retrieval.status(),
                        "semantic_available": False,
                    }
                current = self.retrieval.db.execute(
                    "SELECT source_hash FROM documents WHERE path=?", (path,)
                ).fetchone()
                if not current or str(current[0]) != source_hash:
                    continue
                self.retrieval.db.execute(
                    "INSERT OR REPLACE INTO document_embeddings(path,source_hash,dimensions,vector) VALUES (?,?,?,?)",
                    (path, source_hash, len(vector), struct.pack(f"<{len(vector)}f", *vector)),
                )
                self.retrieval.db.commit()
                await context.save_checkpoint(path, context.source_hash, model, ordinal, {"source_hash": source_hash})
            completed += 1
            await context.progress(completed, len(rows))
        return {"updated": completed, "coverage": self.retrieval.status(), "semantic_available": True}
