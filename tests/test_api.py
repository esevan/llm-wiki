import json
import re
import threading
from pathlib import Path

from fastapi.testclient import TestClient

import llm_wiki.api.app as api_module
from llm_wiki.api.app import create_app


def test_api_is_usable_without_ai_or_embeddings(tmp_path: Path) -> None:
    (tmp_path / "note.md").write_text("# A local note\n#search")
    with TestClient(create_app(tmp_path, tmp_path / "db.sqlite")) as client:
        assert client.get("/api/health").json()["documents"] == 1
        response = client.get("/api/search", params={"q": "local"})
        assert response.status_code == 200
        assert response.json()["results"][0]["path"] == "note.md"
        captured = client.post("/api/captures", json={"text": "A quick problem"})
        assert captured.status_code == 201
        assert captured.json()["id"] != "pending-workflow"


def test_locale_resources_setting_and_localized_board_are_persistent_and_provider_free(tmp_path: Path) -> None:
    db_path = tmp_path / "locale.sqlite"
    app = create_app(tmp_path, db_path)
    with TestClient(app) as client:
        english_response = client.get("/api/i18n/en")
        korean_response = client.get("/api/i18n/ko")
        assert english_response.headers["cache-control"] == "no-store"
        assert korean_response.headers["cache-control"] == "no-store"
        english = english_response.json()
        korean = korean_response.json()
        assert set(english) == set(korean)
        transition = client.get("/api/transitions", headers={"X-LLM-Wiki-Locale": "ko"}).json()["transitions"][0]
        assert transition["label"] == "직접 문제 만들기"
        assert transition["id"] == "capture_to_problem"
        initial_setting = client.get("/api/settings/locale", params={"browser_locale": "ko-KR"})
        assert initial_setting.headers["cache-control"] == "no-store"
        assert initial_setting.json()["locale"] == "ko"
        assert client.put("/api/settings/locale", json={"locale": "en"}).json()["explicit"] is True
        stored = client.app.state.retrieval.db.execute(
            "SELECT locale,explicit FROM locale_settings WHERE id=1"
        ).fetchone()
        assert tuple(stored) == ("en", 1)
        capture = client.post("/api/captures", json={"text": "원문 캡처"}).json()
        versions = {
            "ko": {"statement": "한글 문제", "detail": "한글 맥락"},
            "en": {"statement": "English problem", "detail": "English context"},
        }
        created = client.post(
            f"/api/captures/{capture['id']}/promote",
            json={"statement": "한글 문제", "detail": "한글 맥락", "localized_versions": versions},
        )
        assert created.status_code == 201
        assert client.get("/api/board", headers={"X-LLM-Wiki-Locale": "en"}).json()["problems"][0]["statement"] == "English problem"
        assert client.get("/api/board", headers={"X-LLM-Wiki-Locale": "ko"}).json()["problems"][0]["statement"] == "한글 문제"

    with TestClient(create_app(tmp_path, db_path)) as client:
        restored_setting = client.get("/api/settings/locale", params={"browser_locale": "ko-KR"})
        assert restored_setting.headers["cache-control"] == "no-store"
        assert restored_setting.json()["locale"] == "en"


def test_legacy_content_falls_back_unchanged_and_can_be_manually_supplemented(tmp_path: Path) -> None:
    with TestClient(create_app(tmp_path, tmp_path / "legacy.sqlite")) as client:
        capture = client.post("/api/captures", json={"text": "기존 원문"}).json()
        problem = client.post(f"/api/captures/{capture['id']}/promote", json={}).json()
        legacy = client.get("/api/board", headers={"X-LLM-Wiki-Locale": "en"}).json()["problems"][0]
        assert legacy["statement"] == "기존 원문"
        assert legacy["fallback_used"] is True
        assert legacy["localized_versions"] == {}
        response = client.put(
            f"/api/items/problems/{problem['id']}/localizations",
            json={"locale": "en", "fields": {"statement": "Legacy original", "detail": ""}},
        )
        assert response.status_code == 204
        supplemented = client.get("/api/board", headers={"X-LLM-Wiki-Locale": "en"}).json()["problems"][0]
        assert supplemented["statement"] == "Legacy original"
        assert supplemented["id"] == problem["id"]


