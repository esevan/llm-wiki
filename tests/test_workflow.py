import sqlite3
from time import perf_counter

import pytest

from llm_wiki.services.workflow import WorkflowEngine, WorkflowError


def test_board_has_only_problem_centered_product_stages() -> None:
    db = sqlite3.connect(":memory:")
    db.row_factory = sqlite3.Row
    assert set(WorkflowEngine(db).board()) == {"captures", "problems", "features"}


def test_feature_cannot_be_approved_until_a_cited_clear_evaluation() -> None:
    db = sqlite3.connect(":memory:")
    db.row_factory = sqlite3.Row
    workflow = WorkflowEngine(db)
    capture_id = workflow.capture("We lose decisions")
    problem = workflow.promote_capture(capture_id)
    workflow.approve_problem(problem["id"])
    feature = workflow.create_feature(problem["id"], "Decision home", "Decisions have one trusted home", validation_criteria="- [ ] A person finds a decision")
    with pytest.raises(WorkflowError, match="clear"):
        workflow.approve_feature(feature["id"])
    with pytest.raises(WorkflowError, match="cited"):
        workflow.record_conflict_evaluation(feature["id"], "clear", "")
    workflow.record_conflict_evaluation(feature["id"], "clear", "decisions/standards.md#Scope")
    workflow.approve_feature(feature["id"])
    workflow.set_feature_stage(feature["id"], "approved")
    progress = workflow.add_solution_progress(feature["id"], "People can find approved decisions")
    assert progress["body"] == "People can find approved decisions"


def test_deleting_parent_hides_children_and_can_be_restored() -> None:
    db = sqlite3.connect(":memory:")
    db.row_factory = sqlite3.Row
    workflow = WorkflowEngine(db)
    capture = workflow.capture("A problem")
    problem = workflow.promote_capture(capture)
    workflow.approve_problem(problem["id"])
    feature = workflow.create_feature(problem["id"], "Outcome", "A useful outcome", validation_criteria="- [ ] Outcome is observable")
    workflow.record_conflict_evaluation(feature["id"], "clear", "context.md#Evidence")
    workflow.approve_feature(feature["id"])
    workflow.delete("problems", problem["id"])
    assert not workflow.board()["problems"]
    assert not workflow.board()["features"]
    workflow.restore("problems", problem["id"])
    assert workflow.board()["problems"]


def test_workflow_chat_context_and_manual_update_are_state_neutral() -> None:
    db = sqlite3.connect(":memory:")
    db.row_factory = sqlite3.Row
    workflow = WorkflowEngine(db)
    capture = workflow.capture("Unclear problem")
    context = workflow.context_for("captures", capture)
    assert context["title"] == "Unclear problem"
    workflow.update_manual("captures", capture, "Clearer problem", "")
    assert workflow.context_for("captures", capture)["title"] == "Clearer problem"
    workflow.record_ai_run("captures", capture, "workflow_chat", "Help me", "Consider the evidence.")
    assert db.execute("SELECT count(*) FROM ai_runs").fetchone()[0] == 1
    assert workflow.chat_history("captures", capture) == [
        {"role": "user", "content": "Help me"},
        {"role": "assistant", "content": "Consider the evidence."},
    ]


def test_refinement_context_is_bounded_lineage_aware_and_recent_first() -> None:
    db = sqlite3.connect(":memory:")
    db.row_factory = sqlite3.Row
    workflow = WorkflowEngine(db)
    problem = workflow.promote_capture(
        workflow.capture("Decision context is hard to recover"),
        detail="Evidence shows people repeat old decisions.",
    )
    other = workflow.promote_capture(workflow.capture("Another item"), detail="Must not leak")
    workflow.record_ai_run("problems", problem["id"], "workflow_chat", "What is constrained?", "Avoid a migration project.")
    workflow.record_ai_run("problems", problem["id"], "workflow_refinement", "Refine current item", "Latest preview emphasizes trusted decisions.")
    workflow.record_ai_run("problems", other["id"], "workflow_chat", "Other question", "Other private answer")

    started = perf_counter()
    summary = workflow.refinement_context_summary("problems", problem["id"])
    elapsed = perf_counter() - started

    assert summary["has_context"] is True
    assert [entry["label"] for entry in summary["entries"]] == [
        "Current item",
        "Current context",
        "Previous preview",
        "Recent discussion",
    ]
    visible = "".join(entry["text"] for entry in summary["entries"])
    assert "Latest preview" in visible
    assert "Avoid a migration" in visible
    assert "Other private answer" not in visible
    assert len(summary["entries"]) == 4
    assert len(visible) <= 500
    assert elapsed < 0.1


