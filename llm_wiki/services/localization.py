"""Korean-English locale, durable-field, and derived Knowledge primitives.

This module deliberately has no provider, workflow, or vault dependency.  It
owns the small pieces those boundaries share while keeping legacy base rows and
canonical Markdown unchanged until an explicit integration path writes them.
"""

from __future__ import annotations

import json
import hashlib
import re
import sqlite3
from datetime import datetime, timezone
from collections.abc import Mapping, Sequence
from importlib.resources import files

from llm_wiki.services.vault import MarkdownVaultAdapter


SUPPORTED_LOCALES = ("ko", "en")

# Only AI-produced durable fields are required to have paired versions.
# Work Log text remains authored evidence; its explicitly AI-generated image
# summary is localized without making the rest of the progress record mutable.
LOCALIZED_FIELDS: dict[str, tuple[str, ...]] = {
    "captures": ("text",),
    "problems": ("statement", "detail"),
    "features": ("title", "outcome", "non_goals", "validation_criteria"),
    "solution_progress_entries": ("body", "image_summary"),
    "solution_progress_comments": ("body",),
    "solution_checklist_items": ("body",),
}


def normalize_locale(value: object, default: str = "en") -> str:
    """Resolve a browser-style language tag to the two supported locales."""
    fallback = "ko" if str(default).strip().lower().replace("_", "-").split("-", 1)[0] == "ko" else "en"
    language = str(value or "").strip().lower().replace("_", "-").split("-", 1)[0]
    return language if language in SUPPORTED_LOCALES else fallback


def _require_locale(value: object) -> str:
    locale = str(value or "").strip().lower()
    if locale not in SUPPORTED_LOCALES:
        raise ValueError("Unsupported locale; expected 'ko' or 'en'")
    return locale


def response_language_instruction(locale: object) -> str:
    """Return one reusable instruction for live, single-language AI output."""
    resolved = normalize_locale(locale)
    if resolved == "ko":
        return (
            "Respond only in natural Korean for this request. Do not generate a second English version. "
            "Preserve code, identifiers, citations, and quoted evidence verbatim."
        )
    return (
        "Respond only in natural English for this request. Do not generate a second Korean version. "
        "Preserve code, identifiers, citations, and quoted evidence verbatim."
    )


class LocaleSettings:
    """Persist the single local user's explicit application locale."""

    def __init__(self, db: sqlite3.Connection):
        self.db = db
        self.db.execute(
            """CREATE TABLE IF NOT EXISTS locale_settings (
                 id INTEGER PRIMARY KEY CHECK(id=1),
                 locale TEXT NOT NULL DEFAULT 'en' CHECK(locale IN ('ko','en')),
                 explicit INTEGER NOT NULL DEFAULT 0 CHECK(explicit IN (0,1)),
                 updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
               )"""
        )
        self.db.execute(
            "INSERT OR IGNORE INTO locale_settings(id,locale,explicit) VALUES (1,'en',0)"
        )
        self.db.commit()

    @staticmethod
    def _public(locale: str, explicit: bool) -> dict[str, object]:
        return {
            "locale": locale,
            "explicit": explicit,
            "supported_locales": list(SUPPORTED_LOCALES),
        }

    def get(self, browser_locale: object = None) -> dict[str, object]:
        row = self.db.execute(
            "SELECT locale,explicit FROM locale_settings WHERE id=1"
        ).fetchone()
        explicit = bool(row[1])
        locale = str(row[0]) if explicit else normalize_locale(browser_locale)
        return self._public(locale, explicit)

    def save(self, locale: object) -> dict[str, object]:
        # Validate before issuing SQL so a rejected request cannot alter the
        # previously selected locale.
        resolved = _require_locale(locale)
        self.db.execute(
            """UPDATE locale_settings
               SET locale=?,explicit=1,updated_at=CURRENT_TIMESTAMP WHERE id=1""",
            (resolved,),
        )
        self.db.commit()
        return self._public(resolved, True)


def load_locale_resources(locale: object) -> dict[str, str]:
    """Load one packaged flat locale resource without invoking a provider."""
    resolved = _require_locale(locale)
    resource = files("llm_wiki").joinpath("static", "i18n", f"{resolved}.json")
    parsed = json.loads(resource.read_text(encoding="utf-8"))
    if not isinstance(parsed, dict):
        raise ValueError(f"Locale resource {resolved} must be a flat object")
    result = {str(key): value for key, value in parsed.items()}
    _validate_flat_resource(result, resolved)
    return result


