"""Repeatable structural-index smoke benchmark for the 1,000-note reference shape."""
from __future__ import annotations

import tempfile
from pathlib import Path

from llm_wiki.services.retrieval import RetrievalEngine
from llm_wiki.services.vault import MarkdownVaultAdapter


def main() -> None:
    with tempfile.TemporaryDirectory() as temp:
        root = Path(temp) / "vault"
        for index in range(1_000):
            folder = root / f"area-{index % 20}"
            folder.mkdir(parents=True, exist_ok=True)
            (folder / f"note-{index}.md").write_text(
                f"---\ntags: [project/{index % 10}]\n---\n# Decision {index}\nLocal searchable context.\n",
                encoding="utf-8",
            )
        engine = RetrievalEngine(Path(temp) / "index.sqlite", MarkdownVaultAdapter(root))
        metrics = engine.index_changed()
        print(f"structural_index_ms={metrics['elapsed_ms']} changed={metrics['changed']}")
        assert metrics["elapsed_ms"] < 3_000, metrics


if __name__ == "__main__":
    main()
