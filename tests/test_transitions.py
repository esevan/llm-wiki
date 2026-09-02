"""Tests for Workflow Transitions: menu-only routes with required input forms.

Covers the validation criteria:
- All defined Workflow Transitions are accessible via the menu (API).
- Each transition's required input form is enforced.
- Conflict Check can be skipped with a reason.
- A Solution can be Completed without a report when a no-update reason is given.
"""
from pathlib import Path

import pytest
from fastapi.testclient import TestClient

from llm_wiki.api.app import create_app
from llm_wiki.services.workflow import TRANSITIONS, WorkflowEngine, WorkflowError, available_transitions


def test_transitions_are_defined_for_all_required_routes() -> None:
    """Capture->Refine, Problem->Propose, Solution->Approved, In progress->Completed."""
    ids = {t["id"] for t in TRANSITIONS}
    assert "capture_to_problem" in ids
    assert "problem_to_solution" in ids
    assert "solution_to_approved" in ids
    assert "solution_to_completed" in ids


def test_each_transition_declares_required_input_fields() -> None:
    for transition in TRANSITIONS:
        assert transition["fields"], f"{transition['id']} has no fields"
        required = [f for f in transition["fields"] if f.get("required")]
        assert required, f"{transition['id']} has no required fields"


def test_manual_transition_definitions_use_short_explicit_actions() -> None:
    for transition in TRANSITIONS:
        assert "manually" in str(transition["label"]).lower()
        assert len(str(transition["submit_label"])) <= 12


def test_solution_transition_selects_have_human_readable_options() -> None:
    approval = next(t for t in TRANSITIONS if t["id"] == "solution_to_approved")
    path = next(f for f in approval["fields"] if f["name"] == "approval_path")
    assert path["options"] == [
        {"value": "checked", "label": "Already checked"},
        {"value": "skip", "label": "Skip with a reason"},
    ]


def test_draft_problem_does_not_offer_manual_solution_creation() -> None:
    draft_ids = {item["id"] for item in available_transitions("problems", {"state": "draft"})}
    approved_ids = {item["id"] for item in available_transitions("problems", {"state": "approved"})}
    assert "problem_to_solution" not in draft_ids
    assert "problem_to_solution" in approved_ids


def test_new_manual_approval_path_enforces_its_conditional_reason() -> None:
    import sqlite3
    db = sqlite3.connect(":memory:")
    db.row_factory = sqlite3.Row
    workflow = WorkflowEngine(db)
    problem = workflow.promote_capture(workflow.capture("A thought"))
    workflow.approve_problem(problem["id"])
    feature = workflow.create_feature(problem["id"], "S", "O", validation_criteria="- [ ] Done")
    with pytest.raises(WorkflowError, match="Review basis is required"):
        workflow.apply_transition("solution_to_approved", "features", feature["id"], {
            "approval_path": "checked", "citation": ""
        })


def test_apply_transition_capture_to_problem_requires_statement() -> None:
    import sqlite3
    db = sqlite3.connect(":memory:")
    db.row_factory = sqlite3.Row
    workflow = WorkflowEngine(db)
    capture_id = workflow.capture("A raw thought")
    try:
        workflow.apply_transition("capture_to_problem", "captures", capture_id, {"statement": ""})
        assert False, "Should have raised"
    except WorkflowError as error:
        assert "required" in str(error).lower()


def test_apply_transition_capture_to_problem_creates_problem() -> None:
    import sqlite3
    db = sqlite3.connect(":memory:")
    db.row_factory = sqlite3.Row
    workflow = WorkflowEngine(db)
    capture_id = workflow.capture("A raw thought")
    result = workflow.apply_transition("capture_to_problem", "captures", capture_id, {"statement": "A problem", "detail": "Context"})
    assert result["statement"] == "A problem"
    assert result["detail"] == "Context"
    # The capture leaves the active inbox.
    assert not workflow.board()["captures"]
    assert workflow.board()["problems"]