def _validate_flat_resource(resource: Mapping[str, object], locale: str) -> None:
    if not resource or any(
        not isinstance(key, str)
        or not key.strip()
        or not isinstance(value, str)
        or not value.strip()
        for key, value in resource.items()
    ):
        raise ValueError(f"Locale resource {locale} keys and values must be non-empty strings")


def validate_resource_parity(
    english: Mapping[str, object] | None = None,
    korean: Mapping[str, object] | None = None,
) -> tuple[str, ...]:
    """Validate paired flat resources and return their stable sorted key set."""
    en = dict(english) if english is not None else load_locale_resources("en")
    ko = dict(korean) if korean is not None else load_locale_resources("ko")
    _validate_flat_resource(en, "en")
    _validate_flat_resource(ko, "ko")
    if set(en) != set(ko):
        raise ValueError("English and Korean locale resources must have identical keys")
    return tuple(sorted(en))


def load_resource_bundle() -> dict[str, dict[str, str]]:
    """Load and parity-check the complete packaged KO/EN resource bundle."""
    bundle = {locale: load_locale_resources(locale) for locale in SUPPORTED_LOCALES}
    validate_resource_parity(bundle["en"], bundle["ko"])
    return bundle


def localize_descriptor(value: object, locale: object) -> object:
    """Translate human-facing descriptor strings without changing machine values."""
    resolved = _require_locale(locale)
    if resolved == "en":
        return value
    bundle = load_resource_bundle()
    reverse = {text: key for key, text in bundle["en"].items()}
    machine_keys = {"id", "value", "name", "type", "source_type", "required", "required_when", "visible_when"}

    def walk(item: object, key: str = "") -> object:
        if isinstance(item, dict):
            return {name: walk(child, str(name)) for name, child in item.items()}
        if isinstance(item, list):
            return [walk(child, key) for child in item]
        if isinstance(item, str) and key not in machine_keys:
            resource_key = reverse.get(item)
            return bundle[resolved].get(resource_key, item) if resource_key else item
        return item

    return walk(value)