def test_image_summary_generates_both_locales_once_and_progress_reads_use_stored_versions(tmp_path: Path, monkeypatch) -> None:
    class SummaryProvider:
        calls = 0
        messages: list[dict[str, object]] = []
        result: dict[str, object] = {
            "ko": {"summary": "화면에 완료 상태가 보입니다."},
            "en": {"summary": "The screen shows the completed state."},
        }

        def __init__(self, *_: str) -> None:
            pass

        def complete_json(self, messages: list[dict[str, object]], _label: str) -> dict[str, object]:
            type(self).calls += 1
            type(self).messages = messages
            return type(self).result

    app = create_app(tmp_path, tmp_path / "image-summary.sqlite")
    app.state.provider_settings.save("http://provider.test/v1", "test-model", None)
    monkeypatch.setattr(app.state.provider_settings, "_secret", lambda: "test-key")
    monkeypatch.setattr(api_module, "OpenAICompatibleProvider", SummaryProvider)
    workflow = app.state.workflow
    problem = workflow.promote_capture(workflow.capture("A problem"))
    workflow.approve_problem(problem["id"])
    feature = workflow.create_feature(problem["id"], "A solution", "An outcome", validation_criteria="- [ ] Done")
    workflow.record_conflict_evaluation(feature["id"], "clear", "Human review")
    workflow.approve_feature(feature["id"])
    entry = workflow.add_solution_progress(feature["id"], "원문 작업 기록", "aGVsbG8=", "image/png")

    with TestClient(app) as client:
        response = client.post(
            f"/api/progress/{entry['id']}/summarize-image",
            headers={"X-LLM-Wiki-Locale": "ko"},
        )
        assert response.status_code == 200
        assert response.json()["summary"] == "화면에 완료 상태가 보입니다."
        assert response.json()["localized_versions"]["en"]["image_summary"] == "The screen shows the completed state."
        assert response.json()["missing_locales"] == []
        assert SummaryProvider.calls == 1
        prompt = str(SummaryProvider.messages[0]["content"][0]["text"])
        assert '"ko"' in prompt and '"en"' in prompt

        korean = client.get(
            f"/api/features/{feature['id']}/progress", headers={"X-LLM-Wiki-Locale": "ko"}
        ).json()["entries"][0]
        english = client.get(
            f"/api/features/{feature['id']}/progress", headers={"X-LLM-Wiki-Locale": "en"}
        ).json()["entries"][0]
        assert korean["image_summary"] == "화면에 완료 상태가 보입니다."
        assert english["image_summary"] == "The screen shows the completed state."
        assert korean["body"] == english["body"] == "원문 작업 기록"
        assert SummaryProvider.calls == 1

        SummaryProvider.result = {"ko": {"summary": "불완전한 응답"}}
        failed = client.post(
            f"/api/progress/{entry['id']}/summarize-image",
            headers={"X-LLM-Wiki-Locale": "ko"},
        )
        assert failed.status_code == 502
        preserved = client.get(
            f"/api/features/{feature['id']}/progress", headers={"X-LLM-Wiki-Locale": "en"}
        ).json()["entries"][0]
        assert preserved["image_summary"] == "The screen shows the completed state."


def test_managed_knowledge_korean_translation_cache_tracks_canonical_hash(tmp_path: Path, monkeypatch) -> None:
    class TranslationProvider:
        calls = 0

        def __init__(self, *_: str) -> None:
            pass

        def complete_json(self, messages, _schema):
            type(self).calls += 1
            return {"markdown": "# 한국어\n\n" + str(messages[-1]["content"])}

    path = "Knowledge/result.md"
    target = tmp_path / path
    target.parent.mkdir(parents=True)
    target.write_text("---\nllm_wiki_managed: true\ncanonical_locale: en\n---\n# Result\n", encoding="utf-8")
    app = create_app(tmp_path, tmp_path / "knowledge.sqlite")
    app.state.provider_settings.save("http://provider.test/v1", "test-model", None)
    monkeypatch.setattr(app.state.provider_settings, "_secret", lambda: "secret")
    monkeypatch.setattr(api_module, "OpenAICompatibleProvider", TranslationProvider)

    with TestClient(app) as client:
        first = client.get("/api/knowledge", params={"path": path, "locale": "ko"}).json()
        second = client.get("/api/knowledge", params={"path": path, "locale": "ko"}).json()
        assert first["cache_status"] == "miss"
        assert second["cache_status"] == "hit"
        assert TranslationProvider.calls == 1
        target.write_text("---\nllm_wiki_managed: true\ncanonical_locale: en\n---\n# Changed\n", encoding="utf-8")
        changed = client.get("/api/knowledge", params={"path": path, "locale": "ko"}).json()
        assert changed["cache_status"] == "miss"
        assert TranslationProvider.calls == 2


def test_progressive_knowledge_read_is_provider_free_then_streams_complete_paragraphs(tmp_path: Path, monkeypatch) -> None:
    class ParagraphProvider:
        calls = 0

        def __init__(self, *_: str) -> None:
            pass

        def complete_json(self, messages, _schema):
            type(self).calls += 1
            source = str(messages[-1]["content"])
            return {"markdown": {"# One": "# 하나", "First paragraph.": "첫 문단."}.get(source, source)}

    path = "Knowledge/progressive.md"
    target = tmp_path / path
    target.parent.mkdir(parents=True)
    target.write_text(
        "---\nllm_wiki_managed: true\ncanonical_locale: en\n---\n\n# One\n\nFirst paragraph.\n",
        encoding="utf-8",
    )
    app = create_app(tmp_path, tmp_path / "progressive.sqlite")
    app.state.provider_settings.save("http://provider.test/v1", "test-model", None)
    monkeypatch.setattr(app.state.provider_settings, "_secret", lambda: "secret")
    monkeypatch.setattr(api_module, "OpenAICompatibleProvider", ParagraphProvider)

    with TestClient(app) as client:
        fast = client.get(
            "/api/knowledge", params={"path": path, "locale": "ko", "translate": "false"}
        ).json()
        assert fast["cache_status"] == "pending"
        assert fast["markdown"] == target.read_text(encoding="utf-8")
        assert ParagraphProvider.calls == 0

        response = client.get(
            "/api/knowledge/translate", params={"path": path, "request_id": "reader-1"}
        )
        events = [json.loads(line) for line in response.text.splitlines()]
        paragraphs = [event for event in events if event["event"] == "paragraph"]
        assert [(item["completed"], item["total"]) for item in paragraphs] == [(1, 2), (2, 2)]
        assert paragraphs[0]["markdown"] == "# 하나"
        assert events[-1]["event"] == "complete"
        assert ParagraphProvider.calls == 2

        cached = client.get(
            "/api/knowledge", params={"path": path, "locale": "ko", "translate": "false"}
        ).json()
        assert cached["cache_status"] == "hit"
        assert "첫 문단." in cached["markdown"]
        derived = tmp_path / "Translations/ko" / path
        assert derived.exists()
        assert 'canonical: "[[Knowledge/progressive]]"' in derived.read_text(encoding="utf-8")


