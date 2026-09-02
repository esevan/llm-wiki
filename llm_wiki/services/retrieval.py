from __future__ import annotations

import sqlite3
import struct
import threading
import time
import hashlib
from collections import defaultdict
from pathlib import Path

from llm_wiki.core.models import SearchResult
from llm_wiki.services.markdown import content_hash
from llm_wiki.services.vault import MarkdownVaultAdapter


class RetrievalEngine:
    """Fast structural retrieval. Semantic reranking is intentionally a later lazy layer."""

    def __init__(self, db_path: Path, vault: MarkdownVaultAdapter):
        self.vault = vault
        self.db = sqlite3.connect(db_path, check_same_thread=False)
        self.db.row_factory = sqlite3.Row
        self.db.execute("PRAGMA journal_mode=WAL")
        self._index_lock = threading.Lock()
        self._semantic_lock = threading.Lock()
        self._semantic_embedder: object | None = None
        self._init_schema()

    def _embed(self, texts: list[str]) -> list[list[float]]:
        """Reuse one model session and serialize access to its inference runtime."""
        with self._semantic_lock:
            if self._semantic_embedder is None:
                from llm_wiki.services.semantic import SemanticEmbedder
                self._semantic_embedder = SemanticEmbedder()
            return self._semantic_embedder.embed(texts)  # type: ignore[attr-defined]

    def embed_texts(self, texts: list[str]) -> list[list[float]]:
        """Embedding adapter entry point used only by durable worker handlers."""
        return self._embed(texts)

    def _init_schema(self) -> None:
        self.db.executescript("""
        CREATE TABLE IF NOT EXISTS documents (
          path TEXT PRIMARY KEY, folder TEXT NOT NULL, title TEXT NOT NULL, body TEXT NOT NULL,
          frontmatter TEXT NOT NULL, headings TEXT NOT NULL, tags TEXT NOT NULL, aliases TEXT NOT NULL,
          source_hash TEXT NOT NULL, modified_ns INTEGER NOT NULL
        );
        CREATE VIRTUAL TABLE IF NOT EXISTS documents_fts USING fts5(path UNINDEXED, title, body, headings, tags, aliases);
        CREATE TABLE IF NOT EXISTS document_links (source_path TEXT NOT NULL, target TEXT NOT NULL, anchor TEXT, block_id TEXT, embed INTEGER NOT NULL);
        CREATE TABLE IF NOT EXISTS document_embeddings (path TEXT PRIMARY KEY, source_hash TEXT NOT NULL, dimensions INTEGER NOT NULL, vector BLOB NOT NULL);
        CREATE INDEX IF NOT EXISTS idx_documents_folder ON documents(folder);
        CREATE INDEX IF NOT EXISTS idx_links_target ON document_links(target);
        """)
        self.db.commit()

    def index_changed(self) -> dict[str, int | float]:
        # Filesystem events and an explicit generated-file update may arrive
        # together. SQLite shares this connection, so only one full reindex runs.
        with self._index_lock:
            started = time.perf_counter()
            found: set[str] = set()
            changed = 0
            for file in self.vault.discover():
                path = self.vault.relative_path(file)
                found.add(path)
                modified = file.stat().st_mtime_ns
                row = self.db.execute("SELECT modified_ns FROM documents WHERE path=?", (path,)).fetchone()
                if row and row[0] == modified:
                    continue
                document = self.vault.read(file)
                digest = content_hash(document.content)
                folder = str(Path(path).parent).replace(".", "")
                values = (path, folder, document.title, document.body, str(document.frontmatter), "\n".join(document.headings),
                          " ".join(document.tags), " ".join(document.aliases), digest, modified)
                self.db.execute("INSERT OR REPLACE INTO documents VALUES (?,?,?,?,?,?,?,?,?,?)", values)
                self.db.execute("DELETE FROM documents_fts WHERE path=?", (path,))
                self.db.execute("INSERT INTO documents_fts(path,title,body,headings,tags,aliases) VALUES (?,?,?,?,?,?)",
                                (path, document.title, document.body, "\n".join(document.headings), " ".join(document.tags), " ".join(document.aliases)))
                self.db.execute("DELETE FROM document_links WHERE source_path=?", (path,))
                self.db.executemany("INSERT INTO document_links VALUES (?,?,?,?,?)",
                                    [(path, link.target, link.anchor, link.block_id, int(link.embed)) for link in document.links])
                changed += 1
            rows = self.db.execute("SELECT path FROM documents").fetchall()
            stale = [row[0] for row in rows if row[0] not in found]
            for path in stale:
                self.db.execute("DELETE FROM documents WHERE path=?", (path,))
                self.db.execute("DELETE FROM documents_fts WHERE path=?", (path,))
                self.db.execute("DELETE FROM document_links WHERE source_path=?", (path,))
                self.db.execute("DELETE FROM document_embeddings WHERE path=?", (path,))
            self.db.execute("DELETE FROM document_embeddings WHERE path NOT IN (SELECT path FROM documents)")
            self.db.commit()
            return {"changed": changed, "removed": len(stale), "elapsed_ms": round((time.perf_counter() - started) * 1000, 2)}

    @staticmethod
    def _terms(query: str) -> list[str]:
        return [term for term in query.replace('"', " ").split() if term]

    def search(self, query: str, limit: int = 20, semantic: bool = False, offset: int = 0) -> list[SearchResult]:
        terms = self._terms(query)
        if not terms:
            return []
        # Directory routing uses the path segments first, reducing FTS work in common cases.
        folder_scores: defaultdict[str, int] = defaultdict(int)
        for term in terms:
            for row in self.db.execute("SELECT folder FROM documents WHERE lower(folder) LIKE ?", (f"%{term.lower()}%",)):
                folder_scores[row[0]] += 1
        folders = [folder for folder, _ in sorted(folder_scores.items(), key=lambda item: -item[1])[:5]]
        expression = " OR ".join(f'"{term.replace(chr(34), "")}"' for term in terms)
        sql = """
          SELECT d.*, snippet(documents_fts, 3, '<mark>', '</mark>', '…', 18) snippet,
                 bm25(documents_fts, 5.0, 1.0, 3.0, 3.0, 2.0) rank
          FROM documents_fts JOIN documents d ON d.path=documents_fts.path
          WHERE documents_fts MATCH ?
        """
        params: list[object] = [expression]
        if folders:
            sql += " AND d.folder IN (" + ",".join("?" for _ in folders) + ")"
            params.extend(folders)
        sql += " ORDER BY rank LIMIT ? OFFSET ?"
        params.append(limit)
        params.append(offset)
        rows = self.db.execute(sql, params).fetchall()
        if not rows and folders:  # cross-directory fallback
            rows = self.db.execute(sql.replace(" AND d.folder IN (" + ",".join("?" for _ in folders) + ")", ""), [expression, limit, offset]).fetchall()
        results: list[SearchResult] = []
        for row in rows:
            matched = tuple(field for field in ("path", "title", "headings", "tags", "aliases", "content") if any(term.lower() in str(row[field if field != "content" else "body"]).lower() for term in terms))
            results.append(SearchResult(path=row["path"], title=row["title"], snippet=row["snippet"], headings=tuple(filter(None, row["headings"].split("\n"))), tags=tuple(filter(None, row["tags"].split())), score=round(-float(row["rank"]), 4), source_hash=row["source_hash"], matched_by=matched))
        if semantic and results:
            results = self._semantic_rerank(query, results)
        return results

    def status(self) -> dict[str, int]:
        return {"documents": self.db.execute("SELECT count(*) FROM documents").fetchone()[0], "semantic_ready": self.db.execute("SELECT count(*) FROM document_embeddings").fetchone()[0]}

    def manifest_hash(self) -> str:
        rows = self.db.execute("SELECT path,source_hash FROM documents ORDER BY path").fetchall()
        payload = "\n".join(f"{row['path']}:{row['source_hash']}" for row in rows)
        return hashlib.sha256(payload.encode()).hexdigest()

    def semantic_search(self, query: str, limit: int = 20) -> list[SearchResult]:
        """Search every current embedding, independently of lexical retrieval."""
        groups = self.semantic_search_many([query], limit)
        return groups[0] if groups else []

    def semantic_search_many(self, queries: list[str], limit: int = 20) -> list[list[SearchResult]]:
        """Search embeddings for several queries with one batched model call."""
        from llm_wiki.services.semantic import cosine
        if not queries:
            return []
        rows = self.db.execute("""SELECT d.path,d.title,d.body,d.headings,d.tags,d.source_hash,
          e.dimensions,e.vector FROM documents d JOIN document_embeddings e
          ON e.path=d.path AND e.source_hash=d.source_hash""").fetchall()
        if not rows:
            return [[] for _ in queries]
        vectors = self._embed(queries)
        groups: list[list[SearchResult]] = []
        for query, query_vector in zip(queries, vectors):
            scored = sorted(
                ((cosine(query_vector, struct.unpack(f"<{row['dimensions']}f", row["vector"])), row) for row in rows),
                key=lambda item: item[0], reverse=True,
            )[:limit]
            groups.append([SearchResult(
                path=row["path"], title=row["title"],
                snippet=self._passage(row["path"], row["body"], row["source_hash"], query)["text"],
                headings=tuple(filter(None, row["headings"].split("\n"))), tags=tuple(filter(None, row["tags"].split())),
                score=round(float(score), 4), source_hash=row["source_hash"], matched_by=("semantic",),
            ) for score, row in scored])
        return groups

    def best_passage(self, path: str, query: str, max_chars: int = 1800) -> dict[str, object]:
        row = self.db.execute("SELECT body,source_hash FROM documents WHERE path=?", (path,)).fetchone()
        if not row:
            raise KeyError(path)
        return self._passage(path, str(row["body"]), str(row["source_hash"]), query, max_chars)

    @classmethod
    def _passage(cls, path: str, body: str, source_hash: str, query: str,
                 max_chars: int = 1800) -> dict[str, object]:
        lines = body.splitlines()
        terms = [term.lower() for term in cls._terms(query)]
        best_start, best_score = 0, -1
        for index in range(len(lines)):
            window = "\n".join(lines[index:index + 12])
            score = sum(window.lower().count(term) for term in terms)
            if score > best_score:
                best_start, best_score = index, score
        selected: list[str] = []
        length = 0
        for line in lines[best_start:best_start + 24]:
            if selected and length + len(line) + 1 > max_chars:
                break
            selected.append(line)
            length += len(line) + 1
        if not selected and lines:
            selected = [lines[0][:max_chars]]
        return {"path": path, "source_hash": source_hash, "start_line": best_start + 1,
                "end_line": best_start + len(selected), "text": "\n".join(selected).strip()}

    def _semantic_rerank(self, query: str, results: list[SearchResult]) -> list[SearchResult]:
        from llm_wiki.services.semantic import cosine
        rows = self.db.execute("SELECT path,dimensions,vector FROM document_embeddings WHERE path IN (" + ",".join("?" for _ in results) + ")", [result.path for result in results]).fetchall()
        if not rows:
            return results
        query_vector = self._embed([query])[0]
        scores = {row["path"]: cosine(query_vector, struct.unpack(f"<{row['dimensions']}f", row["vector"])) for row in rows}
        return sorted(results, key=lambda result: scores.get(result.path, -1), reverse=True)