class LocalizedContentStore:
    """Field-level sidecar versions that never rewrite legacy base columns."""

    def __init__(self, db: sqlite3.Connection):
        self.db = db
        self.db.execute(
            """CREATE TABLE IF NOT EXISTS localized_content (
                 entity_type TEXT NOT NULL,
                 entity_id TEXT NOT NULL,
                 field_name TEXT NOT NULL,
                 locale TEXT NOT NULL CHECK(locale IN ('ko','en')),
                 value TEXT NOT NULL,
                 origin TEXT NOT NULL CHECK(origin IN ('ai','user')),
                 source_hash TEXT NOT NULL DEFAULT '',
                 created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 PRIMARY KEY(entity_type,entity_id,field_name,locale)
               )"""
        )
        self.db.execute(
            """CREATE INDEX IF NOT EXISTS idx_localized_content_entity
               ON localized_content(entity_type,entity_id,locale)"""
        )
        self.db.commit()

    @staticmethod
    def registered_fields(entity_type: str) -> tuple[str, ...]:
        fields = LOCALIZED_FIELDS.get(entity_type)
        if fields is None:
            raise ValueError(f"Unregistered localized entity type: {entity_type}")
        return fields

    @classmethod
    def _validated_values(
        cls,
        entity_type: str,
        values: Mapping[str, object],
        *,
        complete: bool,
    ) -> dict[str, str]:
        registered = cls.registered_fields(entity_type)
        unknown = set(values) - set(registered)
        if unknown:
            raise ValueError(f"Unregistered localized field: {sorted(unknown)[0]}")
        if complete and set(values) != set(registered):
            missing = ", ".join(field for field in registered if field not in values)
            raise ValueError(f"Localized version is missing registered fields: {missing}")
        if not values:
            raise ValueError("At least one localized field is required")
        if any(not isinstance(value, str) for value in values.values()):
            raise ValueError("Localized field values must be strings")
        return {field: str(values[field]) for field in registered if field in values}

    def save_versions(
        self,
        entity_type: str,
        entity_id: str,
        versions: Mapping[str, Mapping[str, object]],
        *,
        origin: str = "ai",
        source_hash: str = "",
        complete: bool = False,
    ) -> None:
        if origin not in {"ai", "user"}:
            raise ValueError("Localized content origin must be 'ai' or 'user'")
        if not str(entity_id).strip():
            raise ValueError("Localized content requires an entity id")
        # Fully validate the batch before the first UPSERT.  This makes a bad
        # bilingual payload incapable of leaving one accepted locale behind.
        validated: dict[str, dict[str, str]] = {}
        for locale, values in versions.items():
            resolved = _require_locale(locale)
            validated[resolved] = self._validated_values(entity_type, values, complete=complete)
        if not validated:
            raise ValueError("At least one localized version is required")
        rows = [
            (entity_type, entity_id, field, locale, value, origin, source_hash)
            for locale in SUPPORTED_LOCALES
            if locale in validated
            for field, value in validated[locale].items()
        ]
        self.db.executemany(
            """INSERT INTO localized_content(
                 entity_type,entity_id,field_name,locale,value,origin,source_hash
               ) VALUES (?,?,?,?,?,?,?)
               ON CONFLICT(entity_type,entity_id,field_name,locale) DO UPDATE SET
                 value=excluded.value,origin=excluded.origin,
                 source_hash=excluded.source_hash,updated_at=CURRENT_TIMESTAMP""",
            rows,
        )

    def save_bilingual(
        self,
        entity_type: str,
        entity_id: str,
        versions: Mapping[str, Mapping[str, object]],
        *,
        source_hash: str = "",
    ) -> None:
        if set(versions) != set(SUPPORTED_LOCALES):
            raise ValueError("Bilingual content requires complete 'ko' and 'en' versions")
        self.save_versions(
            entity_type,
            entity_id,
            versions,
            origin="ai",
            source_hash=source_hash,
            complete=entity_type != "solution_progress_entries",
        )

    def supplement(
        self,
        entity_type: str,
        entity_id: str,
        locale: object,
        values: Mapping[str, object],
        *,
        source_hash: str = "",
    ) -> None:
        resolved = _require_locale(locale)
        self.save_versions(
            entity_type,
            entity_id,
            {resolved: values},
            origin="user",
            source_hash=source_hash,
        )

    def versions(self, entity_type: str, entity_id: str) -> dict[str, dict[str, str]]:
        self.registered_fields(entity_type)
        rows = self.db.execute(
            """SELECT field_name,locale,value FROM localized_content
               WHERE entity_type=? AND entity_id=?""",
            (entity_type, entity_id),
        ).fetchall()
        grouped: dict[str, dict[str, str]] = {}
        for row in rows:
            grouped.setdefault(str(row[1]), {})[str(row[0])] = str(row[2])
        return {locale: grouped[locale] for locale in SUPPORTED_LOCALES if locale in grouped}

    def overlay(
        self,
        entity_type: str,
        row: Mapping[str, object] | sqlite3.Row,
        locale: object,
    ) -> dict[str, object]:
        base = dict(row)
        entity_id = str(base.get("id") or "")
        if not entity_id:
            raise ValueError("Localized overlay requires an item id")
        return self._overlay_with_versions(entity_type, base, _require_locale(locale), self.versions(entity_type, entity_id))

    def overlay_many(
        self,
        entity_type: str,
        rows: Sequence[Mapping[str, object] | sqlite3.Row],
        locale: object,
    ) -> list[dict[str, object]]:
        registered = self.registered_fields(entity_type)
        resolved = _require_locale(locale)
        base_rows = [dict(row) for row in rows]
        ids = [str(row.get("id") or "") for row in base_rows]
        if any(not entity_id for entity_id in ids):
            raise ValueError("Localized overlay requires an item id")
        grouped: dict[str, dict[str, dict[str, str]]] = {}
        # Keep well below SQLite's cross-platform parameter limit while still
        # using one query per practical board payload instead of one per row.
        for start in range(0, len(ids), 400):
            batch = ids[start : start + 400]
            if not batch:
                continue
            placeholders = ",".join("?" for _ in batch)
            localized = self.db.execute(
                f"""SELECT entity_id,field_name,locale,value FROM localized_content
                    WHERE entity_type=? AND entity_id IN ({placeholders})""",
                (entity_type, *batch),
            ).fetchall()
            for item in localized:
                grouped.setdefault(str(item[0]), {}).setdefault(str(item[2]), {})[str(item[1])] = str(item[3])
        return [
            self._overlay_with_versions(entity_type, row, resolved, grouped.get(entity_id, {}), registered)
            for row, entity_id in zip(base_rows, ids)
        ]

    @classmethod
    def _overlay_with_versions(
        cls,
        entity_type: str,
        base: dict[str, object],
        locale: str,
        versions: Mapping[str, Mapping[str, str]],
        registered: tuple[str, ...] | None = None,
    ) -> dict[str, object]:
        fields = registered or cls.registered_fields(entity_type)
        selected = versions.get(locale, {})
        fallback_used = any(field not in selected for field in fields)
        result = dict(base)
        for field in fields:
            if field in selected:
                result[field] = selected[field]
        result.update(
            {
                "content_locale": "original" if fallback_used else locale,
                "available_locales": [item for item in SUPPORTED_LOCALES if item in versions],
                "fallback_used": fallback_used,
                "localized_versions": {
                    item: dict(versions[item]) for item in SUPPORTED_LOCALES if item in versions
                },
            }
        )
        return result