def test_progressive_knowledge_translation_can_be_cancelled_on_server(tmp_path: Path, monkeypatch) -> None:
    class BlockingProvider:
        calls = 0
        started = threading.Event()
        release = threading.Event()

        def __init__(self, *_: str) -> None:
            pass

        def complete_json(self, messages, _schema):
            type(self).calls += 1
            type(self).started.set()
            assert type(self).release.wait(timeout=2)
            return {"markdown": "번역 " + str(messages[-1]["content"])}

    path = "Knowledge/cancel.md"
    target = tmp_path / path
    target.parent.mkdir(parents=True)
    target.write_text(
        "---\nllm_wiki_managed: true\ncanonical_locale: en\n---\n\n# Cancel\n\nSecond paragraph.\n",
        encoding="utf-8",
    )
    app = create_app(tmp_path, tmp_path / "cancel.sqlite")
    app.state.provider_settings.save("http://provider.test/v1", "test-model", None)
    monkeypatch.setattr(app.state.provider_settings, "_secret", lambda: "secret")
    monkeypatch.setattr(api_module, "OpenAICompatibleProvider", BlockingProvider)
    with TestClient(app) as client:
        result: dict[str, object] = {}

        def translate() -> None:
            result["response"] = client.get(
                "/api/knowledge/translate", params={"path": path, "request_id": "reader-cancelled"}
            )

        worker = threading.Thread(target=translate)
        worker.start()
        assert BlockingProvider.started.wait(timeout=2)
        cancelled = client.post(
            "/api/knowledge/translation-cancel", params={"request_id": "reader-cancelled"}
        )
        assert cancelled.status_code == 204
        assert app.state.knowledge_translation_cancelled("reader-cancelled") is True
        BlockingProvider.release.set()
        worker.join(timeout=2)

        assert not worker.is_alive()
        events = [json.loads(line) for line in result["response"].text.splitlines()]
        assert events[-1]["event"] == "cancelled"
        assert BlockingProvider.calls == 1


def test_legacy_vault_knowledge_never_translates_on_korean_read(tmp_path: Path) -> None:
    (tmp_path / "legacy.md").write_text("# 기존 Vault 원문\n", encoding="utf-8")
    with TestClient(create_app(tmp_path, tmp_path / "legacy-vault.sqlite")) as client:
        result = client.get("/api/knowledge", params={"path": "legacy.md", "locale": "ko"}).json()
        assert result["markdown"] == "# 기존 Vault 원문\n"
        assert result["translated"] is False
        assert result["cache_status"] == "not_applicable"


def test_managed_knowledge_falls_back_to_canonical_when_translation_fails(tmp_path: Path, monkeypatch) -> None:
    class FailingProvider:
        def __init__(self, *_: str) -> None:
            pass

        def complete_json(self, *_args, **_kwargs):
            raise OSError("provider unavailable")

    canonical = "---\nllm_wiki_managed: true\ncanonical_locale: en\n---\n# Portable result\n"
    target = tmp_path / "Knowledge" / "failure.md"
    target.parent.mkdir(parents=True)
    target.write_text(canonical, encoding="utf-8")
    app = create_app(tmp_path, tmp_path / "knowledge-failure.sqlite")
    app.state.provider_settings.save("http://provider.test/v1", "test-model", None)
    monkeypatch.setattr(app.state.provider_settings, "_secret", lambda: "secret")
    monkeypatch.setattr(api_module, "OpenAICompatibleProvider", FailingProvider)

    with TestClient(app) as client:
        result = client.get("/api/knowledge", params={"path": "Knowledge/failure.md", "locale": "ko"}).json()

    assert result["markdown"] == canonical
    assert result["cache_status"] == "fallback"
    assert result["warning_code"] == "translation_unavailable"
    assert target.read_text(encoding="utf-8") == canonical


