from pathlib import Path

from fastapi.testclient import TestClient

import llm_wiki.api.app as api_module
from llm_wiki.api.app import create_app


class FakeProvider:
    """A provider-boundary fake exercising every AI-backed Workbench action."""

    def __init__(self, *_: str) -> None:
        pass

    def stream(self, _: list[dict[str, str]]):
        yield "✅ Ready. Your AI refinement is ready to review."

    def complete_json(self, _: list[dict[str, str]], schema_name: str) -> dict[str, str]:
        if schema_name == "captures refinement":
            return {"title": "Clearer decision problem"}
        if schema_name == "captures draft":
            return {
                "title": "Decisions are difficult to find",
                "detail": "## Context\nDecisions are scattered.\n\n## Desired outcome\nTrusted decisions are easy to find.",
            }
        if schema_name == "problems refinement":
            return {"title": "Find approved decisions", "detail": "## Context\nDecisions are scattered.\n\n## Impact\nPeople lose time finding trusted decisions.\n\n## Evidence\nThe team reports difficulty finding decisions.\n\n## Desired outcome\nDecisions are easy to find.\n\n## Boundaries\nNo migration is assumed.\n\n## Open questions\nNot yet known."}
        if schema_name == "problems draft":
            return {"title": "One decision home", "outcome": "Approved decisions are easy to find", "non_goals": "No migration", "validation_criteria": "- [ ] A person can find an approved decision"}
        if schema_name == "features refinement":
            return {"title": "One decision home", "detail": "People find trusted decisions"}
        raise AssertionError(f"Unexpected schema: {schema_name}")


class LongChatProvider(FakeProvider):
    def stream(self, _: list[dict[str, str]]):
        yield "x" * 2_000


class FocusCapturingProvider(FakeProvider):
    messages: list[dict[str, str]] = []

    def stream(self, messages: list[dict[str, str]]):
        self.__class__.messages = messages
        yield "Which observed failure best demonstrates the impact?"


class BilingualDraftProvider(FakeProvider):
    calls = 0

    def complete_json(self, _: list[dict[str, str]], schema_name: str):
        type(self).calls += 1
        if schema_name == "captures draft":
            return {
                "ko": {"title": "결정을 찾기 어렵다", "detail": "## 맥락\n결정이 흩어져 있다."},
                "en": {"title": "Decisions are hard to find", "detail": "## Context\nDecisions are scattered."},
            }
        return super().complete_json(_, schema_name)


def test_workbench_chat_allows_explanations_beyond_the_old_240_character_limit(tmp_path: Path, monkeypatch) -> None:
    app = create_app(tmp_path, tmp_path / "db.sqlite")
    app.state.provider_settings.save("http://127.0.0.1:8317/v1", "test-model", None)
    monkeypatch.setattr(app.state.provider_settings, "_secret", lambda: "test-key")
    monkeypatch.setattr(api_module, "OpenAICompatibleProvider", LongChatProvider)

    with TestClient(app) as client:
        capture = client.post("/api/captures", json={"text": "Decisions are scattered"}).json()
        response = client.post(f"/api/captures/{capture['id']}/chat", json={"message": "Help me understand this"})

    streamed_text = "".join(
        line.removeprefix("data: ")
        for line in response.text.splitlines()
        if line.startswith("data: x")
    )
    assert len(streamed_text) == 1_200


def test_problem_chat_receives_the_same_visible_focus_as_preview(tmp_path: Path, monkeypatch) -> None:
    app = create_app(tmp_path, tmp_path / "focus.sqlite")
    app.state.provider_settings.save("http://127.0.0.1:8317/v1", "test-model", None)
    monkeypatch.setattr(app.state.provider_settings, "_secret", lambda: "test-key")
    monkeypatch.setattr(api_module, "OpenAICompatibleProvider", FocusCapturingProvider)
    FocusCapturingProvider.messages = []

    with TestClient(app) as client:
        capture = client.post("/api/captures", json={"text": "AI work blocks without explanation"}).json()
        problem = client.post(f"/api/captures/{capture['id']}/promote", json={}).json()
        preview = client.get(f"/api/problems/{problem['id']}/refinement-context").json()
        response = client.post(f"/api/problems/{problem['id']}/chat", json={"message": "Help me sharpen this"})

    assert response.status_code == 200
    visible_focus = ", ".join(item["label"] for item in preview["focus"])
    focus_messages = [message["content"] for message in FocusCapturingProvider.messages if "visible Refinement Preview" in message["content"]]
    assert len(focus_messages) == 1
    assert visible_focus in focus_messages[0]
    assert "only on this focus group" in focus_messages[0]
    assert "exactly one sharp, open-ended question" in focus_messages[0]