def test_refinement_context_handles_empty_title_only_and_truncation() -> None:
    db = sqlite3.connect(":memory:")
    db.row_factory = sqlite3.Row
    workflow = WorkflowEngine(db)
    title_only = workflow.promote_capture(workflow.capture("Only a short title"))
    initial = workflow.refinement_context_summary("problems", title_only["id"])
    assert initial["has_context"] is True
    assert initial["entries"] == [{"label": "Current item", "text": "Only a short title"}]
    assert initial["view_mode"] == "context"

    workflow.update_manual("problems", title_only["id"], "Only a short title", "Not yet known")
    assert workflow.refinement_context_summary("problems", title_only["id"])["entries"] == [
        {"label": "Current item", "text": "Only a short title"}
    ]

    workflow.update_manual("problems", title_only["id"], "Only a short title", "Context " * 200)
    workflow.record_ai_run("problems", title_only["id"], "workflow_chat", "Question " * 100, "Answer " * 100)
    summary = workflow.refinement_context_summary("problems", title_only["id"])
    visible = "".join(entry["text"] for entry in summary["entries"])
    assert len(summary["entries"]) <= 4
    assert 300 <= len(visible) <= 500
    assert summary["entries"][-1]["text"].endswith("…")

    with pytest.raises(WorkflowError, match="Item not found"):
        workflow.refinement_context_summary("captures", "capture-id")
    with pytest.raises(WorkflowError, match="Capture, Problem, and Solution"):
        workflow.refinement_context_summary("questions", "question-id")


def test_refinement_preview_progresses_from_context_to_final_item_detail_shape() -> None:
    db = sqlite3.connect(":memory:")
    db.row_factory = sqlite3.Row
    workflow = WorkflowEngine(db)
    problem = workflow.promote_capture(
        workflow.capture("Reviewers cannot tell why AI work is blocked"),
        detail="The current queue does not explain enough yet.",
    )

    initial = workflow.refinement_context_summary("problems", problem["id"])
    assert initial["view_mode"] == "context"
    assert [item["label"] for item in initial["structure"]] == [
        "Problem statement", "Context", "Impact", "Evidence", "Desired outcome", "Boundaries", "Open questions"
    ]

    workflow.record_ai_run("problems", problem["id"], "workflow_chat", "The impact is that the team loses time every day.", "What evidence shows the cost?")
    workflow.record_ai_run("problems", problem["id"], "workflow_chat", "Evidence: three reviewers each lose an hour per week.", "What should improve?")
    developed = workflow.refinement_context_summary("problems", problem["id"])

    assert developed["view_mode"] == "structure"
    assert developed["readiness"]["missing"] > 0
    assert [item["label"] for item in developed["focus"]] == ["Evidence", "Impact"]
    evidence = next(item for item in developed["structure"] if item["key"] == "evidence")
    assert evidence["status"] == "weak"


def test_structured_solution_preview_matches_refined_solution_detail_fields() -> None:
    db = sqlite3.connect(":memory:")
    db.row_factory = sqlite3.Row
    workflow = WorkflowEngine(db)
    problem = workflow.promote_capture(workflow.capture("Refinement intent is hard to see"))
    workflow.approve_problem(problem["id"])
    solution = workflow.create_feature(
        problem["id"],
        "Show refinement direction",
        "## Intended outcome\nPeople can anticipate the next question and answer with confidence.\n\n"
        "## Scope\nProblem and Solution refinement previews.\n\n"
        "## Trade-offs\nKeep the current casual chat while making gaps visible.",
        non_goals="Do not add Preview to Capture.",
        validation_criteria="- [ ] The next focus is visible before the AI asks",
    )

    summary = workflow.refinement_context_summary("features", solution["id"])

    assert summary["view_mode"] == "structure"
    assert [item["label"] for item in summary["structure"]] == [
        "Solution title", "Problem this supports", "Intended outcome", "Scope", "Non-goals",
        "Evidence & prior context", "Trade-offs & risks", "Dependencies", "Validation criteria", "Open questions",
    ]
    assert next(item for item in summary["structure"] if item["key"] == "scope")["status"] != "missing"
    assert next(item for item in summary["structure"] if item["key"] == "dependencies")["status"] == "missing"


