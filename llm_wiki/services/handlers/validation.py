from __future__ import annotations

from typing import Any

from llm_wiki.services.localization import SUPPORTED_LOCALES


_DRAFT_FIELDS = {
    "captures": ("title", "detail"),
    "problems": ("title", "outcome", "non_goals", "validation_criteria"),
}


def validate_draft(entity_type: str, value: dict[str, Any]) -> dict[str, str]:
    fields = _DRAFT_FIELDS.get(entity_type)
    if not fields:
        raise ValueError(f"Unknown workflow draft: {entity_type}")
    result = {field: str(value.get(field, "")).strip() for field in fields}
    missing = [field for field in fields if not result[field]]
    if missing:
        raise ValueError(f"AI draft is missing required fields: {', '.join(missing)}")
    return result


def validate_bilingual_draft(entity_type: str, value: dict[str, Any]) -> dict[str, dict[str, str]]:
    if set(value) != set(SUPPORTED_LOCALES):
        raise ValueError("AI draft must contain complete Korean and English versions")
    return {locale: validate_draft(entity_type, dict(value[locale])) for locale in SUPPORTED_LOCALES}


def validate_refinement(entity_type: str, value: dict[str, Any]) -> dict[str, str]:
    fields = {"captures": ("title",), "problems": ("title", "detail"), "features": ("title", "detail")}.get(entity_type)
    if not fields:
        raise ValueError(f"Unknown workflow refinement: {entity_type}")
    result = {field: str(value.get(field, "")).strip() for field in fields}
    missing = [field for field in fields if not result[field]]
    if missing:
        raise ValueError(f"AI refinement is missing required fields: {', '.join(missing)}")
    return result


def validate_bilingual_image_summary(value: dict[str, Any]) -> dict[str, dict[str, str]]:
    if set(value) != set(SUPPORTED_LOCALES):
        raise ValueError("Image Summary must contain complete Korean and English versions")
    versions: dict[str, dict[str, str]] = {}
    for locale in SUPPORTED_LOCALES:
        payload = value[locale]
        if not isinstance(payload, dict):
            raise ValueError(f"Image Summary {locale} version must be an object")
        summary = str(payload.get("summary", "")).strip()
        if not summary:
            raise ValueError(f"Image Summary {locale} version cannot be empty")
        versions[locale] = {"image_summary": summary}
    return versions