def test_live_chat_uses_request_start_locale_once(tmp_path: Path, monkeypatch) -> None:
    app = create_app(tmp_path, tmp_path / "locale-chat.sqlite")
    app.state.provider_settings.save("http://127.0.0.1:8317/v1", "test-model", None)
    monkeypatch.setattr(app.state.provider_settings, "_secret", lambda: "test-key")
    monkeypatch.setattr(api_module, "OpenAICompatibleProvider", FocusCapturingProvider)
    FocusCapturingProvider.messages = []
    with TestClient(app) as client:
        capture = client.post("/api/captures", json={"text": "결정이 흩어져 있다"}).json()
        response = client.post(
            f"/api/captures/{capture['id']}/chat",
            json={"message": "도와주세요"},
            headers={"X-LLM-Wiki-Locale": "ko"},
        )
    assert response.status_code == 200
    locale_messages = [message["content"] for message in FocusCapturingProvider.messages if "Respond only in natural" in message["content"]]
    assert len(locale_messages) == 1
    assert "Korean" in locale_messages[0]


def test_one_bilingual_draft_call_supplies_stored_problem_versions(tmp_path: Path, monkeypatch) -> None:
    app = create_app(tmp_path, tmp_path / "bilingual-draft.sqlite")
    app.state.provider_settings.save("http://127.0.0.1:8317/v1", "test-model", None)
    monkeypatch.setattr(app.state.provider_settings, "_secret", lambda: "test-key")
    monkeypatch.setattr(api_module, "OpenAICompatibleProvider", BilingualDraftProvider)
    BilingualDraftProvider.calls = 0
    with TestClient(app) as client:
        capture = client.post("/api/captures", json={"text": "결정이 흩어져 있다"}).json()
        draft = client.post(f"/api/captures/{capture['id']}/draft", headers={"X-LLM-Wiki-Locale": "ko"}).json()
        assert set(draft["localized_versions"]) == {"ko", "en"}
        problem = client.post(
            f"/api/captures/{capture['id']}/promote",
            json={"statement": draft["title"], "detail": draft["detail"], "localized_versions": draft["localized_versions"]},
        ).json()
        english = client.get("/api/board", headers={"X-LLM-Wiki-Locale": "en"}).json()["problems"][0]
        korean = client.get("/api/board", headers={"X-LLM-Wiki-Locale": "ko"}).json()["problems"][0]
    assert problem["id"] == english["id"] == korean["id"]
    assert english["statement"] == "Decisions are hard to find"
    assert korean["statement"] == "결정을 찾기 어렵다"
    assert BilingualDraftProvider.calls == 1