def test_named_but_unknown_detail_is_highlighted_as_thin_not_ready() -> None:
    db = sqlite3.connect(":memory:")
    db.row_factory = sqlite3.Row
    workflow = WorkflowEngine(db)
    problem = workflow.promote_capture(workflow.capture("A dependency decision is unclear"))
    workflow.approve_problem(problem["id"])
    solution = workflow.create_feature(
        problem["id"],
        "Resolve the dependency",
        "## Intended outcome\nPeople can make a supported decision.\n\n"
        "## Dependencies\nOwner: Not yet known. Timing: Not yet known.\n\n"
        "## Open questions\nWhich owner should make the decision?",
        validation_criteria="- [ ] An owner is named",
    )

    summary = workflow.refinement_context_summary("features", solution["id"])
    dependencies = next(item for item in summary["structure"] if item["key"] == "dependencies")
    open_questions = next(item for item in summary["structure"] if item["key"] == "open_questions")
    assert dependencies["status"] == "weak"
    assert open_questions["status"] == "complete"


def test_latest_refinement_draft_is_restored_and_knows_when_it_was_applied() -> None:
    db = sqlite3.connect(":memory:")
    db.row_factory = sqlite3.Row
    workflow = WorkflowEngine(db)
    problem = workflow.promote_capture(workflow.capture("Refinement should survive closing Preview"))
    draft = {
        "title": "Keep the latest refinement visible",
        "detail": "## Context\nThe generated refinement remains available for review.",
    }
    workflow.record_ai_run("problems", problem["id"], "workflow_refinement", "Refine current item", str(draft))

    pending = workflow.refinement_context_summary("problems", problem["id"])["refinement_draft"]
    assert pending == {**draft, "applied": False}

    workflow.update_manual("problems", problem["id"], draft["title"], draft["detail"])
    applied = workflow.refinement_context_summary("problems", problem["id"])["refinement_draft"]
    assert applied == {**draft, "applied": True}


def test_latest_next_solution_draft_is_restored_and_knows_when_it_was_created() -> None:
    db = sqlite3.connect(":memory:")
    db.row_factory = sqlite3.Row
    workflow = WorkflowEngine(db)
    problem = workflow.promote_capture(workflow.capture("A Solution draft should survive closing Preview"))
    workflow.approve_problem(problem["id"])
    draft = {
        "title": "Keep the proposed Solution visible",
        "outcome": "People can review the generated Solution in context.",
        "non_goals": "Do not create it without a human action.",
        "validation_criteria": "- [ ] The Preview reopens with the latest proposal",
    }
    workflow.record_ai_run("problems", problem["id"], "workflow_draft", "Create a reviewed draft", str(draft))

    pending = workflow.refinement_context_summary("problems", problem["id"])["next_draft"]
    assert pending == {**draft, "applied": False}

    workflow.create_feature(problem["id"], **draft)
    created = workflow.refinement_context_summary("problems", problem["id"])["next_draft"]
    assert created == {**draft, "applied": True}


