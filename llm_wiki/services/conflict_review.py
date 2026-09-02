from __future__ import annotations

import copy
import hashlib
import json
import threading
import time
from collections.abc import Callable
from pathlib import Path

from llm_wiki.services.retrieval import RetrievalEngine


STATE_MEANINGS = {
    "reviewing": "Candidate review is still in progress; clear is not available.",
    "potential_conflict": "At least one evidence-backed potential conflict was found.",
    "no_conflict_found": "No conflict has been found yet, but the evidence does not justify clear.",
    "clear": "All retained candidates were reviewed with adequate coverage and no conflict was found.",
    "insufficient_evidence": "Search coverage, candidates, model output, or citations cannot support clear.",
    "cancelled": "The review was cancelled before completion.",
    "failed": "The review failed before a reliable recommendation was available.",
}


def _hash(value: object) -> str:
    return hashlib.sha256(json.dumps(value, ensure_ascii=False, sort_keys=True).encode()).hexdigest()


def extract_claims(feature: dict[str, object], problem: dict[str, object]) -> list[dict[str, str]]:
    fields = (
        ("scope", feature.get("title")), ("requirement", feature.get("outcome")),
        ("non_goal", feature.get("non_goals")), ("validation", feature.get("validation_criteria")),
        ("constraint", problem.get("statement")),
    )
    claims: list[dict[str, str]] = []
    for kind, raw in fields:
        for text in str(raw or "").splitlines():
            text = text.strip().lstrip("-*[] ")
            if text:
                claims.append({"id": f"claim-{len(claims) + 1}", "kind": kind, "text": text[:2000]})
    return claims


def _canonical_path(path: str) -> str:
    value = Path(path)
    stem = value.stem.lower()
    for suffix in (".raw", "-raw", "_raw"):
        if stem.endswith(suffix):
            stem = stem[:-len(suffix)]
    return str(value.with_name(stem + value.suffix)).lower()