def test_api_manual_capture_to_problem_preserves_category(tmp_path: Path) -> None:
    with TestClient(create_app(tmp_path, tmp_path / "db.sqlite")) as client:
        capture = client.post("/api/captures", json={"text": "A categorized Capture"}).json()
        assert client.put("/api/workbench/category", json={
            "entity_type": "captures",
            "entity_id": capture["id"],
            "category": "LLM Wiki",
        }).status_code == 204

        response = client.post(f"/api/transitions/captures/{capture['id']}", json={
            "transition_id": "capture_to_problem",
            "fields": {"statement": "A manually created Problem", "detail": ""},
        })

        assert response.status_code == 200
        problem_id = response.json()["id"]
        problem = next(item for item in client.get("/api/board").json()["problems"] if item["id"] == problem_id)
        assert problem["category"] == "LLM Wiki"


def test_apply_transition_problem_to_solution_requires_approved_problem() -> None:
    import sqlite3
    db = sqlite3.connect(":memory:")
    db.row_factory = sqlite3.Row
    workflow = WorkflowEngine(db)
    capture_id = workflow.capture("A thought")
    problem = workflow.promote_capture(capture_id)
    try:
        workflow.apply_transition("problem_to_solution", "problems", problem["id"], {
            "title": "S", "outcome": "O", "validation_criteria": "- [ ] Done"
        })
        assert False, "Should have raised"
    except WorkflowError as error:
        assert "approved" in str(error).lower()


def test_apply_transition_solution_to_approved_with_clear_citation() -> None:
    import sqlite3
    db = sqlite3.connect(":memory:")
    db.row_factory = sqlite3.Row
    workflow = WorkflowEngine(db)
    capture_id = workflow.capture("A thought")
    problem = workflow.promote_capture(capture_id)
    workflow.approve_problem(problem["id"])
    feature = workflow.create_feature(problem["id"], "S", "O", validation_criteria="- [ ] Done")
    result = workflow.apply_transition("solution_to_approved", "features", feature["id"], {
        "conflict_state": "clear", "citation": "context.md#Current"
    })
    assert result["approved"] is True


def test_apply_transition_solution_to_approved_skip_conflict_check_requires_reason() -> None:
    import sqlite3
    db = sqlite3.connect(":memory:")
    db.row_factory = sqlite3.Row
    workflow = WorkflowEngine(db)
    capture_id = workflow.capture("A thought")
    problem = workflow.promote_capture(capture_id)
    workflow.approve_problem(problem["id"])
    feature = workflow.create_feature(problem["id"], "S", "O", validation_criteria="- [ ] Done")
    try:
        workflow.apply_transition("solution_to_approved", "features", feature["id"], {
            "conflict_state": "clear", "skip_conflict_check": True, "skip_reason": ""
        })
        assert False, "Should have raised"
    except WorkflowError as error:
        assert "reason" in str(error).lower()


def test_apply_transition_solution_to_approved_skip_conflict_check_with_reason() -> None:
    import sqlite3
    db = sqlite3.connect(":memory:")
    db.row_factory = sqlite3.Row
    workflow = WorkflowEngine(db)
    capture_id = workflow.capture("A thought")
    problem = workflow.promote_capture(capture_id)
    workflow.approve_problem(problem["id"])
    feature = workflow.create_feature(problem["id"], "S", "O", validation_criteria="- [ ] Done")
    result = workflow.apply_transition("solution_to_approved", "features", feature["id"], {
        "conflict_state": "clear", "skip_conflict_check": True, "skip_reason": "No existing work to conflict with"
    })
    assert result["approved"] is True


def test_apply_transition_solution_to_completed_without_report_requires_no_update_reason() -> None:
    import sqlite3
    db = sqlite3.connect(":memory:")
    db.row_factory = sqlite3.Row
    workflow = WorkflowEngine(db)
    capture_id = workflow.capture("A thought")
    problem = workflow.promote_capture(capture_id)
    workflow.approve_problem(problem["id"])
    feature = workflow.create_feature(problem["id"], "S", "O", validation_criteria="- [ ] Done")
    workflow.record_conflict_evaluation(feature["id"], "clear", "context.md")
    workflow.approve_feature(feature["id"])
    try:
        workflow.apply_transition("solution_to_completed", "features", feature["id"], {
            "evidence": "Done", "report": "", "no_update_reason": ""
        })
        assert False, "Should have raised"
    except WorkflowError as error:
        assert "report" in str(error).lower() or "reason" in str(error).lower()