def test_completed_solution_builds_idempotent_evidence_first_lineage() -> None:
    db = sqlite3.connect(":memory:"); db.row_factory = sqlite3.Row
    workflow = WorkflowEngine(db)
    capture = workflow.capture("Keep the original feedback intact")
    problem = workflow.promote_capture(capture, "Refinement loses its origin", "## Desired outcome\nThe origin remains traceable.")
    workflow.approve_problem(problem["id"])
    solution = workflow.create_feature(
        problem["id"], "Preserve lineage", "Completed work explains its origin", "No new workflow stage", "- [ ] Origin is linked"
    )
    workflow.record_conflict_evaluation(solution["id"], "clear", "Human review")
    workflow.approve_feature(solution["id"])
    workflow.update_manual("features", solution["id"], "Preserve final lineage", "Completed work explains every transition")
    workflow.record_completion(solution["id"], "The origin link was inspected", "Human verified the Lineage")
    workflow.db.execute("UPDATE completions SET knowledge_status='integrated' WHERE feature_id=?", (solution["id"],))
    workflow.verify_completion(solution["id"])
    workflow.complete_problem(problem["id"], "All evidence was reviewed")

    first = workflow.create_lineage_snapshot(solution["id"])
    second = workflow.create_lineage_snapshot(solution["id"])

    assert first["snapshot_id"] == second["snapshot_id"]
    assert [stage["kind"] for stage in first["lineage"]["stages"]] == ["capture", "problem", "solution", "complete"]
    assert all(stage.get("occurred_at") for stage in first["lineage"]["stages"])
    assert [stage.get("record_type") for stage in first["lineage"]["stages"][:3]] == ["captures", "problems", "features"]
    assert len(first["lineage"]["transitions"]) == 3
    assert first["lineage"]["transitions"][1]["context_kind"] == "recorded_change"
    transition_claim = first["claims"][first["lineage"]["transitions"][1]["claim_id"]]
    assert "Preserve final lineage" in transition_claim["text"]
    assert transition_claim["text"] != "Not explicitly recorded"
    assert {claim["classification"] for claim in first["claims"].values()} >= {"observed", "decided"}
    assert any(item["event_type"] == "manual_edit" for item in first["decision_changes"])
    assert all(claim["evidence_ids"] for claim in first["claims"].values())


def test_conflict_address_requires_human_or_implementation_basis() -> None:
    db = sqlite3.connect(":memory:"); db.row_factory = sqlite3.Row
    workflow = WorkflowEngine(db)
    problem = workflow.promote_capture(workflow.capture("A requirement conflicts"))
    workflow.approve_problem(problem["id"])
    solution = workflow.create_feature(problem["id"], "Resolve direction", "A supported direction", validation_criteria="- [ ] Direction is supported")
    report_id = workflow.record_conflict_evaluation(solution["id"], "conflicted", "The old requirement differs")

    with pytest.raises(WorkflowError, match="AI inference"):
        workflow.record_conflict_address(
            solution["id"], report_id, "addressed", "ai_inferred", "modified", "Likely changed", "conflict_report", report_id
        )

    address = workflow.record_conflict_address(
        solution["id"], report_id, "addressed", "explicit_decision", "modified", "The user chose Preview context", "conflict_report", report_id
    )
    assert address["status"] == "addressed"
    assert address["disposition"] == "modified"
    conflict = workflow.create_lineage_snapshot(solution["id"])["conflicts"][0]
    assert (conflict["status"], conflict["basis"], conflict["disposition"]) == (
        "addressed", "explicit_decision", "modified"
    )
    workflow.record_conflict_evaluation(solution["id"], "conflicted", "A later requirement is still open")
    refreshed = workflow.create_lineage_snapshot(solution["id"])
    assert refreshed["lineage"]["transitions"][1]["material_conflict"]["status"] == "unaddressed"


def test_lineage_correction_preserves_ai_revision_and_carries_forward() -> None:
    db = sqlite3.connect(":memory:"); db.row_factory = sqlite3.Row
    workflow = WorkflowEngine(db)
    problem = workflow.promote_capture(workflow.capture("Interpret this change"))
    workflow.approve_problem(problem["id"])
    solution = workflow.create_feature(problem["id"], "Interpret safely", "The history is clear", validation_criteria="- [ ] History is cited")
    workflow.record_conflict_evaluation(solution["id"], "clear", "Human review")
    workflow.approve_feature(solution["id"])
    workflow.complete_problem(problem["id"], "Reviewed")
    lineage = workflow.create_lineage_snapshot(solution["id"])
    evidence_id = next(iter(lineage["evidence"]))
    lineage = workflow.add_lineage_inferences(solution["id"], lineage["snapshot_id"], [{
        "claim_key": "inferred:reason", "text": "Likely rationale", "confidence": "medium", "evidence_ids": [evidence_id]
    }])
    inferred = next(claim for claim in lineage["claims"].values() if claim["claim_key"] == "inferred:reason")
    workflow.correct_lineage_claim(solution["id"], inferred["id"], "The context moved into Preview", "User correction", inferred["current_revision_id"])
    corrected = workflow.lineage(solution["id"])["claims"][inferred["id"]]
    assert corrected["text"] == "The context moved into Preview"
    assert [revision["author_type"] for revision in corrected["revisions"]] == ["ai", "user"]

    regenerated = workflow.create_lineage_snapshot(solution["id"], force=True)
    regenerated_evidence = next(iter(regenerated["evidence"]))
    regenerated = workflow.add_lineage_inferences(solution["id"], regenerated["snapshot_id"], [{
        "claim_key": "inferred:reason", "text": "A new AI wording", "confidence": "low", "evidence_ids": [regenerated_evidence]
    }])
    carried = next(claim for claim in regenerated["claims"].values() if claim["claim_key"] == "inferred:reason")
    assert carried["text"] == "The context moved into Preview"
    assert carried["current_author_type"] == "user"