def test_korean_managed_knowledge_patch_is_normalized_before_review_and_apply(tmp_path: Path, monkeypatch) -> None:
    class NormalizingProvider:
        calls = 0

        def __init__(self, *_: str) -> None:
            pass

        def complete_json(self, _messages, schema_name):
            assert schema_name == "knowledge English normalization"
            type(self).calls += 1
            return {"content": "English reviewed addition with `code_id`."}

    target = tmp_path / "Knowledge" / "patch.md"
    target.parent.mkdir(parents=True)
    target.write_text(
        "---\nllm_wiki_managed: true\ncanonical_locale: en\n---\n# Result\n\n## Notes\nOriginal.\n",
        encoding="utf-8",
    )
    app = create_app(tmp_path, tmp_path / "knowledge-patch.sqlite")
    app.state.provider_settings.save("http://provider.test/v1", "test-model", None)
    monkeypatch.setattr(app.state.provider_settings, "_secret", lambda: "secret")
    monkeypatch.setattr(api_module, "OpenAICompatibleProvider", NormalizingProvider)

    with TestClient(app) as client:
        proposal = client.post(
            "/api/features/feature-1/patches",
            headers={"X-LLM-Wiki-Locale": "ko"},
            json={"path": "Knowledge/patch.md", "operation": "insert_after_heading", "heading": "Notes", "content": "한국어 보강 `code_id`."},
        )
        assert proposal.status_code == 201
        patch = proposal.json()
        assert "English reviewed addition" in patch["proposed_text"]
        assert "한국어 보강" not in patch["proposed_text"]
        assert client.post(f"/api/patches/{patch['id']}/apply").status_code == 204

    assert "English reviewed addition with `code_id`." in target.read_text(encoding="utf-8")
    assert NormalizingProvider.calls == 1


def test_bilingual_problem_refinement_updates_both_stored_versions(tmp_path: Path, monkeypatch) -> None:
    class RefinementProvider:
        calls = 0

        def __init__(self, *_: str) -> None:
            pass

        def complete_json(self, _messages, schema_name):
            assert schema_name == "problems refinement"
            type(self).calls += 1
            return {
                "ko": {"title": "정제된 문제", "detail": "정제된 맥락"},
                "en": {"title": "Refined problem", "detail": "Refined context"},
            }

    app = create_app(tmp_path, tmp_path / "bilingual-refinement.sqlite")
    app.state.provider_settings.save("http://provider.test/v1", "test-model", None)
    monkeypatch.setattr(app.state.provider_settings, "_secret", lambda: "secret")
    monkeypatch.setattr(api_module, "OpenAICompatibleProvider", RefinementProvider)
    with TestClient(app) as client:
        capture = client.post("/api/captures", json={"text": "기존 문제"}).json()
        problem = client.post(f"/api/captures/{capture['id']}/promote", json={}).json()
        draft = client.post(
            f"/api/problems/{problem['id']}/refine",
            headers={"X-LLM-Wiki-Locale": "ko"},
        ).json()
        assert set(draft["localized_versions"]) == {"ko", "en"}
        response = client.put(
            f"/api/items/problems/{problem['id']}",
            json={"title": draft["title"], "detail": draft["detail"], "localized_versions": draft["localized_versions"]},
        )
        assert response.status_code == 204
        korean = client.get("/api/board", headers={"X-LLM-Wiki-Locale": "ko"}).json()["problems"][0]
        english = client.get("/api/board", headers={"X-LLM-Wiki-Locale": "en"}).json()["problems"][0]

    assert korean["statement"] == "정제된 문제"
    assert english["statement"] == "Refined problem"
    assert RefinementProvider.calls == 1


def test_api_projects_an_approved_problem(tmp_path: Path) -> None:
    with TestClient(create_app(tmp_path, tmp_path / "db.sqlite")) as client:
        capture = client.post("/api/captures", json={"text": "Make reuse easier"}).json()
        problem = client.post(f"/api/captures/{capture['id']}/promote", json={}).json()
        assert client.post(f"/api/problems/{problem['id']}/approve").status_code == 204
        projected = client.post(f"/api/problems/{problem['id']}/project")
        assert projected.status_code == 201
        assert (tmp_path / projected.json()["path"]).exists()


def test_manual_problem_update_persists_title_and_multiline_detail(tmp_path: Path) -> None:
    with TestClient(create_app(tmp_path, tmp_path / "db.sqlite")) as client:
        capture = client.post("/api/captures", json={"text": "Original"}).json()
        problem = client.post(f"/api/captures/{capture['id']}/promote", json={}).json()

        response = client.put(f"/api/items/problems/{problem['id']}", json={
            "title": "Updated owner's problem",
            "detail": "Updated line one\nUpdated line two",
        })

        assert response.status_code == 204
        saved = next(item for item in client.get("/api/board").json()["problems"] if item["id"] == problem["id"])
        assert saved["statement"] == "Updated owner's problem"
        assert saved["detail"] == "Updated line one\nUpdated line two"


def test_recent_archive_panel_data_comes_from_the_vault_index(tmp_path: Path) -> None:
    archive = tmp_path / "2026" / "90. Archive" / "Features"
    archive.mkdir(parents=True)
    (archive / "Completed work.md").write_text("# Completed work\n\nA reusable result.")
    with TestClient(create_app(tmp_path, tmp_path / "db.sqlite")) as client:
        response = client.get("/api/workbench/recent-archive")
        assert response.status_code == 200
        assert response.json()["documents"][0]["path"] == "2026/90. Archive/Features/Completed work.md"


