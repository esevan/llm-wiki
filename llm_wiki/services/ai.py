"""AI enrichment behind the provider boundary; never advances human workflow state."""
from __future__ import annotations

from llm_wiki.services.provider import OpenAICompatibleProvider


class AIEnrichmentEngine:
    def __init__(self, provider: OpenAICompatibleProvider):
        self.provider = provider

    def enrich_problem(self, statement: str, citations: list[str]) -> dict[str, object]:
        prompt = {
            "role": "user",
            "content": (
                "Return JSON only with normalized_problem, pain, non_goals, categories, and importance_rationale. "
                "Use only the cited context. Do not approve state or provide files, code, commands, deployments, "
                "or technical implementation steps.\nProblem: " + statement + "\nCitations: " + ", ".join(citations)
            ),
        }
        result = self.provider.complete_json([prompt], "problem enrichment")
        required = {"normalized_problem", "pain", "non_goals", "categories", "importance_rationale"}
        if not required <= result.keys():
            raise ValueError("Problem enrichment response missed required fields")
        return {key: result[key] for key in required}