def knowledge_translation_blocks(markdown: str) -> list[dict[str, object]]:
    """Split Markdown at stable blank-line boundaries without losing a byte of layout."""
    parts = re.split(r"(\n[ \t]*\n+)", markdown)
    blocks: list[dict[str, object]] = []
    for index in range(0, len(parts), 2):
        text = parts[index]
        prefix = "" if index == 0 else parts[index - 1]
        stripped = text.strip()
        frontmatter = not blocks and stripped.startswith("---") and stripped.endswith("---")
        fenced = stripped.startswith(("```", "~~~"))
        blocks.append(
            {
                "prefix": prefix,
                "markdown": text,
                "translatable": bool(stripped) and not frontmatter and not fenced,
            }
        )
    return blocks


class KnowledgeTranslationCache:
    """Hash-validated derived Korean readings; canonical content lives elsewhere."""

    def __init__(self, db: sqlite3.Connection):
        self.db = db
        self.db.execute(
            """CREATE TABLE IF NOT EXISTS knowledge_translation_cache (
                 path TEXT NOT NULL,
                 locale TEXT NOT NULL CHECK(locale='ko'),
                 source_hash TEXT NOT NULL,
                 translated_markdown TEXT NOT NULL,
                 model TEXT NOT NULL DEFAULT '',
                 created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 PRIMARY KEY(path,locale)
               )"""
        )
        self.db.commit()

    @staticmethod
    def _require_derived_locale(locale: object) -> str:
        if _require_locale(locale) != "ko":
            raise ValueError("Knowledge translation cache stores Korean derived readings only")
        return "ko"

    def get(self, path: str, locale: object, source_hash: str) -> dict[str, object] | None:
        resolved = self._require_derived_locale(locale)
        row = self.db.execute(
            """SELECT path,locale,source_hash,translated_markdown,model,created_at,updated_at
               FROM knowledge_translation_cache
               WHERE path=? AND locale=? AND source_hash=?""",
            (path, resolved, source_hash),
        ).fetchone()
        if not row:
            return None
        names = ("path", "locale", "source_hash", "translated_markdown", "model", "created_at", "updated_at")
        return dict(row) if isinstance(row, sqlite3.Row) else dict(zip(names, row))

    def put(
        self,
        path: str,
        locale: object,
        source_hash: str,
        translated_markdown: str,
        model: str = "",
        *,
        current_source_hash: str | None = None,
    ) -> bool:
        resolved = self._require_derived_locale(locale)
        if current_source_hash is not None and current_source_hash != source_hash:
            return False
        if not path or not source_hash or not translated_markdown:
            raise ValueError("Knowledge cache requires path, source hash, and translated Markdown")
        self.db.execute(
            """INSERT INTO knowledge_translation_cache(
                 path,locale,source_hash,translated_markdown,model
               ) VALUES (?,?,?,?,?)
               ON CONFLICT(path,locale) DO UPDATE SET
                 source_hash=excluded.source_hash,
                 translated_markdown=excluded.translated_markdown,
                 model=excluded.model,updated_at=CURRENT_TIMESTAMP""",
            (path, resolved, source_hash, translated_markdown, model),
        )
        self.db.commit()
        return True

    def invalidate(self, path: str) -> int:
        removed = self.db.execute(
            "DELETE FROM knowledge_translation_cache WHERE path=?", (path,)
        ).rowcount
        self.db.commit()
        return removed