def test_promoted_capture_leaves_the_active_inbox_but_remains_linked(tmp_path: Path) -> None:
    with TestClient(create_app(tmp_path, tmp_path / "db.sqlite")) as client:
        capture = client.post("/api/captures", json={"text": "A raw thought"}).json()
        problem = client.post(f"/api/captures/{capture['id']}/promote", json={"statement": "A reviewable problem"}).json()
        assert client.post(f"/api/captures/{capture['id']}/promote", json={}).status_code == 201
        board = client.get("/api/board").json()
        assert board["captures"] == []
        assert board["problems"][0]["id"] == problem["id"]


def test_completed_problem_writes_a_summary_first_playbook(tmp_path: Path) -> None:
    with TestClient(create_app(tmp_path, tmp_path / "db.sqlite")) as client:
        capture = client.post("/api/captures", json={"text": "Preserve reusable decisions"}).json()
        problem = client.post(f"/api/captures/{capture['id']}/promote", json={"detail": "Context and evidence"}).json()
        response = client.post(f"/api/problems/{problem['id']}/complete", json={"reason": "Human review complete"})
        assert response.status_code == 200
        playbook = tmp_path / response.json()["path"]
        assert playbook.exists()
        assert problem["id"][:8] not in playbook.name
        assert "In Progress" not in playbook.name
        content = playbook.read_text(encoding="utf-8")
        assert "## Executive Summary" in content
        assert "llm_wiki_problem_id" not in content
        assert "## Supporting evidence" in content
        assert "[Raw work record](<assets/" in content
        raw_path = next((playbook.parent / "assets").glob("*.raw.md"))
        raw = raw_path.read_text(encoding="utf-8")
        assert "Human review complete" in raw
        assert "## Feedback and workflow history" in raw
        cards = client.get("/api/workbench/completed-solutions?limit=20")
        assert cards.status_code == 200
        assert cards.json()["solutions"] == []


def test_completed_problem_archives_full_work_log_checklist_and_image_capture(tmp_path: Path) -> None:
    with TestClient(create_app(tmp_path, tmp_path / "db.sqlite")) as client:
        capture = client.post("/api/captures", json={"text": "Keep the work record"}).json()
        problem = client.post(f"/api/captures/{capture['id']}/promote", json={}).json()
        assert client.post(f"/api/problems/{problem['id']}/approve").status_code == 204
        solution = client.post(f"/api/problems/{problem['id']}/features", json={"title": "Preserve history", "outcome": "Review the record", "non_goals": "None", "validation_criteria": "- [ ] Review the record"}).json()
        assert client.put(f"/api/features/{solution['id']}/conflict", json={"state": "clear", "citation": "Human review"}).status_code == 200
        assert client.post(f"/api/features/{solution['id']}/approve").status_code == 204
        entry = client.post(f"/api/features/{solution['id']}/progress", json={"body": "Captured the final state", "image_data": "aGVsbG8=", "image_media_type": "image/png"}).json()
        assert client.post(f"/api/progress/{entry['id']}/comments", json={"body": "Keep this evidence"}).status_code == 201
        client.app.state.workflow.set_solution_progress_summary(entry["id"], "The final state is captured.")
        response = client.post(f"/api/problems/{problem['id']}/complete", json={"reason": "Archive the full work record"})
        assert response.status_code == 200
        playbook = tmp_path / response.json()["path"]
        content = playbook.read_text(encoding="utf-8")
        raw_path = next((playbook.parent / "assets").glob("*.raw.md"))
        raw = raw_path.read_text(encoding="utf-8")
        assert "Captured the final state" in raw
        assert "Keep this evidence" in raw
        assert "- [ ] Review the record" in raw
        assert "Canonical AI image summary: The final state is captured." in raw
        assert f"![[{entry['id']}.png]]" in raw
        assert f"![[2026/90. Archive/Completed Work/assets/{entry['id']}.png]]" not in raw
        assert "[Raw work record](<assets/" in content
        assert "[Captured image" not in content
        image_path = playbook.parent / "assets" / f"{entry['id']}.png"
        assert image_path.read_bytes() == b"hello"
        regenerated = client.post(f"/api/problems/{problem['id']}/completion-playbook/regenerate")
        assert regenerated.status_code == 200
        assert regenerated.json()["path"] == response.json()["path"]
        cards = client.get("/api/workbench/completed-solutions?limit=20")
        assert cards.status_code == 200
        assert cards.json()["solutions"][0]["completion_playbook_path"] == response.json()["path"]
        assert cards.json()["solutions"][0]["archive_status"] == "available"
        playbook.write_text(playbook.read_text(encoding="utf-8") + "\nExternal note\n", encoding="utf-8")
        assert client.delete(f"/api/problems/{problem['id']}/completion-playbook").status_code == 409
        assert client.delete(f"/api/problems/{problem['id']}/completion-playbook?force=true").status_code == 204
        assert not playbook.exists()
        assert not raw_path.exists()
        assert not image_path.exists()
        remaining = client.get("/api/workbench/completed-solutions?limit=20").json()["solutions"]
        assert remaining[0]["id"] == solution["id"]
        assert remaining[0]["archive_status"] == "missing"


