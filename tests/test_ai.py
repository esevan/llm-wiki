from llm_wiki.services.ai import AIEnrichmentEngine


class FakeProvider:
    def __init__(self, value: dict[str, object]): self.value = value
    def complete_json(self, messages: list[dict[str, str]], schema_name: str) -> dict[str, object]: return self.value


def test_problem_enrichment_requires_the_human_review_fields() -> None:
    engine = AIEnrichmentEngine(FakeProvider({"normalized_problem": "Useful work", "pain": "Value", "non_goals": "None", "categories": ["General"], "importance_rationale": "Evidence"}))  # type: ignore[arg-type]
    assert engine.enrich_problem("Useful work", ["capture"]) ["normalized_problem"] == "Useful work"