class VaultKnowledgeTranslationCache:
    """Atomic, inspectable Korean readings stored below Translations/ko in the Vault."""

    def __init__(
        self,
        vault: MarkdownVaultAdapter,
        legacy: KnowledgeTranslationCache | None = None,
    ) -> None:
        self.vault = vault
        self.legacy = legacy

    @staticmethod
    def _require_korean(locale: object) -> None:
        if _require_locale(locale) != "ko":
            raise ValueError("Knowledge translation cache stores Korean derived readings only")

    @staticmethod
    def _metadata(markdown: str) -> dict[str, str]:
        if not markdown.startswith("---\n"):
            return {}
        end = markdown.find("\n---\n", 4)
        if end < 0:
            return {}
        result: dict[str, str] = {}
        for line in markdown[4:end].splitlines():
            if ":" not in line:
                continue
            key, value = line.split(":", 1)
            raw = value.strip()
            try:
                parsed = json.loads(raw)
            except json.JSONDecodeError:
                parsed = raw
            result[key.strip()] = str(parsed)
        return result

    @staticmethod
    def _without_frontmatter(markdown: str) -> str:
        if not markdown.startswith("---\n"):
            return markdown
        end = markdown.find("\n---\n", 4)
        return markdown[end + 5 :] if end >= 0 else markdown

    def get(self, path: str, locale: object, source_hash: str) -> dict[str, object] | None:
        self._require_korean(locale)
        derived_path = self.vault.korean_translation_path(path)
        try:
            translated = self.vault.read_text(derived_path)
        except FileNotFoundError:
            translated = ""
        metadata = self._metadata(translated)
        if translated and metadata.get("source_hash") == source_hash:
            return {
                "path": path,
                "locale": "ko",
                "source_hash": source_hash,
                "translated_markdown": translated,
                "model": metadata.get("model", ""),
            }
        if not self.legacy:
            return None
        legacy = self.legacy.get(path, "ko", source_hash)
        if not legacy:
            return None
        self.put(
            path,
            "ko",
            source_hash,
            str(legacy["translated_markdown"]),
            str(legacy.get("model", "")),
        )
        self.legacy.invalidate(path)
        return self.get(path, "ko", source_hash)

    def put(
        self,
        path: str,
        locale: object,
        source_hash: str,
        translated_markdown: str,
        model: str = "",
        *,
        current_source_hash: str | None = None,
    ) -> bool:
        self._require_korean(locale)
        if current_source_hash is not None and current_source_hash != source_hash:
            return False
        if not path or not source_hash or not translated_markdown:
            raise ValueError("Knowledge cache requires path, source hash, and translated Markdown")
        canonical_link = path[:-3] if path.endswith(".md") else path
        body = self._without_frontmatter(translated_markdown).lstrip("\n")
        derived = (
            "---\n"
            "llm_wiki_derived: true\n"
            "locale: ko\n"
            f"canonical: {json.dumps(f'[[{canonical_link}]]', ensure_ascii=False)}\n"
            f"source_path: {json.dumps(path, ensure_ascii=False)}\n"
            f"source_hash: {json.dumps(source_hash)}\n"
            f"model: {json.dumps(model, ensure_ascii=False)}\n"
            f"generated_at: {json.dumps(datetime.now(timezone.utc).isoformat())}\n"
            "---\n"
            f"{body}"
        )
        self.vault.atomic_write(self.vault.korean_translation_path(path), derived)
        if self.legacy:
            self.legacy.invalidate(path)
        return True

    def invalidate(self, path: str) -> int:
        derived_path = self.vault.korean_translation_path(path)
        try:
            self.vault.read_text(derived_path)
            existed = 1
        except FileNotFoundError:
            existed = 0
        self.vault.remove(derived_path)
        if self.legacy:
            self.legacy.invalidate(path)
        return existed

    def cleanup(self) -> int:
        removed = 0
        for derived_path in self.vault.discover_korean_translations():
            try:
                translated = self.vault.read_text(derived_path)
            except FileNotFoundError:
                continue
            metadata = self._metadata(translated)
            source_path = metadata.get("source_path", "")
            source_hash = metadata.get("source_hash", "")
            try:
                canonical = self.vault.read_text(source_path) if source_path else ""
            except (OSError, ValueError):
                canonical = ""
            current_hash = hashlib.sha256(canonical.encode()).hexdigest() if canonical else ""
            if not source_path or not source_hash or current_hash != source_hash:
                self.vault.remove(derived_path)
                removed += 1
        return removed
