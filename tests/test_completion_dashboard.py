import json
import sqlite3
from pathlib import Path

import pytest
from fastapi.testclient import TestClient

import llm_wiki.api.app as api_module
from llm_wiki.api.app import create_app
from llm_wiki.services.workflow import WorkflowEngine, WorkflowError


def approved_feature(workflow: WorkflowEngine) -> tuple[str, str]:
    capture = workflow.capture("A recurring problem")
    problem = workflow.promote_capture(capture)
    workflow.assess_importance(problem["id"], 5, 4, 2, 3, "Evidence from review")
    workflow.approve_problem(problem["id"])
    feature = workflow.create_feature(problem["id"], "A reusable outcome", "People can reuse knowledge", validation_criteria="- [ ] Reuse is observed")
    workflow.record_conflict_evaluation(feature["id"], "clear", "standards.md#Current")
    workflow.approve_feature(feature["id"])
    return problem["id"], feature["id"]


def test_completion_needs_knowledge_resolution_and_scores_on_verify() -> None:
    db = sqlite3.connect(":memory:"); db.row_factory = sqlite3.Row
    workflow = WorkflowEngine(db)
    _, feature = approved_feature(workflow)
    workflow.record_completion(feature, "Human reviewed results", "It worked", "No reusable knowledge was created")
    workflow.verify_completion(feature)
    assert workflow.dashboard()["events"]
    assert workflow.recent_completed_solutions()[0]["id"] == feature
    assert "Implementation handoff" in workflow.handoff(feature)


def test_human_can_complete_a_partially_resolved_problem_with_a_reason() -> None:
    db = sqlite3.connect(":memory:"); db.row_factory = sqlite3.Row
    workflow = WorkflowEngine(db)
    problem, feature = approved_feature(workflow)

    workflow.complete_problem(problem, "The remaining work is intentionally deferred.", "review-123")

    assert db.execute("SELECT state FROM problems WHERE id=?", (problem,)).fetchone()[0] == "completed"
    decision = db.execute("SELECT reason, review_id FROM problem_completion_decisions WHERE problem_id=?", (problem,)).fetchone()
    assert tuple(decision) == ("The remaining work is intentionally deferred.", "review-123")
    assert problem not in {item["id"] for item in workflow.board()["problems"]}
    assert feature not in {item["id"] for item in workflow.board()["features"]}


def test_unapproved_problem_cannot_create_feature() -> None:
    db = sqlite3.connect(":memory:"); db.row_factory = sqlite3.Row
    workflow = WorkflowEngine(db)
    problem = workflow.promote_capture(workflow.capture("draft"))
    with pytest.raises(WorkflowError, match="approved problem"):
        workflow.create_feature(problem["id"], "No", "No", validation_criteria="- [ ] Not used because parent is unapproved")


def test_completion_review_excludes_raw_image_data_from_provider_request(tmp_path: Path, monkeypatch) -> None:
    captured_messages: list[dict[str, object]] = []

    class RecordingProvider:
        def __init__(self, *_: str) -> None:
            pass

        def complete_json(self, messages: list[dict[str, object]], _: str) -> dict[str, object]:
            captured_messages.extend(messages)
            return {
                "resolution": "partial",
                "executive_summary": "The image summary was reviewed.",
                "what_changed": [],
                "criteria_review": [],
                "remaining_checklist": [],
                "decision_rationale": "Only recorded evidence was used.",
                "problem_recommendation": "keep_open",
                "capture_recommendation": "keep_open",
            }

    app = create_app(tmp_path, tmp_path / "db.sqlite")
    app.state.provider_settings.save("http://provider.test/v1", "test-model", None)
    monkeypatch.setattr(app.state.provider_settings, "_secret", lambda: "test-key")
    monkeypatch.setattr(api_module, "OpenAICompatibleProvider", RecordingProvider)

    with TestClient(app) as client:
        capture = client.post("/api/captures", json={"text": "Review image evidence"}).json()
        problem = client.post(f"/api/captures/{capture['id']}/promote", json={}).json()
        client.post(f"/api/problems/{problem['id']}/approve")
        feature = client.post(f"/api/problems/{problem['id']}/features", json={
            "title": "Review screenshots",
            "outcome": "The recorded result is reviewed",
            "non_goals": "None",
            "validation_criteria": "- [ ] The screenshot summary supports the result",
        }).json()
        client.put(f"/api/features/{feature['id']}/conflict", json={"state": "clear", "citation": "Human review"})
        client.post(f"/api/features/{feature['id']}/approve")
        entry = client.post(f"/api/features/{feature['id']}/progress", json={
            "body": "Captured the result",
            "image_data": "a" * 1_000_000,
            "image_media_type": "image/png",
        }).json()
        app.state.workflow.set_solution_progress_summary(entry["id"], "The expected result is visible.")

        response = client.post(f"/api/features/{feature['id']}/completion-review")

    assert response.status_code == 200
    evidence = json.loads(str(captured_messages[-1]["content"]))
    assert evidence["progress_records"][0]["image_summary"] == "The expected result is visible."
    assert "image_data" not in evidence["progress_records"][0]
    assert len(str(captured_messages[1]["content"])) < 10_000