def test_apply_transition_solution_to_completed_with_no_update_reason() -> None:
    import sqlite3
    db = sqlite3.connect(":memory:")
    db.row_factory = sqlite3.Row
    workflow = WorkflowEngine(db)
    capture_id = workflow.capture("A thought")
    problem = workflow.promote_capture(capture_id)
    workflow.approve_problem(problem["id"])
    feature = workflow.create_feature(problem["id"], "S", "O", validation_criteria="- [ ] Done")
    workflow.record_conflict_evaluation(feature["id"], "clear", "context.md")
    workflow.approve_feature(feature["id"])
    result = workflow.apply_transition("solution_to_completed", "features", feature["id"], {
        "evidence": "The work is done",
        "report": "",
        "no_update_reason": "No knowledge update needed",
        "reason": "Completed via transition"
    })
    assert result["completed"] is True
    assert result["problem_id"] == problem["id"]


def test_api_list_transitions_returns_all_definitions(tmp_path: Path) -> None:
    with TestClient(create_app(tmp_path, tmp_path / "db.sqlite")) as client:
        response = client.get("/api/transitions")
        assert response.status_code == 200
    transitions = response.json()["transitions"]
    ids = {t["id"] for t in transitions}
    assert "capture_to_problem" in ids
    assert "problem_to_solution" in ids
    assert "solution_to_approved" in ids
    assert "solution_to_completed" in ids


def test_api_apply_transition_capture_to_problem(tmp_path: Path) -> None:
    with TestClient(create_app(tmp_path, tmp_path / "db.sqlite")) as client:
        capture = client.post("/api/captures", json={"text": "A raw thought"}).json()
        response = client.post(f"/api/transitions/captures/{capture['id']}", json={
            "transition_id": "capture_to_problem",
            "fields": {"statement": "A problem", "detail": "Context"}
        })
        assert response.status_code == 200
        assert response.json()["statement"] == "A problem"
        board = client.get("/api/board").json()
        assert board["captures"] == []
        assert board["problems"][0]["id"] == response.json()["id"]


def test_api_apply_transition_solution_to_completed_writes_playbook(tmp_path: Path) -> None:
    with TestClient(create_app(tmp_path, tmp_path / "db.sqlite")) as client:
        capture = client.post("/api/captures", json={"text": "A thought"}).json()
        problem = client.post(f"/api/captures/{capture['id']}/promote", json={}).json()
        assert client.post(f"/api/problems/{problem['id']}/approve").status_code == 204
        solution = client.post(f"/api/problems/{problem['id']}/features", json={
            "title": "S", "outcome": "O", "non_goals": "", "validation_criteria": "- [ ] Done"
        }).json()
        assert client.put(f"/api/features/{solution['id']}/conflict", json={"state": "clear", "citation": "ctx"}).status_code == 200
        assert client.post(f"/api/features/{solution['id']}/approve").status_code == 204
        response = client.post(f"/api/transitions/features/{solution['id']}", json={
            "transition_id": "solution_to_completed",
            "fields": {
                "evidence": "Done",
                "report": "Completed report",
                "no_update_reason": "",
                "reason": "All done"
            }
        })
        assert response.status_code == 200
        result = response.json()
        assert result["completed"] is True
        assert "playbook" in result
        assert (tmp_path / result["playbook"]["path"]).exists()


def test_api_manual_completion_can_skip_note_creation(tmp_path: Path) -> None:
    with TestClient(create_app(tmp_path, tmp_path / "db.sqlite")) as client:
        capture = client.post("/api/captures", json={"text": "A thought"}).json()
        problem = client.post(f"/api/captures/{capture['id']}/promote", json={}).json()
        assert client.post(f"/api/problems/{problem['id']}/approve").status_code == 204
        solution = client.post(f"/api/problems/{problem['id']}/features", json={
            "title": "S", "outcome": "O", "non_goals": "", "validation_criteria": "- [ ] Done"
        }).json()
        assert client.put(f"/api/features/{solution['id']}/conflict", json={"state": "clear", "citation": "ctx"}).status_code == 200
        assert client.post(f"/api/features/{solution['id']}/approve").status_code == 204
        response = client.post(f"/api/transitions/features/{solution['id']}", json={
            "transition_id": "solution_to_completed",
            "fields": {
                "evidence": "Done",
                "completion_path": "no_update",
                "no_update_reason": "Nothing reusable changed",
                "reason": "All done",
            },
        })

    assert response.status_code == 200
    result = response.json()
    assert result["completed"] is True
    assert result["note_skipped"] is True
    assert "playbook" not in result
    assert list(tmp_path.rglob("*.md")) == []
