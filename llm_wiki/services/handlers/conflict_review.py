from __future__ import annotations

import asyncio
import hashlib
import json
from pathlib import Path
from typing import Any

from llm_wiki.adapters.provider import AsyncOpenAICompatibleProvider
from llm_wiki.services.handlers.registry import HandlerContext, HandlerRegistry
from llm_wiki.services.jobs import StaleJobError, TaskDescriptor
from llm_wiki.services.localization import response_language_instruction
from llm_wiki.services.retrieval import RetrievalEngine
from llm_wiki.services.settings import ProviderSettings
from llm_wiki.services.patches import digest
from llm_wiki.services.workflow import WorkflowEngine, WorkflowError


STATE_MEANINGS = {
    "potential_conflict": "At least one evidence-backed potential conflict was found.",
    "clear": "All retained candidates were reviewed with adequate coverage and no conflict was found.",
    "insufficient_evidence": "Search coverage, candidates, model output, or citations cannot support clear.",
}


def _hash(value: object) -> str:
    return hashlib.sha256(json.dumps(value, ensure_ascii=False, sort_keys=True).encode()).hexdigest()


def _claims(feature: dict[str, object], problem: dict[str, object]) -> list[dict[str, str]]:
    fields = (("scope", feature.get("title")), ("requirement", feature.get("outcome")), ("non_goal", feature.get("non_goals")), ("validation", feature.get("validation_criteria")), ("constraint", problem.get("statement")))
    result: list[dict[str, str]] = []
    for kind, raw in fields:
        for text in str(raw or "").splitlines():
            text = text.strip().lstrip("-*[] ")
            if text:
                result.append({"id": f"claim-{len(result) + 1}", "kind": kind, "text": text[:2000]})
    return result


def _search(retrieval: RetrievalEngine, claims: list[dict[str, str]]) -> list[dict[str, object]]:
    merged: dict[tuple[str, str], dict[str, object]] = {}
    for claim in claims:
        candidates = retrieval.search(claim["text"], limit=8)
        try:
            candidates += retrieval.semantic_search(claim["text"], limit=8)
        except (ImportError, RuntimeError, ValueError):
            pass
        for result in candidates:
            key = (claim["id"], str(Path(result.path)).lower().replace(".raw.md", ".md"))
            if key in merged:
                continue
            try:
                passage = retrieval.best_passage(result.path, claim["text"])
            except KeyError:
                continue
            merged[key] = {**passage, "id": f"evidence-{len(merged) + 1}", "claim_id": claim["id"], "claim": claim["text"], "score": result.score, "matched_by": list(result.matched_by)}
    return sorted(merged.values(), key=lambda item: float(item["score"]), reverse=True)[:12]


def conflict_source_hash(workflow: WorkflowEngine, retrieval: RetrievalEngine, feature_id: str, locale: str) -> str:
    board = workflow.board(locale)
    feature = next((item for item in board["features"] if item["id"] == feature_id), None)
    if not feature:
        raise WorkflowError("Solution not found")
    problem = next((item for item in board["problems"] if item["id"] == feature["problem_id"]), {})
    return digest(json.dumps({"feature": feature, "problem": problem}, sort_keys=True, ensure_ascii=False) + retrieval.manifest_hash())


