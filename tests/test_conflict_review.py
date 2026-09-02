import time

from llm_wiki.core.models import SearchResult
from llm_wiki.services.conflict_review import ConflictReviewManager, extract_claims


class FakeWorkflow:
    def __init__(self):
        self.cached = None
        self.finished = []

    def cached_conflict_review(self, _query):
        return self.cached

    def start_conflict_review(self, _feature_id, _query):
        return "run-1"

    def finish_conflict_review(self, run_id, candidates, report, error=""):
        self.finished.append((run_id, candidates, report, error))

    def cancel_conflict_review(self, *_args):
        pass


class FakeRetrieval:
    def __init__(self, coverage=1.0, candidates=True):
        self.coverage = coverage
        self.has_candidates = candidates
        self.semantic_batches = 0

    def manifest_hash(self):
        return "vault-hash"

    def status(self):
        return {"documents": 1, "semantic_ready": int(self.coverage)}

    def search(self, _query, limit=8):
        if not self.has_candidates:
            return []
        return [SearchResult("decision.md", "Decision", "Old scope", (), (), 2.0, "source-hash", ("content",))]

    def semantic_search(self, _query, limit=8):
        return []

    def semantic_search_many(self, queries, limit=8):
        self.semantic_batches += 1
        return [[] for _ in queries]

    def best_passage(self, path, _query):
        return {"path": path, "source_hash": "source-hash", "start_line": 3, "end_line": 4, "text": "Old scope\nMust stay local"}


class FakeProvider:
    def __init__(self, strong=False, malformed_screen=False, delay=0):
        self.strong, self.malformed_screen, self.delay = strong, malformed_screen, delay

    def complete_json(self, messages, _name, cancel_event=None):
        if self.delay:
            time.sleep(self.delay)
        if cancel_event and cancel_event.is_set():
            raise InterruptedError("cancelled")
        if not self.strong:
            return {} if self.malformed_screen else {"decisions": [{"evidence_id": "evidence-1", "disposition": "retain", "reason": "possible conflict"}]}
        evidence = __import__("json").loads(messages[-1]["content"])["evidence"]
        return {"conflict": True, "claim": evidence["claim"], "severity": "high", "evidence_id": evidence["id"],
                "explanation": "The required scope differs.", "required_resolution": "Choose one scope."}


def wait_ready(manager):
    for _ in range(100):
        result = manager.get("run-1")
        if result and result["status"] != "running":
            return result
        time.sleep(0.01)
    raise AssertionError("review did not finish")


def test_extracts_reviewable_solution_claims():
    claims = extract_claims({"title": "Local search", "outcome": "Search all notes", "non_goals": "No cloud",
                             "validation_criteria": "- [ ] Exact citations"}, {"statement": "Avoid missing decisions"})
    assert {claim["kind"] for claim in claims} == {"scope", "requirement", "non_goal", "validation", "constraint"}


def test_publishes_exact_evidence_backed_finding():
    workflow = FakeWorkflow()
    retrieval = FakeRetrieval()
    manager = ConflictReviewManager(retrieval, workflow, lambda strong: FakeProvider(strong))
    started = manager.start({"id": "solution-1", "title": "New", "outcome": "Cloud", "non_goals": "", "validation_criteria": "Done"}, {"statement": "Scope"}, "English")
    result = wait_ready(manager)
    assert started["recommended_state"] == "reviewing"
    assert result["recommended_state"] == "potential_conflict"
    assert result["findings"][0]["citation"] == "decision.md:3-4"
    assert result["findings"][0]["excerpt"] == "Old scope\nMust stay local"
    assert set(result["timings_ms"]) == {"search", "screen", "review"}
    assert retrieval.semantic_batches == 1


def test_stale_candidate_removed_during_search_is_skipped():
    class StaleRetrieval(FakeRetrieval):
        def best_passage(self, path, query):
            raise KeyError(path)

    manager = ConflictReviewManager(StaleRetrieval(), FakeWorkflow(), lambda strong: FakeProvider(strong))
    manager.start({"id": "solution-1", "title": "New", "outcome": "Cloud", "non_goals": "", "validation_criteria": "Done"}, {"statement": "Scope"}, "English")
    result = wait_ready(manager)
    assert result["status"] == "ready"
    assert result["recommended_state"] == "insufficient_evidence"
    assert result["candidate_count"] == 0


def test_zero_candidates_and_incomplete_coverage_never_clear():
    for retrieval in (FakeRetrieval(candidates=False), FakeRetrieval(coverage=0.0)):
        manager = ConflictReviewManager(retrieval, FakeWorkflow(), lambda strong: FakeProvider(strong))
        manager.start({"id": "solution-1", "title": "New", "outcome": "Cloud", "non_goals": "", "validation_criteria": "Done"}, {"statement": "Scope"}, "English")
        assert wait_ready(manager)["recommended_state"] != "clear"


def test_malformed_screen_does_not_exclude_candidate():
    manager = ConflictReviewManager(FakeRetrieval(), FakeWorkflow(), lambda strong: FakeProvider(strong, malformed_screen=True))
    manager.start({"id": "solution-1", "title": "New", "outcome": "Cloud", "non_goals": "", "validation_criteria": "Done"}, {"statement": "Scope"}, "English")
    result = wait_ready(manager)
    assert result["retained_count"] == result["candidate_count"]
    assert result["reviewed_count"] == result["retained_count"]


def test_cancel_stops_before_later_provider_calls():
    calls = []
    def factory(strong):
        calls.append(strong)
        return FakeProvider(strong, delay=0.05)
    manager = ConflictReviewManager(FakeRetrieval(), FakeWorkflow(), factory)
    manager.start({"id": "solution-1", "title": "New", "outcome": "Cloud", "non_goals": "", "validation_criteria": "Done"}, {"statement": "Scope"}, "English")
    manager.cancel("run-1")
    result = wait_ready(manager)
    assert result["recommended_state"] == "cancelled"
    assert True not in calls


def test_identical_solution_and_vault_hashes_reuse_completed_snapshot():
    workflow = FakeWorkflow()
    workflow.cached = {"run_id": "old-run", "status": "ready", "recommended_state": "clear"}
    manager = ConflictReviewManager(FakeRetrieval(), workflow, lambda _strong: (_ for _ in ()).throw(AssertionError("provider must not run")))
    result = manager.start({"id": "solution-1", "title": "New", "outcome": "Cloud", "non_goals": "", "validation_criteria": "Done"}, {"statement": "Scope"}, "English")
    assert result == {"run_id": "old-run", "status": "ready", "recommended_state": "clear", "cached": True}
