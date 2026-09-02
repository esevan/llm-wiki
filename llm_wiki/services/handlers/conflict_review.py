from __future__ import annotations

import asyncio
import hashlib
import json
from pathlib import Path
from typing import Any

from llm_wiki.adapters.provider import AsyncOpenAICompatibleProvider
from llm_wiki.core.jobs import StaleJobError, TaskDescriptor
from llm_wiki.services.handlers.registry import HandlerContext, HandlerRegistry
from llm_wiki.services.localization import response_language_instruction
from llm_wiki.services.patches import digest
from llm_wiki.services.retrieval import RetrievalEngine
from llm_wiki.services.settings import ProviderSettings
from llm_wiki.services.workflow import WorkflowEngine, WorkflowError

STATE_MEANINGS = {
    "potential_conflict": "At least one evidence-backed potential conflict was found.",
    "clear": "All retained candidates were reviewed with adequate coverage and no conflict was found.",
    "insufficient_evidence": "Search coverage, candidates, model output, or citations cannot support clear.",
}


def _hash(value: object) -> str:
    return hashlib.sha256(json.dumps(value, ensure_ascii=False, sort_keys=True).encode()).hexdigest()


def _response_text(response: dict[str, Any], key: str, fallback: object) -> str:
    return str(response.get(key) or fallback).strip()


def _severity(value: object) -> str:
    normalized = str(value or "medium").lower()
    return normalized if normalized in {"low", "medium", "high"} else "medium"


def _claims(feature: dict[str, object], problem: dict[str, object]) -> list[dict[str, str]]:
    fields = (
        ("scope", feature.get("title")),
        ("requirement", feature.get("outcome")),
        ("non_goal", feature.get("non_goals")),
        ("validation", feature.get("validation_criteria")),
        ("constraint", problem.get("statement")),
    )
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
            merged[key] = {
                **passage,
                "id": f"evidence-{len(merged) + 1}",
                "claim_id": claim["id"],
                "claim": claim["text"],
                "score": result.score,
                "matched_by": list(result.matched_by),
            }
    return sorted(merged.values(), key=lambda item: float(item["score"]), reverse=True)[:12]


def conflict_source_hash(workflow: WorkflowEngine, retrieval: RetrievalEngine, feature_id: str, locale: str) -> str:
    board = workflow.board(locale)
    feature = next((item for item in board["features"] if item["id"] == feature_id), None)
    if not feature:
        raise WorkflowError("Solution not found")
    problem = next((item for item in board["problems"] if item["id"] == feature["problem_id"]), {})
    return digest(
        json.dumps({"feature": feature, "problem": problem}, sort_keys=True, ensure_ascii=False)
        + retrieval.manifest_hash()
    )


