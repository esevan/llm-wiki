from pathlib import Path

from llm_wiki.services.retrieval import RetrievalEngine
from llm_wiki.services.vault import MarkdownVaultAdapter


def test_search_routes_by_directory_then_finds_frontmatter_and_text(tmp_path: Path) -> None:
    vault = tmp_path / "vault"
    (vault / "product").mkdir(parents=True)
    (vault / "product" / "launch.md").write_text(
        "---\ntags: [release]\naliases: launch notes\n---\n# Launch decision\nKeep the release local first."
    )
    (vault / "random.md").write_text("# Other\nrelease word")
    engine = RetrievalEngine(tmp_path / "index.sqlite", MarkdownVaultAdapter(vault))
    assert engine.index_changed()["changed"] == 2
    results = engine.search("product release")
    assert results[0].path == "product/launch.md"
    assert "tags" in results[0].matched_by
    assert engine.index_changed()["changed"] == 0


def test_indexes_one_thousand_notes_and_finds_korean_unicode(tmp_path: Path) -> None:
    vault = tmp_path / "vault"
    vault.mkdir()
    for index in range(1000):
        body = "한국어 배포 기준" if index == 731 else "ordinary reusable note"
        (vault / f"note-{index:04d}.md").write_text(f"# Note {index}\n{body}", encoding="utf-8")

    engine = RetrievalEngine(tmp_path / "index.sqlite", MarkdownVaultAdapter(vault))
    assert engine.index_changed()["changed"] == 1000
    results = engine.search("한국어 배포")
    assert results[0].path == "note-0731.md"


def test_korean_translation_files_are_excluded_from_canonical_index(tmp_path: Path) -> None:
    vault = tmp_path / "vault"
    (vault / "Knowledge").mkdir(parents=True)
    (vault / "Translations/ko/Knowledge").mkdir(parents=True)
    (vault / "Knowledge/result.md").write_text("# Canonical searchable", encoding="utf-8")
    (vault / "Translations/ko/Knowledge/result.md").write_text("# 파생 번역 검색어", encoding="utf-8")
    engine = RetrievalEngine(tmp_path / "index.sqlite", MarkdownVaultAdapter(vault))

    result = engine.index_changed()

    assert result["changed"] == 1
    assert engine.search("파생") == []


def test_passage_has_exact_line_range_and_manifest_changes(tmp_path: Path) -> None:
    vault = tmp_path / "vault"
    vault.mkdir()
    note = vault / "decision.md"
    note.write_text("# Decision\nUnrelated\nKeep all data local\nNo cloud storage\n", encoding="utf-8")
    engine = RetrievalEngine(tmp_path / "index.sqlite", MarkdownVaultAdapter(vault))
    engine.index_changed()
    before = engine.manifest_hash()
    passage = engine.best_passage("decision.md", "cloud storage")
    assert passage["start_line"] <= passage["end_line"]
    assert "cloud storage" in passage["text"]
    note.write_text(note.read_text() + "Changed\n", encoding="utf-8")
    engine.index_changed()
    assert engine.manifest_hash() != before