class ConflictReviewJobHandler:
    def __init__(self, retrieval: RetrievalEngine, workflow: WorkflowEngine, settings: ProviderSettings):
        self.retrieval = retrieval
        self.workflow = workflow
        self.settings = settings

    def register(self, registry: HandlerRegistry) -> None:
        registry.register(TaskDescriptor("conflict_review", result_interface="conflict_review"), self.__call__)

    async def __call__(self, context: HandlerContext) -> dict[str, Any]:
        feature_id = str(context.payload.get("entity_id", ""))
        locale = str(context.payload.get("locale", "en"))
        board = self.workflow.board(locale)
        feature = next((item for item in board["features"] if item["id"] == feature_id), None)
        if not feature:
            raise WorkflowError("Solution not found")
        problem = next((item for item in board["problems"] if item["id"] == feature["problem_id"]), {})
        if context.source_hash != conflict_source_hash(self.workflow, self.retrieval, feature_id, locale):
            raise StaleJobError("Conflict review source changed before execution")
        claims = _claims(feature, problem)
        query = json.dumps({"solution_hash": _hash({"feature": feature, "problem": problem}), "vault_hash": self.retrieval.manifest_hash()}, sort_keys=True)
        cached = self.workflow.cached_conflict_review(query)
        if cached:
            return {**cached, "cached": True}
        candidates = await asyncio.to_thread(_search, self.retrieval, claims)
        scope = self.retrieval.status()
        coverage = round(scope["semantic_ready"] / scope["documents"], 4) if scope["documents"] else 0.0
        base_url, api_key, weak_model = self.settings.credentials(None)
        provider = AsyncOpenAICompatibleProvider.with_client(base_url, api_key, weak_model)
        retained = candidates
        try:
            if candidates:
                response = await provider.complete_json([
                    {"role": "system", "content": "Screen all evidence candidates. Return JSON {decisions:[{evidence_id,disposition:'non_conflict|retain',reason}]}. Exclude only explicit evidence-grounded non-conflicts; retain uncertainty."},
                    {"role": "system", "content": response_language_instruction(locale)},
                    {"role": "user", "content": json.dumps({"solution": feature, "problem": problem, "claims": claims, "evidence": candidates}, ensure_ascii=False)},
                ], "conflict screen")
                decisions = response.get("decisions") if isinstance(response.get("decisions"), list) else []
                by_id = {str(item.get("evidence_id")): item for item in decisions if isinstance(item, dict)}
                retained = [item for item in candidates if not (item["id"] in by_id and by_id[item["id"]].get("disposition") == "non_conflict" and str(by_id[item["id"]].get("reason", "")).strip())]
        finally:
            await provider.aclose()
        base_url, api_key, strong_model = self.settings.credentials("conflict_review")
        provider = AsyncOpenAICompatibleProvider.with_client(base_url, api_key, strong_model)
        findings: list[dict[str, object]] = []
        try:
            for index, evidence in enumerate(retained):
                if await context.cancelled():
                    raise InterruptedError("Conflict review cancelled")
                response = await provider.complete_json([
                    {"role": "system", "content": "Review exactly one Vault passage against its Solution claim. Return JSON {conflict:boolean,claim,severity:'low|medium|high',evidence_id,explanation,required_resolution}. Never invent a citation."},
                    {"role": "system", "content": response_language_instruction(locale)},
                    {"role": "user", "content": json.dumps({"solution": feature, "problem": problem, "evidence": evidence}, ensure_ascii=False)},
                ], "conflict evidence review")
                if response.get("conflict") is True and str(response.get("evidence_id")) == evidence["id"] and str(response.get("explanation", "")).strip():
                    findings.append({"claim": str(response.get("claim") or evidence["claim"]), "severity": str(response.get("severity", "medium")), "evidence_id": evidence["id"], "path": evidence["path"], "source_hash": evidence["source_hash"], "start_line": evidence["start_line"], "end_line": evidence["end_line"], "excerpt": evidence["text"], "citation": f"{evidence['path']}:{evidence['start_line']}-{evidence['end_line']}", "explanation": str(response["explanation"]), "required_resolution": str(response.get("required_resolution", "Review manually"))})
                await context.progress(index + 1, len(retained))
        finally:
            await provider.aclose()
        sufficient = bool(candidates) and coverage >= 1.0
        state = "potential_conflict" if findings else ("clear" if sufficient else "insufficient_evidence")
        if context.source_hash != conflict_source_hash(self.workflow, self.retrieval, feature_id, locale):
            raise StaleJobError("Conflict review source changed during execution")
        run_id = self.workflow.start_conflict_review(feature_id, query)
        report = {"run_id": run_id, "feature_id": feature_id, "status": "ready", "phase": "complete", "recommended_state": state, "state_meanings": STATE_MEANINGS, "scope": {**scope, "embedding_coverage": coverage}, "claims": claims, "candidate_count": len(candidates), "retained_count": len(retained), "reviewed_count": len(retained), "progress": 1.0, "findings": findings, "candidates": candidates, "summary": STATE_MEANINGS[state]}
        self.workflow.finish_conflict_review(run_id, candidates, report)
        return report