def conflict_review_query(workflow: WorkflowEngine, retrieval: RetrievalEngine, feature_id: str, locale: str) -> str:
    board = workflow.board(locale)
    feature = next((item for item in board["features"] if item["id"] == feature_id), None)
    if not feature:
        raise WorkflowError("Solution not found")
    problem = next((item for item in board["problems"] if item["id"] == feature["problem_id"]), {})
    return json.dumps(
        {
            "solution_hash": _hash({"feature": feature, "problem": problem}),
            "vault_hash": retrieval.manifest_hash(),
        },
        sort_keys=True,
    )


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
        feature, problem = self._feature_context(feature_id, locale)
        self._require_current_source(context, feature_id, locale, "before execution")
        claims = _claims(feature, problem)
        query = self._query(feature, problem)
        cached = self.workflow.cached_conflict_review(query)
        if cached:
            return {**cached, "cached": True}
        candidates = await asyncio.to_thread(_search, self.retrieval, claims)
        scope = self.retrieval.status()
        coverage = self._coverage(scope)
        retained = await self._screen_candidates(feature, problem, claims, candidates, locale)
        findings = await self._review_candidates(context, feature, problem, retained, locale)
        state = self._recommended_state(findings, candidates, coverage)
        self._require_current_source(context, feature_id, locale, "during execution")
        report = self._report(
            feature_id,
            query,
            scope,
            coverage,
            claims,
            candidates,
            retained,
            findings,
            state,
        )
        self.workflow.finish_conflict_review(report["run_id"], candidates, report)
        return report

    def _feature_context(self, feature_id: str, locale: str) -> tuple[dict[str, object], dict[str, object]]:
        board = self.workflow.board(locale)
        feature = next((item for item in board["features"] if item["id"] == feature_id), None)
        if not feature:
            raise WorkflowError("Solution not found")
        problem = next((item for item in board["problems"] if item["id"] == feature["problem_id"]), {})
        return feature, problem

    def _require_current_source(
        self,
        context: HandlerContext,
        feature_id: str,
        locale: str,
        phase: str,
    ) -> None:
        current = conflict_source_hash(self.workflow, self.retrieval, feature_id, locale)
        if context.source_hash != current:
            raise StaleJobError(f"Conflict review source changed {phase}")

    def _query(self, feature: dict[str, object], problem: dict[str, object]) -> str:
        return json.dumps(
            {"solution_hash": _hash({"feature": feature, "problem": problem}), "vault_hash": self.retrieval.manifest_hash()},
            sort_keys=True,
        )

    @staticmethod
    def _coverage(scope: dict[str, object]) -> float:
        documents = int(scope["documents"])
        return round(int(scope["semantic_ready"]) / documents, 4) if documents else 0.0

    async def _screen_candidates(
        self,
        feature: dict[str, object],
        problem: dict[str, object],
        claims: list[dict[str, str]],
        candidates: list[dict[str, object]],
        locale: str,
    ) -> list[dict[str, object]]:
        if not candidates:
            return []
        base_url, api_key, weak_model = self.settings.credentials(None)
        provider = AsyncOpenAICompatibleProvider.with_client(base_url, api_key, weak_model)
        try:
            response = await provider.complete_json(
                [
                    {
                        "role": "system",
                        "content": (
                            "Screen all evidence candidates. Return JSON "
                            "{decisions:[{evidence_id,disposition:'non_conflict|retain',reason}]}. "
                            "Exclude only explicit evidence-grounded non-conflicts; retain uncertainty."
                        ),
                    },
                    {"role": "system", "content": response_language_instruction(locale)},
                    {
                        "role": "user",
                        "content": json.dumps(
                            {"solution": feature, "problem": problem, "claims": claims, "evidence": candidates},
                            ensure_ascii=False,
                        ),
                    },
                ],
                "conflict screen",
            )
            return self._retained_candidates(candidates, response)
        finally:
            await provider.aclose()

    @staticmethod
    def _retained_candidates(
        candidates: list[dict[str, object]],
        response: dict[str, Any],
    ) -> list[dict[str, object]]:
        raw_decisions = response.get("decisions")
        decisions = raw_decisions if isinstance(raw_decisions, list) else []
        by_id = {str(item.get("evidence_id")): item for item in decisions if isinstance(item, dict)}
        return [item for item in candidates if not ConflictReviewJobHandler._is_explicit_non_conflict(item, by_id)]

    @staticmethod
    def _is_explicit_non_conflict(
        evidence: dict[str, object],
        decisions: dict[str, dict[str, object]],
    ) -> bool:
        decision = decisions.get(str(evidence["id"]))
        return bool(
            decision and decision.get("disposition") == "non_conflict" and str(decision.get("reason", "")).strip()
        )

    async def _review_candidates(
        self,
        context: HandlerContext,
        feature: dict[str, object],
        problem: dict[str, object],
        retained: list[dict[str, object]],
        locale: str,
    ) -> list[dict[str, object]]:
        base_url, api_key, strong_model = self.settings.credentials("conflict_review")
        provider = AsyncOpenAICompatibleProvider.with_client(base_url, api_key, strong_model)
        findings: list[dict[str, object]] = []
        try:
            for index, evidence in enumerate(retained):
                if await context.cancelled():
                    raise InterruptedError("Conflict review cancelled")
                finding = await self._review_evidence(provider, feature, problem, evidence, locale, index + 1)
                if finding:
                    findings.append(finding)
                await context.progress(index + 1, len(retained))
        finally:
            await provider.aclose()
        return findings

    async def _review_evidence(
        self,
        provider: AsyncOpenAICompatibleProvider,
        feature: dict[str, object],
        problem: dict[str, object],
        evidence: dict[str, object],
        locale: str,
        index: int,
    ) -> dict[str, object] | None:
        response = await provider.complete_json(
            [
                {
                    "role": "system",
                    "content": (
                        "Review exactly one Vault passage against its Solution claim. Return JSON "
                        "{conflict:boolean,evidence_id,severity:'low|medium|high',category,summary,"
                        "current_claim,existing_claim,impact,recommendation,explanation}. "
                        "Keep every field concise. Never invent a citation."
                    ),
                },
                {"role": "system", "content": response_language_instruction(locale)},
                {
                    "role": "user",
                    "content": json.dumps(
                        {"solution": feature, "problem": problem, "evidence": evidence},
                        ensure_ascii=False,
                    ),
                },
            ],
            "conflict evidence review",
        )
        if not self._is_conflict(response, evidence):
            return None
        return self._structured_conflict(response, evidence, index)

    @staticmethod
    def _structured_conflict(
        response: dict[str, Any], evidence: dict[str, object], index: int
    ) -> dict[str, object]:
        path = str(evidence["path"])
        severity = _severity(response.get("severity"))
        explanation = str(response.get("explanation", "")).strip()
        current_claim = _response_text(response, "current_claim", _response_text(response, "claim", evidence["claim"]))
        existing_claim = _response_text(response, "existing_claim", evidence["text"])
        return {
            "id": f"conflict-{index}",
            "target_id": path,
            "target_title": Path(path).stem or path,
            "severity": severity,
            "category": _response_text(response, "category", "Conflicting requirement"),
            "summary": _response_text(response, "summary", explanation or "The current and existing claims differ."),
            "current_claim": current_claim,
            "existing_claim": existing_claim,
            "impact": _response_text(response, "impact", explanation or "The competing claims require a decision."),
            "recommendation": _response_text(response, "recommendation", "Review the competing claims."),
            "evidence": [
                {
                    "evidence_id": evidence["id"],
                    "citation": f"{path}:{evidence['start_line']}-{evidence['end_line']}",
                    "excerpt": evidence["text"],
                    "source_hash": evidence["source_hash"],
                    "start_line": evidence["start_line"],
                    "end_line": evidence["end_line"],
                }
            ],
        }

    @staticmethod
    def _is_conflict(response: dict[str, Any], evidence: dict[str, object]) -> bool:
        return bool(
            response.get("conflict") is True
            and str(response.get("evidence_id")) == evidence["id"]
            and str(response.get("explanation", "")).strip()
        )

    @staticmethod
    def _recommended_state(
        findings: list[dict[str, object]],
        candidates: list[dict[str, object]],
        coverage: float,
    ) -> str:
        if findings:
            return "potential_conflict"
        if candidates and coverage >= 1.0:
            return "clear"
        return "insufficient_evidence"

    def _report(
        self,
        feature_id: str,
        query: str,
        scope: dict[str, object],
        coverage: float,
        claims: list[dict[str, str]],
        candidates: list[dict[str, object]],
        retained: list[dict[str, object]],
        findings: list[dict[str, object]],
        state: str,
    ) -> dict[str, Any]:
        run_id = self.workflow.start_conflict_review(feature_id, query)
        return {
            "run_id": run_id,
            "feature_id": feature_id,
            "status": "conflicts_found" if findings else state,
            "phase": "complete",
            "recommended_state": state,
            "state_meanings": STATE_MEANINGS,
            "scope": {**scope, "embedding_coverage": coverage},
            "claims": claims,
            "candidate_count": len(candidates),
            "retained_count": len(retained),
            "reviewed_count": len(retained),
            "progress": 1.0,
            "findings": findings,
            "conflicts": [
                {**finding, "id": f"conflict-{index + 1}"} for index, finding in enumerate(findings)
            ],
            "candidates": candidates,
            "summary": STATE_MEANINGS[state],
        }
