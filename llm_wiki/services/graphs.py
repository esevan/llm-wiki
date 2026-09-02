"""LangGraph orchestration is loaded only when a person initiates AI enrichment."""
from __future__ import annotations

from llm_wiki.services.ai import AIEnrichmentEngine


def enrich_problem_graph(engine: AIEnrichmentEngine, statement: str, citations: list[str]) -> dict[str, object]:
    try:
        from langgraph.graph import END, START, StateGraph
    except ImportError as error:
        raise RuntimeError("Install the optional AI workflow runtime") from error

    def enrich(state: dict[str, object]) -> dict[str, object]:
        return {"result": engine.enrich_problem(str(state["statement"]), list(state["citations"]))}

    graph = StateGraph(dict)
    graph.add_node("enrich", enrich)
    graph.add_edge(START, "enrich")
    graph.add_edge("enrich", END)
    return dict(graph.compile().invoke({"statement": statement, "citations": citations})["result"])