def test_api_deletes_and_restores_a_capture(tmp_path: Path) -> None:
    with TestClient(create_app(tmp_path, tmp_path / "db.sqlite")) as client:
        capture = client.post("/api/captures", json={"text": "Remove me"}).json()
        assert client.delete(f"/api/items/captures/{capture['id']}").status_code == 204
        assert client.get("/api/board").json()["captures"] == []
        assert client.post(f"/api/items/captures/{capture['id']}/restore").status_code == 204
        assert client.get("/api/board").json()["captures"][0]["id"] == capture["id"]


def test_completed_solution_can_explicitly_create_a_follow_up_problem(tmp_path: Path) -> None:
    with TestClient(create_app(tmp_path, tmp_path / "db.sqlite")) as client:
        workflow = client.app.state.workflow
        problem = workflow.promote_capture(workflow.capture("Preserve a completed result"))
        workflow.approve_problem(problem["id"])
        solution = workflow.create_feature(
            problem["id"], "Completed outcome", "The original work is done.", "", "- [ ] Done"
        )
        workflow.record_conflict_evaluation(solution["id"], "clear", "Human review")
        workflow.approve_feature(solution["id"])
        workflow.record_completion(solution["id"], "Done", "Recorded")
        workflow.db.execute("UPDATE completions SET knowledge_status='integrated' WHERE feature_id=?", (solution["id"],))
        workflow.db.commit()
        workflow.verify_completion(solution["id"])
        workflow.complete_problem(problem["id"], "Finished")

        response = client.post(f"/api/features/{solution['id']}/follow-up-problem")

        assert response.status_code == 201
        follow_up = response.json()
        assert follow_up["statement"] == "Follow up: Completed outcome"
        assert "The original work is done." in follow_up["detail"]
        assert client.get("/api/board").json()["problems"][0]["id"] == follow_up["id"]
        lineage = client.get(f"/api/problems/{follow_up['id']}/refinement-context").json()["entries"]
        assert {entry["label"]: entry["text"] for entry in lineage}["Source Completed Solution"] == "Completed outcome"


def test_completion_generates_final_document_from_lineage_context(tmp_path: Path, monkeypatch) -> None:
    provider_inputs: list[tuple[str, list[dict[str, object]]]] = []
    credential_tasks: list[str | None] = []

    class RecordingProvider:
        def __init__(self, *_: str) -> None:
            pass

        def complete_json(self, messages: list[dict[str, object]], label: str) -> dict[str, object]:
            provider_inputs.append((label, messages))
            if label == "lineage inference":
                return {"claims": []}
            return {
                "executive_summary_markdown": "### Completed\n- Lineage preserved [Evidence: Original capture]\n### Decisions\n- Human reviewed [Evidence: Completion decision]\n### Verification\n- Evidence linked [Evidence: Work log 1]\n### Risks and follow-up\n- None recorded",
                "report_body_markdown": "### Why\nPreserve origin [Evidence: Problem record].\n### What changed\nLineage was recorded.\n### How the work was carried out\nEvidence was linked [Evidence: Work log 1].\n### Final verification\nHuman reviewed [Evidence: Validation criterion 1].\n### Decision and risks\nNo recorded risk.",
            }

    app = create_app(tmp_path, tmp_path / "db.sqlite")
    app.state.provider_settings.save("http://provider.test/v1", "test-model", None)
    monkeypatch.setattr(app.state.provider_settings, "_secret", lambda: "test-key")
    original_credentials = app.state.provider_settings.credentials
    monkeypatch.setattr(
        app.state.provider_settings,
        "credentials",
        lambda task=None: (credential_tasks.append(task), original_credentials(task))[1],
    )
    monkeypatch.setattr(api_module, "OpenAICompatibleProvider", RecordingProvider)
    with TestClient(app) as client:
        capture = client.post("/api/captures", json={"text": "Original lineage feedback"}).json()
        problem = client.post(f"/api/captures/{capture['id']}/promote", json={"detail": "## Desired outcome\nTrace the origin"}).json()
        client.post(f"/api/problems/{problem['id']}/approve")
        solution = client.post(f"/api/problems/{problem['id']}/features", json={
            "title": "Trace completed work",
            "outcome": "The final record includes Lineage",
            "non_goals": "No extra workflow stage",
            "validation_criteria": "- [ ] Four stages are present",
        }).json()
        client.put(f"/api/features/{solution['id']}/conflict", json={"state": "clear", "citation": "Human review"})
        client.post(f"/api/features/{solution['id']}/approve")
        client.post(f"/api/features/{solution['id']}/progress", json={"body": "Browser persistence verified"})

        completed = client.post(f"/api/problems/{problem['id']}/complete", json={"reason": "Human completion review"})
        assert completed.status_code == 200
        assert completed.json()["lineage"]["status"] == "ready"
        lineage = client.get(f"/api/features/{solution['id']}/lineage")
        assert lineage.status_code == 200
        assert [stage["kind"] for stage in lineage.json()["lineage"]["stages"]] == ["capture", "problem", "solution", "complete"]
        assert client.get(f"/api/items/captures/{capture['id']}").json()["kind"] == "capture"
        assert client.get(f"/api/items/problems/{problem['id']}").json()["kind"] == "problem"
        assert client.get(f"/api/items/features/{solution['id']}").json()["kind"] == "solution"
        assert client.get("/api/items/features/missing-solution").status_code == 404
        document = (tmp_path / completed.json()["path"]).read_text(encoding="utf-8")
        assert "## Lineage" in document
        assert "**Snapshot**:" not in document
        assert "## Decision Changes" in document
        assert "## Conflicts & Addresses" in document
        assert "## Completion Evidence" in document
        assert "[Evidence: Original capture]" in document
        assert re.search(r"\[Evidence:[^\]]*[0-9a-f]{8}-[0-9a-f]{4}", document, re.IGNORECASE) is None

    report_label, report_messages = provider_inputs[-1]
    assert report_label == "completion executive summary"
    context = json.loads(str(report_messages[1]["content"]))
    report_snapshot = context["lineage_snapshots"][0]
    assert "snapshot_id" not in report_snapshot
    assert any(item["label"] == "Original capture" for item in report_snapshot["referenced_evidence"])
    assert any(item["label"] == "Problem record" for item in report_snapshot["referenced_evidence"])
    assert any(item["label"] == "Work log 1" for item in report_snapshot["referenced_evidence"])
    assert any(item["label"] == "Validation criterion 1" for item in report_snapshot["referenced_evidence"])
    uuid_pattern = r"[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}"
    assert re.search(uuid_pattern, str(report_messages[1]["content"]), re.IGNORECASE) is None
    assert "Feedback and workflow history" not in str(report_messages[1]["content"])
    assert credential_tasks == ["lineage_inference", "completion_report"]


