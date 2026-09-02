from __future__ import annotations

import sqlite3
import hashlib

import pytest

from llm_wiki.services.localization import (
    KnowledgeTranslationCache,
    LocaleSettings,
    LocalizedContentStore,
    normalize_locale,
    response_language_instruction,
    validate_resource_parity,
)
from llm_wiki.services.vault import MarkdownVaultAdapter


def test_knowledge_paragraphs_preserve_exact_canonical_layout() -> None:
    from llm_wiki.services.localization import knowledge_translation_blocks

    markdown = "---\ncanonical_locale: en\n---\n\n# First heading\n\nFirst paragraph.\n\n```py\nprint('exact')\n```\n\nLast paragraph.\n"
    blocks = knowledge_translation_blocks(markdown)

    assert "".join(block["prefix"] + block["markdown"] for block in blocks) == markdown
    assert [block["translatable"] for block in blocks] == [False, True, True, False, True]


def database() -> sqlite3.Connection:
    connection = sqlite3.connect(":memory:")
    connection.row_factory = sqlite3.Row
    return connection


def test_locale_normalization_and_response_instruction_support_only_korean_and_english() -> None:
    assert normalize_locale("ko-KR") == "ko"
    assert normalize_locale("ko_KR") == "ko"
    assert normalize_locale("en-US") == "en"
    assert normalize_locale("fr-FR") == "en"
    assert normalize_locale(None) == "en"
    assert "Korean" in response_language_instruction("ko")
    assert "English" in response_language_instruction("en")


def test_locale_settings_are_singleton_explicit_and_preserve_prior_value_on_invalid_save() -> None:
    settings = LocaleSettings(database())

    assert settings.get("ko-KR") == {"locale": "ko", "explicit": False, "supported_locales": ["ko", "en"]}
    assert settings.save("en") == {"locale": "en", "explicit": True, "supported_locales": ["ko", "en"]}
    assert settings.get("ko-KR")["locale"] == "en"

    with pytest.raises(ValueError, match="Unsupported locale"):
        settings.save("fr")
    assert settings.get()["locale"] == "en"


def test_resource_parity_requires_identical_nonempty_flat_keys() -> None:
    assert validate_resource_parity({"menu.search": "Search"}, {"menu.search": "검색"}) == ("menu.search",)
    with pytest.raises(ValueError, match="identical keys"):
        validate_resource_parity({"menu.search": "Search"}, {"menu.vault": "Vault"})
    with pytest.raises(ValueError, match="non-empty strings"):
        validate_resource_parity({"menu.search": ""}, {"menu.search": "검색"})


def test_localized_store_overlays_registered_fields_and_preserves_legacy_originals() -> None:
    store = LocalizedContentStore(database())
    legacy_problem = {"id": "problem-1", "statement": "원래 문제", "detail": "원래 맥락", "state": "draft"}

    legacy = store.overlay("problems", legacy_problem, "en")
    assert legacy["statement"] == "원래 문제"
    assert legacy["state"] == "draft"
    assert legacy["content_locale"] == "original"
    assert legacy["available_locales"] == []
    assert legacy["fallback_used"] is True
    assert legacy["localized_versions"] == {}

    store.save_bilingual(
        "problems",
        "problem-1",
        {
            "ko": {"statement": "한글 문제", "detail": "한글 맥락"},
            "en": {"statement": "English problem", "detail": "English context"},
        },
    )
    english = store.overlay("problems", legacy_problem, "en")
    assert english["statement"] == "English problem"
    assert english["detail"] == "English context"
    assert english["state"] == "draft"
    assert english["content_locale"] == "en"
    assert english["available_locales"] == ["ko", "en"]
    assert english["fallback_used"] is False
    assert english["localized_versions"]["ko"]["statement"] == "한글 문제"


def test_localized_store_bulk_overlay_and_manual_partial_supplement_fall_back_per_field() -> None:
    store = LocalizedContentStore(database())
    rows = [
        {"id": "feature-1", "title": "Original one", "outcome": "Outcome one", "non_goals": "None", "validation_criteria": "- [ ] One"},
        {"id": "feature-2", "title": "Original two", "outcome": "Outcome two", "non_goals": "None", "validation_criteria": "- [ ] Two"},
    ]
    store.supplement("features", "feature-1", "ko", {"title": "첫 번째"})

    overlaid = store.overlay_many("features", rows, "ko")

    assert overlaid[0]["title"] == "첫 번째"
    assert overlaid[0]["outcome"] == "Outcome one"
    assert overlaid[0]["fallback_used"] is True
    assert overlaid[0]["available_locales"] == ["ko"]
    assert overlaid[1]["title"] == "Original two"
    assert overlaid[1]["localized_versions"] == {}
    count = store.db.execute("SELECT count(*) FROM localized_content").fetchone()[0]
    assert count == 1


def test_localized_store_rejects_unregistered_fields_without_changing_existing_versions() -> None:
    store = LocalizedContentStore(database())
    store.supplement("problems", "problem-1", "ko", {"statement": "유효함"})

    with pytest.raises(ValueError, match="Unregistered localized field"):
        store.supplement("problems", "problem-1", "en", {"state": "approved"})

    assert store.versions("problems", "problem-1")["ko"]["statement"] == "유효함"
    assert "en" not in store.versions("problems", "problem-1")


