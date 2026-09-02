import sqlite3
from time import perf_counter

from llm_wiki.services.workflow import WorkflowEngine


def test_lineage_assembly_and_reads_remain_bounded_for_twenty_decisions() -> None:
    db = sqlite3.connect(":memory:")
    db.row_factory = sqlite3.Row
    workflow = WorkflowEngine(db)
    problem = workflow.promote_capture(workflow.capture("Preserve the origin of completed work"))
    workflow.approve_problem(problem["id"])
    feature = workflow.create_feature(
        problem["id"],
        "Traceable completion",
        "A reader can follow decisions to their source",
        validation_criteria="- [x] Lineage remains readable",
    )
    for index in range(20):
        workflow.update_manual(
            "features",
            feature["id"],
            f"Traceable completion {index}",
            f"Decision {index} remains linked",
        )
    workflow.record_conflict_evaluation(feature["id"], "clear", "Recorded evaluation")
    workflow.approve_feature(feature["id"])
    workflow.record_completion(feature["id"], "Tests passed", "Human reviewed")

    started = perf_counter()
    snapshot = workflow.create_lineage_snapshot(feature["id"])
    for _ in range(20):
        current = workflow.lineage(feature["id"])
    elapsed = perf_counter() - started

    assert snapshot["snapshot_id"] == current["snapshot_id"]
    assert len(snapshot["decision_changes"]) == 20
    assert elapsed < 1.0