def test_saved_solution_detail_exists_without_its_own_refinement_run() -> None:
    db = sqlite3.connect(":memory:")
    db.row_factory = sqlite3.Row
    workflow = WorkflowEngine(db)
    problem = workflow.promote_capture(workflow.capture("Create a complete Solution"))
    workflow.approve_problem(problem["id"])
    solution = workflow.create_feature(
        problem["id"],
        "A saved Solution",
        "People can see its intended outcome.",
        "Do not hide its boundaries.",
        "- [ ] Detail is visible on first Explore",
    )

    context = workflow.refinement_context_summary("features", solution["id"])

    assert context["refinement_draft"] is None
    assert context["current_detail"] == {
        "kind": "solution",
        "title": "A saved Solution",
        "outcome": "People can see its intended outcome.",
        "non_goals": "Do not hide its boundaries.",
        "validation_criteria": "- [ ] Detail is visible on first Explore",
        "state": "proposed",
        "conflict_state": "unknown",
        "created_at": solution["created_at"],
        "problem_id": problem["id"],
        "problem_statement": problem["statement"],
    }


def test_capture_preview_restores_a_problem_draft_without_promoting() -> None:
    db = sqlite3.connect(":memory:")
    db.row_factory = sqlite3.Row
    workflow = WorkflowEngine(db)
    capture_id = workflow.capture("A raw thought that still needs refinement")
    workflow.record_ai_run(
        "captures",
        capture_id,
        "workflow_draft",
        "Create a reviewed draft",
        "{'title': 'A clear proposed Problem', 'detail': '## Context\\nKnown evidence'}",
    )

    context = workflow.refinement_context_summary("captures", capture_id)

    assert context["current_detail"]["kind"] == "capture"
    assert context["next_draft"] == {
        "title": "A clear proposed Problem",
        "detail": "## Context\nKnown evidence",
        "applied": False,
    }
    assert workflow.db.execute("SELECT COUNT(*) FROM problems").fetchone()[0] == 0


def test_promoted_problem_inherits_capture_conversation_for_preview_and_ai_context() -> None:
    db = sqlite3.connect(":memory:")
    db.row_factory = sqlite3.Row
    workflow = WorkflowEngine(db)
    capture = workflow.capture("A raw queue idea")
    workflow.record_ai_run("captures", capture, "workflow_chat", "Who needs this?", "People submitting AI work requests.")
    workflow.record_ai_run("captures", capture, "workflow_refinement", "Refine current item", "A clearer queue request")
    problem = workflow.promote_capture(capture, "Make AI work requests queueable")

    summary = workflow.refinement_context_summary("problems", problem["id"])

    assert summary["entries"] == [
        {"label": "Current item", "text": "Make AI work requests queueable"},
        {"label": "Earlier Capture refinement", "text": "A clearer queue request"},
        {
            "label": "Earlier Capture discussion",
            "text": "Who needs this? — People submitting AI work requests.",
        },
    ]
    assert summary["view_mode"] == "structure"
    assert workflow.chat_history("problems", problem["id"])[-2:] == [
        {"role": "user", "content": "Who needs this?"},
        {"role": "assistant", "content": "People submitting AI work requests."},
    ]


