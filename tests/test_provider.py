import sqlite3

from llm_wiki.services.provider import OpenAICompatibleProvider
from llm_wiki.services.settings import ProviderSettings


def test_provider_accepts_base_url_with_or_without_v1_suffix() -> None:
    with_suffix = OpenAICompatibleProvider("http://127.0.0.1:8317/v1", "key", "model")
    without_suffix = OpenAICompatibleProvider("http://127.0.0.1:8317", "key", "model")
    assert with_suffix._request("models").full_url == "http://127.0.0.1:8317/v1/models"
    assert without_suffix._request("models").full_url == "http://127.0.0.1:8317/v1/models"


def test_advanced_task_uses_advanced_model_and_other_tasks_use_default() -> None:
    settings = ProviderSettings(sqlite3.connect(":memory:"))
    settings.save("http://127.0.0.1:8317/v1", "default-model", None, "advanced-model", {"capture_assistance": True})
    settings._secret = lambda: "secret"  # type: ignore[method-assign]
    assert settings.credentials("capture_assistance")[2] == "advanced-model"
    assert settings.credentials("problem_enrichment")[2] == "default-model"


def test_discussion_and_refinement_use_the_advanced_model_by_default() -> None:
    settings = ProviderSettings(sqlite3.connect(":memory:"))
    settings.save("http://127.0.0.1:8317/v1", "default-model", None, "advanced-model")
    settings._secret = lambda: "secret"  # type: ignore[method-assign]
    assert settings.credentials("capture_assistance")[2] == "advanced-model"
    assert settings.credentials("problem_assistance")[2] == "advanced-model"
    assert settings.credentials("solution_assistance")[2] == "advanced-model"
    assert settings.credentials("lineage_inference")[2] == "advanced-model"


def test_advanced_task_falls_back_to_default_when_advanced_model_is_blank() -> None:
    settings = ProviderSettings(sqlite3.connect(":memory:"))
    settings._secret = lambda: "secret"  # type: ignore[method-assign]
    settings.save("http://127.0.0.1:8317/v1", "default", None, "", {"image_summary": True})
    assert settings.credentials("image_summary")[2] == "default"