def test_completed_document_regeneration_rebuilds_current_lineage(tmp_path: Path) -> None:
    with TestClient(create_app(tmp_path, tmp_path / "db.sqlite")) as client:
        workflow = client.app.state.workflow
        problem = workflow.promote_capture(workflow.capture("Original lifecycle context"))
        workflow.approve_problem(problem["id"])
        solution = workflow.create_feature(
            problem["id"], "Initial direction", "The current document is traceable",
            validation_criteria="- [ ] Current Lineage is included",
        )
        workflow.record_conflict_evaluation(solution["id"], "clear", "Human review")
        workflow.approve_feature(solution["id"])

        completed = client.post(f"/api/problems/{problem['id']}/complete", json={"reason": "Human reviewed"})
        assert completed.status_code == 200
        first = client.get(f"/api/features/{solution['id']}/lineage").json()
        workflow.update_manual("features", solution["id"], "Current direction", "The regenerated document follows current Lineage")

        regenerated = client.post(f"/api/problems/{problem['id']}/completion-playbook/regenerate")
        assert regenerated.status_code == 200
        current = client.get(f"/api/features/{solution['id']}/lineage").json()
        document = (tmp_path / regenerated.json()["path"]).read_text(encoding="utf-8")

        assert current["snapshot_id"] != first["snapshot_id"]
        assert "Current direction" in document
        assert "The regenerated document follows current Lineage" in document


def test_external_document_conflict_does_not_advance_lineage(tmp_path: Path) -> None:
    with TestClient(create_app(tmp_path, tmp_path / "db.sqlite")) as client:
        workflow = client.app.state.workflow
        problem = workflow.promote_capture(workflow.capture("Keep document and Lineage together"))
        workflow.approve_problem(problem["id"])
        solution = workflow.create_feature(
            problem["id"], "One lifecycle", "Lineage and document move together",
            validation_criteria="- [ ] External edits are protected",
        )
        workflow.record_conflict_evaluation(solution["id"], "clear", "Human review")
        workflow.approve_feature(solution["id"])
        completed = client.post(f"/api/problems/{problem['id']}/complete", json={"reason": "Reviewed"}).json()
        snapshot_id = client.get(f"/api/features/{solution['id']}/lineage").json()["snapshot_id"]
        document = tmp_path / completed["path"]
        document.write_text(document.read_text(encoding="utf-8") + "\nExternal edit\n", encoding="utf-8")

        rejected = client.post(f"/api/problems/{problem['id']}/completion-playbook/regenerate")
        lineage_rejected = client.post(
            f"/api/features/{solution['id']}/lineage/regenerate", json={"include_inference": False}
        )

        assert rejected.status_code == 409
        assert lineage_rejected.status_code == 409
        assert client.get(f"/api/features/{solution['id']}/lineage").json()["snapshot_id"] == snapshot_id