def test_solution_inherits_problem_and_capture_conversation_lineage() -> None:
    db = sqlite3.connect(":memory:")
    db.row_factory = sqlite3.Row
    workflow = WorkflowEngine(db)
    capture = workflow.capture("A raw decision thought")
    workflow.record_ai_run("captures", capture, "workflow_chat", "Who is affected?", "The whole team.")
    problem = workflow.promote_capture(capture, "Trusted decisions are hard to find", "People repeat old decisions.")
    workflow.record_ai_run("problems", problem["id"], "workflow_chat", "What is the boundary?", "No migration project.")
    workflow.approve_problem(problem["id"])
    solution = workflow.create_feature(
        problem["id"],
        "One decision home",
        "People can find a trusted decision. " * 30,
        validation_criteria="- [ ] A trusted decision is findable",
    )
    workflow.record_ai_run("features", solution["id"], "workflow_refinement", "Refine current item", "Keep the decision home focused.")

    summary = workflow.refinement_context_summary("features", solution["id"])

    assert [entry["label"] for entry in summary["entries"]] == [
        "Current item",
        "Current context",
        "Previous preview",
        "Earlier Problem discussion",
        "Earlier Capture discussion",
    ]
    visible = " ".join(entry["text"] for entry in summary["entries"])
    assert "No migration project" in visible
    assert "whole team" in visible
    assert len(visible) <= 500
    assert workflow.chat_history("features", solution["id"])[-4:] == [
        {"role": "user", "content": "Who is affected?"},
        {"role": "assistant", "content": "The whole team."},
        {"role": "user", "content": "What is the boundary?"},
        {"role": "assistant", "content": "No migration project."},
    ]


def test_only_validation_criteria_bullets_seed_an_editable_checklist_once() -> None:
    db = sqlite3.connect(":memory:"); db.row_factory = sqlite3.Row
    workflow = WorkflowEngine(db)
    problem = workflow.promote_capture(workflow.capture("A problem"))
    workflow.approve_problem(problem["id"])
    feature = workflow.create_feature(problem["id"], "A solution", "- [ ] This outcome bullet must not become a checklist", validation_criteria="- [ ] Review evidence\n- [x] Preserve work log\n- Review evidence")
    progress = workflow.solution_progress(feature["id"])
    assert [(item["body"], item["checked"]) for item in progress["checklist"]] == [("Review evidence", 0), ("Preserve work log", 1)]
    workflow.update_manual("features", feature["id"], "A solution", "- A new outcome bullet")
    assert len(workflow.solution_progress(feature["id"])["checklist"]) == 2


def test_work_log_image_summaries_are_bilingual_atomic_and_leave_authored_evidence_unchanged() -> None:
    db = sqlite3.connect(":memory:"); db.row_factory = sqlite3.Row
    workflow = WorkflowEngine(db)
    problem = workflow.promote_capture(workflow.capture("A problem"))
    workflow.approve_problem(problem["id"])
    feature = workflow.create_feature(problem["id"], "A solution", "An outcome", validation_criteria="- [ ] Done")
    workflow.record_conflict_evaluation(feature["id"], "clear", "Human review")
    workflow.approve_feature(feature["id"])
    entry = workflow.add_solution_progress(feature["id"], "사용자가 쓴 작업 기록", "aGVsbG8=", "image/png")
    versions = {
        "ko": {"image_summary": "한글 이미지 요약"},
        "en": {"image_summary": "English image summary"},
    }

    workflow.set_solution_progress_summaries(entry["id"], versions, "ko")

    korean = workflow.solution_progress(feature["id"], "ko")["entries"][0]
    english = workflow.solution_progress(feature["id"], "en")["entries"][0]
    assert korean["image_summary"] == "한글 이미지 요약"
    assert english["image_summary"] == "English image summary"
    assert korean["body"] == english["body"] == "사용자가 쓴 작업 기록"
    assert english["available_locales"] == ["ko", "en"]
    assert english["fallback_used"] is True

    with pytest.raises(ValueError):
        workflow.set_solution_progress_summaries(
            entry["id"], {"ko": {"image_summary": "덮어쓰면 안 됨"}}, "ko"
        )
    unchanged = workflow.solution_progress(feature["id"], "en")["entries"][0]
    assert unchanged["image_summary"] == "English image summary"