def test_workbench_ai_and_human_actions_follow_the_inbox_to_problem_flow(tmp_path: Path, monkeypatch) -> None:
    app = create_app(tmp_path, tmp_path / "db.sqlite")
    app.state.provider_settings.save("http://127.0.0.1:8317/v1", "test-model", None)
    monkeypatch.setattr(app.state.provider_settings, "_secret", lambda: "test-key")
    monkeypatch.setattr(api_module, "OpenAICompatibleProvider", FakeProvider)

    with TestClient(app) as client:
        capture = client.post("/api/captures", json={"text": "Decisions are scattered"}).json()
        assert client.post(f"/api/captures/{capture['id']}/chat", json={"message": "It affects my team"}).status_code == 200
        refinement = client.post(f"/api/captures/{capture['id']}/refine").json()
        assert refinement["source_note"] == "Decisions are scattered"
        assert client.put(f"/api/items/captures/{capture['id']}", json={"title": refinement["title"], "detail": ""}).status_code == 204
        assert client.post(f"/api/captures/{capture['id']}/next-chat", json={"message": "People need trusted decisions"}).status_code == 200
        problem_draft = client.post(f"/api/captures/{capture['id']}/draft").json()
        problem = client.post(
            f"/api/captures/{capture['id']}/promote",
            json={"statement": problem_draft["title"], "detail": problem_draft["detail"]},
        ).json()
        board = client.get("/api/board").json()
        assert board["captures"] == []
        assert board["problems"][0]["id"] == problem["id"]
        refined_problem = client.post(f"/api/problems/{problem['id']}/refine").json()
        assert "## Context" in refined_problem["detail"]
        assert client.put(f"/api/items/problems/{problem['id']}", json={"title": refined_problem["title"], "detail": refined_problem["detail"]}).status_code == 204
        assert "## Context" in client.get("/api/board").json()["problems"][0]["detail"]
        assert client.post(f"/api/problems/{problem['id']}/approve").status_code == 204

        feature_draft = client.post(f"/api/problems/{problem['id']}/draft").json()
        feature = client.post(f"/api/problems/{problem['id']}/features", json=feature_draft).json()
        assert client.put(f"/api/features/{feature['id']}/conflict", json={"state": "clear", "citation": "context.md#Current"}).status_code == 200
        assert client.post(f"/api/features/{feature['id']}/approve").status_code == 204
        entry = client.post(f"/api/features/{feature['id']}/progress", json={"body": "Reviewed the decision home", "image_data": "aGVsbG8=", "image_media_type": "image/png"}).json()
        assert client.post(f"/api/progress/{entry['id']}/comments", json={"body": "The current state is ready for review."}).status_code == 201
        progress = client.get(f"/api/features/{feature['id']}/progress").json()
        assert progress["entries"][0]["body"] == "Reviewed the decision home"
        assert progress["entries"][0]["comments"][0]["body"] == "The current state is ready for review."
        checklist = client.post(f"/api/features/{feature['id']}/checklist", json={"body": "Confirm the review"}).json()
        assert client.put(f"/api/checklist/{checklist['id']}", json={"body": "Confirm the review", "checked": True}).status_code == 204
        assert client.get(f"/api/features/{feature['id']}/handoff").status_code == 200


def test_refinement_context_contract_supports_capture_problem_and_solution(tmp_path: Path) -> None:
    app = create_app(tmp_path, tmp_path / "context.sqlite")

    with TestClient(app) as client:
        capture = client.post("/api/captures", json={"text": "A context-bearing problem"}).json()
        problem = client.post(
            f"/api/captures/{capture['id']}/promote",
            json={"detail": "People need the earlier evidence while refining."},
        ).json()
        problem_context = client.get(f"/api/problems/{problem['id']}/refinement-context")
        assert problem_context.status_code == 200
        assert problem_context.json()["entries"][0] == {
            "label": "Current item",
            "text": "A context-bearing problem",
        }
        assert problem_context.json()["entries"][1] == {
            "label": "Current context",
            "text": "People need the earlier evidence while refining.",
        }

        client.post(f"/api/problems/{problem['id']}/approve")
        solution = client.post(
            f"/api/problems/{problem['id']}/features",
            json={
                "title": "Keep context visible",
                "outcome": "Refiners can recall the current decision.",
                "validation_criteria": "- [ ] Context appears",
            },
        ).json()
        solution_context = client.get(f"/api/features/{solution['id']}/refinement-context")
        assert solution_context.status_code == 200
        assert solution_context.json()["has_context"] is True
        assert solution_context.json()["entries"][0] == {
            "label": "Current item",
            "text": "Keep context visible",
        }
        assert "current decision" in solution_context.json()["entries"][1]["text"]

        assert client.get("/api/problems/missing/refinement-context").status_code == 404
        capture_context = client.get(f"/api/captures/{capture['id']}/refinement-context")
        assert capture_context.status_code == 200
        assert capture_context.json()["current_detail"]["kind"] == "capture"