def test_lineage_regeneration_and_correction_preserve_audit_history(tmp_path: Path, monkeypatch) -> None:
    class InferenceProvider:
        def __init__(self, *_: str) -> None:
            pass

        def complete_json(self, messages: list[dict[str, object]], label: str) -> dict[str, object]:
            if label == "lineage inference":
                context = json.loads(str(messages[1]["content"]))
                evidence_id = context["lineage_snapshots"][0]["referenced_evidence"][0]["id"]
                return {"claims": [{
                    "claim_key": "inferred:likely-rationale",
                    "text": "Likely rationale based on the cited record",
                    "confidence": "medium",
                    "evidence_ids": [evidence_id],
                }]}
            return {"executive_summary_markdown": "", "report_body_markdown": ""}

    app = create_app(tmp_path, tmp_path / "db.sqlite")
    app.state.provider_settings.save("http://provider.test/v1", "test-model", None)
    monkeypatch.setattr(app.state.provider_settings, "_secret", lambda: "test-key")
    monkeypatch.setattr(api_module, "OpenAICompatibleProvider", InferenceProvider)
    with TestClient(app) as client:
        workflow = client.app.state.workflow
        problem = workflow.promote_capture(workflow.capture("Correct an interpretation"))
        workflow.approve_problem(problem["id"])
        solution = workflow.create_feature(problem["id"], "Correct safely", "Knowledge stays accurate", validation_criteria="- [ ] Correction is current")
        workflow.record_conflict_evaluation(solution["id"], "clear", "Human review")
        workflow.approve_feature(solution["id"])
        workflow.complete_problem(problem["id"], "Reviewed")
        workflow.create_lineage_snapshot(solution["id"])

        regenerated = client.post(f"/api/features/{solution['id']}/lineage/regenerate", json={"include_inference": True})
        assert regenerated.status_code == 201
        inferred = next(claim for claim in regenerated.json()["claims"].values() if claim["classification"] == "inferred" and claim["claim_key"].startswith("inferred:"))
        corrected = client.post(
            f"/api/features/{solution['id']}/lineage/claims/{inferred['id']}/corrections",
            json={"text": "The user explicitly corrected this interpretation", "reason": "Audit correction", "current_revision_id": inferred["current_revision_id"]},
        )
        assert corrected.status_code == 201
        current = client.get(f"/api/features/{solution['id']}/lineage").json()["claims"][inferred["id"]]
        assert current["text"] == "The user explicitly corrected this interpretation"
        assert [revision["author_type"] for revision in current["revisions"]] == ["ai", "user"]


def test_conflict_address_api_does_not_allow_ai_to_mark_addressed(tmp_path: Path) -> None:
    with TestClient(create_app(tmp_path, tmp_path / "db.sqlite")) as client:
        workflow = client.app.state.workflow
        problem = workflow.promote_capture(workflow.capture("Conflicting requirement"))
        workflow.approve_problem(problem["id"])
        solution = workflow.create_feature(problem["id"], "Choose safely", "A decision is supported", validation_criteria="- [ ] Basis is recorded")
        detected = client.put(f"/api/features/{solution['id']}/conflict", json={"state": "conflicted", "citation": "Requirements differ"})
        assert detected.status_code == 200

        rejected = client.put(f"/api/features/{solution['id']}/conflict", json={
            "state": "clear",
            "citation": "AI guessed",
            "address": {
                "basis": "ai_inferred",
                "disposition": "modified",
                "summary": "Likely addressed",
                "evidence_source_type": "conflict_report",
                "evidence_source_id": detected.json()["report_id"],
            },
        })
        assert rejected.status_code == 400
        assert workflow.db.execute("SELECT conflict_state FROM features WHERE id=?", (solution["id"],)).fetchone()[0] == "conflicted"


def test_completed_chat_rejects_an_active_solution(tmp_path: Path) -> None:
    with TestClient(create_app(tmp_path, tmp_path / "db.sqlite")) as client:
        workflow = client.app.state.workflow
        problem = workflow.promote_capture(workflow.capture("Still active"))
        workflow.approve_problem(problem["id"])
        solution = workflow.create_feature(problem["id"], "Active Solution", "Not done", "", "- [ ] Done")

        response = client.post(f"/api/features/{solution['id']}/completed-chat", json={"message": "What happened?"})

        assert response.status_code == 400
        assert "Completed Solution not found" in response.text


def test_provider_configuration_route_is_not_captured_as_an_item_update(tmp_path: Path) -> None:
    vault = tmp_path / "vault"
    vault.mkdir()
    with TestClient(create_app(vault, tmp_path / "db.sqlite3")) as client:
        response = client.put("/api/provider/config", json={"base_url": "http://127.0.0.1:8317/v1", "model": "test-model", "advanced_model": "advanced-model", "advanced_tasks": {}})
        assert response.status_code == 200
        assert response.json()["model"] == "test-model"


def test_item_update_route_does_not_return_method_not_allowed(tmp_path: Path) -> None:
    vault = tmp_path / "vault"
    vault.mkdir()
    with TestClient(create_app(vault, tmp_path / "db.sqlite3")) as client:
        capture = client.post("/api/captures", json={"text": "Original Capture"}).json()
        response = client.put(f"/api/items/captures/{capture['id']}", json={"title": "Refined Capture", "detail": ""})
        assert response.status_code == 204
        assert client.get("/api/board").json()["captures"][0]["text"] == "Refined Capture"