def test_localized_store_supports_only_the_generated_image_summary_field_for_progress_entries() -> None:
    store = LocalizedContentStore(database())
    base = {"id": "entry-1", "body": "사용자가 쓴 작업 기록", "image_summary": "기존 요약"}

    store.save_bilingual(
        "solution_progress_entries",
        "entry-1",
        {
            "ko": {"image_summary": "한글 이미지 요약"},
            "en": {"image_summary": "English image summary"},
        },
    )

    english = store.overlay("solution_progress_entries", base, "en")
    assert english["image_summary"] == "English image summary"
    assert english["body"] == "사용자가 쓴 작업 기록"
    assert english["available_locales"] == ["ko", "en"]
    assert english["fallback_used"] is False
    with pytest.raises(ValueError, match="Unregistered localized field"):
        store.supplement("solution_progress_entries", "entry-1", "ko", {"body": "번역 금지"})


def test_knowledge_cache_hits_only_the_exact_path_locale_and_source_hash() -> None:
    cache = KnowledgeTranslationCache(database())
    cache.put("Knowledge/Result.md", "ko", "hash-one", "# 한국어 결과", "test-model")

    hit = cache.get("Knowledge/Result.md", "ko", "hash-one")
    assert hit is not None
    assert hit["translated_markdown"] == "# 한국어 결과"
    assert cache.get("Knowledge/Result.md", "ko", "hash-two") is None
    assert cache.get("Knowledge/Other.md", "ko", "hash-one") is None
    with pytest.raises(ValueError, match="Korean derived readings"):
        cache.get("Knowledge/Result.md", "en", "hash-one")


def test_knowledge_cache_rejects_changed_source_at_commit_and_supports_invalidation() -> None:
    cache = KnowledgeTranslationCache(database())

    assert cache.put(
        "Knowledge/Result.md",
        "ko",
        "old-hash",
        "stale translation",
        "test-model",
        current_source_hash="new-hash",
    ) is False
    assert cache.get("Knowledge/Result.md", "ko", "old-hash") is None
    assert cache.put("Knowledge/Result.md", "ko", "new-hash", "현재 번역", "test-model") is True
    assert cache.invalidate("Knowledge/Result.md") == 1
    assert cache.get("Knowledge/Result.md", "ko", "new-hash") is None


def test_vault_knowledge_cache_writes_linked_derived_markdown_and_validates_hash(tmp_path) -> None:
    from llm_wiki.services.localization import VaultKnowledgeTranslationCache

    vault = MarkdownVaultAdapter(tmp_path)
    canonical_path = "Knowledge/Result.md"
    canonical = "---\nllm_wiki_managed: true\ncanonical_locale: en\n---\n# Result\n"
    vault.atomic_write(canonical_path, canonical)
    source_hash = hashlib.sha256(canonical.encode()).hexdigest()
    cache = VaultKnowledgeTranslationCache(vault)

    assert cache.put(canonical_path, "ko", source_hash, "# 한국어 결과", "test-model") is True
    derived = vault.read_text("Translations/ko/Knowledge/Result.md")
    assert 'canonical: "[[Knowledge/Result]]"' in derived
    assert 'source_path: "Knowledge/Result.md"' in derived
    assert f'source_hash: "{source_hash}"' in derived
    assert "locale: ko" in derived
    assert "# 한국어 결과" in derived
    assert cache.get(canonical_path, "ko", source_hash)["translated_markdown"] == derived
    assert cache.get(canonical_path, "ko", "changed") is None
    assert cache.invalidate(canonical_path) == 1
    assert not (tmp_path / "Translations/ko/Knowledge/Result.md").exists()


def test_vault_knowledge_cache_promotes_matching_legacy_sqlite_entry(tmp_path) -> None:
    from llm_wiki.services.localization import VaultKnowledgeTranslationCache

    legacy = KnowledgeTranslationCache(database())
    legacy.put("Knowledge/Legacy.md", "ko", "same-hash", "# 기존 번역", "old-model")
    cache = VaultKnowledgeTranslationCache(MarkdownVaultAdapter(tmp_path), legacy)

    hit = cache.get("Knowledge/Legacy.md", "ko", "same-hash")

    assert hit is not None
    assert (tmp_path / "Translations/ko/Knowledge/Legacy.md").exists()
    assert legacy.get("Knowledge/Legacy.md", "ko", "same-hash") is None


def test_vault_knowledge_cache_cleans_translation_when_canonical_changes(tmp_path) -> None:
    from llm_wiki.services.localization import VaultKnowledgeTranslationCache

    vault = MarkdownVaultAdapter(tmp_path)
    original = "# Original\n"
    source_hash = hashlib.sha256(original.encode()).hexdigest()
    vault.atomic_write("Knowledge/Changed.md", original)
    cache = VaultKnowledgeTranslationCache(vault)
    cache.put("Knowledge/Changed.md", "ko", source_hash, "# 번역\n")
    vault.atomic_write("Knowledge/Changed.md", "# Changed externally\n")

    assert cache.cleanup() == 1
    assert not (tmp_path / "Translations/ko/Knowledge/Changed.md").exists()