class ConflictReviewManager:
    def __init__(self, retrieval: RetrievalEngine, workflow: object, provider_factory: Callable[[bool], object]):
        self.retrieval = retrieval
        self.workflow = workflow
        self.provider_factory = provider_factory
        self._lock = threading.Lock()
        self._runs: dict[str, dict[str, object]] = {}
        self._cancel: dict[str, threading.Event] = {}

    def start(self, feature: dict[str, object], problem: dict[str, object], locale_instruction: str) -> dict[str, object]:
        claims = extract_claims(feature, problem)
        solution_hash, vault_hash = _hash({"feature": feature, "problem": problem}), self.retrieval.manifest_hash()
        cache_query = json.dumps({"solution_hash": solution_hash, "vault_hash": vault_hash}, sort_keys=True)
        cached = self.workflow.cached_conflict_review(cache_query)
        if cached:
            cached["cached"] = True
            return cached
        run_id = self.workflow.start_conflict_review(str(feature["id"]), cache_query)
        scope = self.retrieval.status()
        snapshot: dict[str, object] = {
            "run_id": run_id, "feature_id": feature["id"], "status": "running", "phase": "search",
            "recommended_state": "reviewing", "state_meanings": STATE_MEANINGS, "cached": False,
            "scope": {**scope, "embedding_coverage": round(scope["semantic_ready"] / scope["documents"], 4) if scope["documents"] else 0.0},
            "claims": claims, "candidate_count": 0, "screened_count": 0, "retained_count": 0,
            "reviewed_count": 0, "progress": 0.0, "findings": [], "candidates": [],
            "timings_ms": {"search": 0.0, "screen": 0.0, "review": 0.0}, "summary": "Searching Vault evidence.",
        }
        event = threading.Event()
        with self._lock:
            self._runs[run_id], self._cancel[run_id] = snapshot, event
        initial = copy.deepcopy(snapshot)
        threading.Thread(target=self._run, args=(run_id, feature, problem, locale_instruction, event),
                         name=f"conflict-review-{run_id[:8]}", daemon=True).start()
        return initial

    def get(self, run_id: str) -> dict[str, object] | None:
        with self._lock:
            value = self._runs.get(run_id)
            return copy.deepcopy(value) if value else None

    def cancel(self, run_id: str) -> dict[str, object] | None:
        with self._lock:
            event, snapshot = self._cancel.get(run_id), self._runs.get(run_id)
            if not snapshot:
                return None
            if event:
                event.set()
            if snapshot["status"] == "running":
                snapshot.update(status="cancelled", phase="cancelled", recommended_state="cancelled",
                                summary=STATE_MEANINGS["cancelled"])
            return copy.deepcopy(snapshot)

    def _update(self, run_id: str, **values: object) -> None:
        with self._lock:
            self._runs[run_id].update(values)

    @staticmethod
    def _check_cancel(event: threading.Event) -> None:
        if event.is_set():
            raise InterruptedError("Conflict review cancelled")

    def _search(self, claims: list[dict[str, str]], event: threading.Event) -> list[dict[str, object]]:
        merged: dict[tuple[str, str], dict[str, object]] = {}
        canonical_seen: dict[tuple[str, str], str] = {}
        for claim in claims:
            self._check_cancel(event)
            lexical = self.retrieval.search(claim["text"], limit=8)
            try:
                semantic = self.retrieval.semantic_search(claim["text"], limit=8)
            except (ImportError, RuntimeError, ValueError):
                semantic = []
            for result in [*lexical, *semantic]:
                key = (claim["id"], result.path)
                passage = self.retrieval.best_passage(result.path, claim["text"])
                canonical = (claim["id"], _canonical_path(result.path))
                existing_path = canonical_seen.get(canonical)
                if existing_path and "raw" in result.path.lower():
                    continue
                if existing_path and "raw" in existing_path.lower():
                    merged.pop((claim["id"], existing_path), None)
                canonical_seen[canonical] = result.path
                evidence = {**passage, "id": f"evidence-{len(merged) + 1}", "claim_id": claim["id"],
                            "claim": claim["text"], "score": result.score, "matched_by": list(result.matched_by)}
                if key in merged:
                    merged[key]["matched_by"] = sorted(set(merged[key]["matched_by"] + evidence["matched_by"]))
                    merged[key]["score"] = max(float(merged[key]["score"]), float(evidence["score"]))
                else:
                    merged[key] = evidence
        return sorted(merged.values(), key=lambda item: float(item["score"]), reverse=True)[:12]

    def _run(self, run_id: str, feature: dict[str, object], problem: dict[str, object], locale_instruction: str,
             event: threading.Event) -> None:
        snapshot = self.get(run_id) or {}
        claims = list(snapshot.get("claims", []))
        candidates: list[dict[str, object]] = []
        try:
            started = time.perf_counter()
            candidates = self._search(claims, event)
            search_ms = round((time.perf_counter() - started) * 1000, 2)
            self._update(run_id, candidates=candidates, candidate_count=len(candidates), phase="screen",
                         summary="Screening candidate passages.", timings_ms={"search": search_ms, "screen": 0.0, "review": 0.0})
            self._check_cancel(event)
            retained = candidates
            screen_started = time.perf_counter()
            if candidates:
                response = self.provider_factory(False).complete_json([
                    {"role": "system", "content": "Screen all evidence candidates. Return JSON {decisions:[{evidence_id,disposition:'non_conflict|retain',reason}]}. Exclude only an explicit, evidence-grounded non_conflict; retain uncertainty or incomplete evidence."},
                    {"role": "system", "content": locale_instruction},
                    {"role": "user", "content": json.dumps({"solution": feature, "problem": problem, "claims": claims, "evidence": candidates}, ensure_ascii=False)},
                ], "conflict screen", cancel_event=event)
                decisions = response.get("decisions") if isinstance(response.get("decisions"), list) else []
                by_id = {str(item.get("evidence_id")): item for item in decisions if isinstance(item, dict)}
                retained = [item for item in candidates if not (
                    item["id"] in by_id and by_id[item["id"]].get("disposition") == "non_conflict"
                    and str(by_id[item["id"]].get("reason", "")).strip()
                )]
            screen_ms = round((time.perf_counter() - screen_started) * 1000, 2)
            self._update(run_id, screened_count=len(candidates), retained_count=len(retained), phase="review",
                         summary="Reviewing retained evidence.", timings_ms={"search": search_ms, "screen": screen_ms, "review": 0.0})
            findings: list[dict[str, object]] = []
            review_started = time.perf_counter()
            for index, evidence in enumerate(retained):
                self._check_cancel(event)
                response = self.provider_factory(True).complete_json([
                    {"role": "system", "content": "Review exactly one supplied Vault passage against its Solution claim. Return JSON {conflict:boolean,claim,severity:'low|medium|high',evidence_id,explanation,required_resolution}. Never invent a citation."},
                    {"role": "system", "content": locale_instruction},
                    {"role": "user", "content": json.dumps({"solution": feature, "problem": problem, "evidence": evidence}, ensure_ascii=False)},
                ], "conflict evidence review", cancel_event=event)
                if response.get("conflict") is True and str(response.get("evidence_id")) == evidence["id"] and str(response.get("explanation", "")).strip():
                    findings.append({
                        "claim": str(response.get("claim") or evidence["claim"]), "severity": str(response.get("severity", "medium")),
                        "evidence_id": evidence["id"], "path": evidence["path"], "source_hash": evidence["source_hash"],
                        "start_line": evidence["start_line"], "end_line": evidence["end_line"], "excerpt": evidence["text"],
                        "citation": f"{evidence['path']}:{evidence['start_line']}-{evidence['end_line']}",
                        "explanation": str(response["explanation"]), "required_resolution": str(response.get("required_resolution", "Review manually")),
                    })
                reviewed = index + 1
                self._update(run_id, findings=findings, reviewed_count=reviewed,
                             progress=round(reviewed / len(retained), 4) if retained else 1.0,
                             recommended_state="potential_conflict" if findings else "no_conflict_found")
            review_ms = round((time.perf_counter() - review_started) * 1000, 2)
            scope = snapshot.get("scope", {})
            sufficient = bool(candidates) and float(scope.get("embedding_coverage", 0)) >= 1.0 and len(retained) == int((self.get(run_id) or {}).get("reviewed_count", 0))
            state = "potential_conflict" if findings else ("clear" if sufficient else "insufficient_evidence")
            summary = STATE_MEANINGS[state]
            timings = {"search": search_ms, "screen": screen_ms, "review": review_ms}
            self._update(run_id, status="ready", phase="complete", recommended_state=state, summary=summary,
                         progress=1.0, findings=findings, timings_ms=timings)
            final = self.get(run_id) or {}
            self.workflow.finish_conflict_review(run_id, candidates, final)
        except InterruptedError:
            self.cancel(run_id)
            self.workflow.cancel_conflict_review(run_id, self.get(run_id) or {})
        except Exception as error:
            current = self.get(run_id) or {}
            state = "potential_conflict" if current.get("findings") else "insufficient_evidence"
            self._update(run_id, status="failed", phase="failed", recommended_state=state,
                         summary=STATE_MEANINGS[state], error=str(error))
            self.workflow.finish_conflict_review(run_id, candidates, self.get(run_id) or {}, str(error))