def test_legacy_image_summary_falls_back_without_localization_rows() -> None:
    db = sqlite3.connect(":memory:"); db.row_factory = sqlite3.Row
    workflow = WorkflowEngine(db)
    problem = workflow.promote_capture(workflow.capture("A problem"))
    workflow.approve_problem(problem["id"])
    feature = workflow.create_feature(problem["id"], "A solution", "An outcome", validation_criteria="- [ ] Done")
    workflow.record_conflict_evaluation(feature["id"], "clear", "Human review")
    workflow.approve_feature(feature["id"])
    entry = workflow.add_solution_progress(feature["id"], image_data="aGVsbG8=", image_media_type="image/png")
    workflow.set_solution_progress_summary(entry["id"], "기존 단일 요약")

    english = workflow.solution_progress(feature["id"], "en")["entries"][0]
    assert english["image_summary"] == "기존 단일 요약"
    assert english["content_locale"] == "original"
    assert english["available_locales"] == []
    assert english["fallback_used"] is True


def test_organize_workbench_persists_attention_and_category() -> None:
    db = sqlite3.connect(":memory:"); db.row_factory = sqlite3.Row
    workflow = WorkflowEngine(db)
    capture = workflow.capture("Improve the LLM Wiki workbench")
    problem = workflow.promote_capture(capture, "Improve the LLM Wiki workbench")
    workflow.organize_workbench()
    board = workflow.board()
    assert board["problems"][0]["category"] == "LLM Wiki"
    assert board["problems"][0]["attention_rank"] >= 100
    workflow.set_workbench_category("problems", problem["id"], "Personal")
    assert workflow.board()["problems"][0]["category"] == "Personal"
    workflow.organize_workbench()
    assert workflow.board()["problems"][0]["category"] == "Personal"
    workflow.set_workbench_importance("problems", problem["id"], True)
    assert workflow.board()["problems"][0]["manual_priority"] == 1


def test_new_problem_and_solution_inherit_capture_category() -> None:
    db = sqlite3.connect(":memory:"); db.row_factory = sqlite3.Row
    workflow = WorkflowEngine(db)
    capture = workflow.capture("A categorized thought")
    workflow.set_workbench_category("captures", capture, "Product")

    problem = workflow.promote_capture(capture)
    workflow.approve_problem(problem["id"])
    solution = workflow.create_feature(
        problem["id"],
        "Keep categories aligned",
        "The linked workflow has one category",
        validation_criteria="- [ ] Categories match",
    )

    categories = {
        (row["entity_type"], row["entity_id"]): row["category"]
        for row in db.execute("SELECT entity_type,entity_id,category FROM workbench_priorities")
    }
    assert categories[("captures", capture)] == "Product"
    assert categories[("problems", problem["id"])] == "Product"
    assert categories[("features", solution["id"])] == "Product"


def test_explicit_general_survives_transitions_until_linked_item_is_dragged() -> None:
    db = sqlite3.connect(":memory:"); db.row_factory = sqlite3.Row
    workflow = WorkflowEngine(db)
    capture = workflow.capture("LLM Wiki category heuristic would disagree")
    workflow.set_workbench_category("captures", capture, "General")
    problem = workflow.promote_capture(capture)
    workflow.approve_problem(problem["id"])
    solution = workflow.create_feature(
        problem["id"],
        "A linked Solution",
        "The explicit category is preserved",
        validation_criteria="- [ ] Categories match",
    )
    sibling_solution = workflow.create_feature(
        problem["id"],
        "Another linked Solution",
        "Every Solution in the lineage moves together",
        validation_criteria="- [ ] Categories match",
    )

    assert workflow.board()["problems"][0]["category"] == "General"
    assert {item["category"] for item in workflow.board()["features"]} == {"General"}

    workflow.set_workbench_category("features", solution["id"], "Engineering")

    categories = {
        (row["entity_type"], row["entity_id"]): row["category"]
        for row in db.execute("SELECT entity_type,entity_id,category FROM workbench_priorities")
    }
    assert categories[("captures", capture)] == "Engineering"
    assert categories[("problems", problem["id"])] == "Engineering"
    assert categories[("features", solution["id"])] == "Engineering"
    assert categories[("features", sibling_solution["id"])] == "Engineering"
    overrides = {
        (row["entity_type"], row["entity_id"]): row["category"]
        for row in db.execute("SELECT entity_type,entity_id,category FROM workbench_category_overrides")
    }
    assert set(overrides.values()) == {"Engineering"}
