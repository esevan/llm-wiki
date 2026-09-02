from __future__ import annotations

from importlib.resources import files

from llm_wiki.services.localization import response_language_instruction


_STAGES = frozenset({"captures", "problems", "features"})


def system_prompt(entity_type: str) -> str:
    """Load an independently editable prompt for the active workflow stage."""
    if entity_type not in _STAGES:
        raise ValueError(f"Unknown workflow prompt: {entity_type}")
    return files("llm_wiki.prompts").joinpath(f"{entity_type}.md").read_text(encoding="utf-8")


def refinement_focus_prompt(assessment: dict[str, object]) -> str:
    """Constrain the next turn to the readiness gap shown in Refinement Preview."""
    focus = assessment.get("focus")
    if not isinstance(focus, list) or not focus:
        return (
            "The visible refinement structure has no material gap. Do not invent another topic or "
            "prolong the conversation; declare readiness when the item can be drafted safely."
        )
    labels = [str(item.get("label", "")).strip() for item in focus if isinstance(item, dict) and item.get("label")]
    return (
        f"The visible Refinement Preview says the next focus is: {', '.join(labels)}. Concentrate the "
        "next turn only on this focus group. Do not try to resolve every remaining gap. Ask exactly "
        "one sharp, open-ended question. Multiple fields are listed only because one concrete answer "
        "can resolve them together and reduce ping-pong; do not turn them into a checklist or multiple "
        "questions. Use known context and never ask for information already supplied."
    )


def draft_prompt(entity_type: str, title: str, detail: str) -> str:
    """Return the constrained JSON-drafting instruction for one workflow stage."""
    schemas = {
        "captures": '{"title":"a clear problem statement","detail":"structured problem note"}',
        "problems": '{"title":"short feature name","outcome":"intended outcome","non_goals":"boundaries","validation_criteria":"- [ ] observable criterion"}',
    }
    if entity_type not in schemas:
        raise ValueError(f"This workflow item has no next stage: {entity_type}")
    richness = ""
    if entity_type == "captures":
        richness = (
            " You are drafting a Problem. Make detail a useful structured Problem note with these headings: "
            "Context, Impact, Evidence, Desired outcome, Boundaries, and Open questions. Preserve known facts "
            "from the Capture conversation and use 'Not yet known' for missing facts."
        )
    elif entity_type == "problems":
        richness = (
            " You are drafting a Solution. Keep title short, but make outcome a detailed structured Solution note with these headings: "
            "Context, Intended outcome, Scope, Non-goals, Evidence and prior context, Trade-offs, Dependencies, "
            "Validation criteria, Risks, and Open questions. Preserve every relevant fact from the conversation and use "
            "'Not yet known' for missing facts. Do not compress important context into a slogan."
        )
    return (
        "You prepare one proposed workflow draft for human review. Return JSON only, matching this "
        f"exact shape: {schemas[entity_type]}. Be concise and use only the supplied context. "
        "Do not invent facts. Do not write source filenames, code, commands, frameworks, deployment "
        "steps, or a technical implementation plan. The human must review before anything is applied."
        f"{richness}\n\n"
        f"Current item title: {title}\nCurrent detail: {detail}"
    )


def bilingual_draft_prompt(entity_type: str, title: str, detail: str) -> str:
    """Return one structured instruction that creates both durable language versions."""
    fields = {
        "captures": '{"title":string,"detail":string}',
        "problems": '{"title":string,"outcome":string,"non_goals":string,"validation_criteria":string}',
    }
    if entity_type not in fields:
        raise ValueError(f"This workflow item has no next stage: {entity_type}")
    base = draft_prompt(entity_type, title, detail)
    return (
        f"{base}\n\nThis proposal will become durable content after human review. Return both natural Korean "
        "and English versions in one response so switching languages never requires another model call. "
        f"Return JSON only with exactly this outer shape: {{\"ko\":{fields[entity_type]},\"en\":{fields[entity_type]}}}. "
        "Both versions must preserve the same facts, boundaries, checklist meaning, citations, code, and identifiers."
    )


def refinement_prompt(entity_type: str, title: str, detail: str) -> str:
    """Return a JSON instruction for improving the current item, never advancing it."""
    schemas = {
        "captures": '{"title":"refined capture"}',
        "problems": '{"title":"short problem title","detail":"structured problem note"}',
        "features": '{"title":"refined feature name","detail":"refined intended outcome"}',
    }
    if entity_type not in _STAGES:
        raise ValueError(f"Unknown workflow prompt: {entity_type}")
    return (
        "Refine the current workflow item using the conversation. Do not advance to another workflow "
        f"stage. Return JSON only, matching exactly: {schemas[entity_type]}. Be concise, preserve known "
        "facts, and do not invent facts. Keep title fields under 70 characters and focused on the outcome; "
        "put useful nuance in detail fields instead. For Problems, make detail a rich structured note with the headings: "
        "Context, Impact, Evidence, Desired outcome, Boundaries, and Open questions. Include only facts from the item "
        "and its conversation; use 'Not yet known' where needed. For Solutions, make detail a rich structured Solution note with the headings: "
        "Context, Intended outcome, Scope, Non-goals, Evidence and prior context, Trade-offs, Dependencies, Validation criteria, Risks, and Open questions. "
        "Preserve concrete user feedback and do not reduce known constraints to a short summary. Do not write code, commands, filenames, frameworks, deployment "
        "steps, or technical implementation plans. The human must review before it is applied.\n\n"
        f"Current item title: {title}\nCurrent detail: {detail}"
    )


def bilingual_refinement_prompt(entity_type: str, title: str, detail: str) -> str:
    """Refine a durable item into aligned Korean and English reviewed versions."""
    if entity_type not in {"problems", "features"}:
        return refinement_prompt(entity_type, title, detail)
    base = refinement_prompt(entity_type, title, detail)
    return (
        f"{base}\n\nThis refinement updates durable stored content. Return both natural Korean and English "
        "versions in one response, with identical facts, boundaries, citations, code, and identifiers. "
        "Return JSON only with exactly this outer shape: "
        '{"ko":{"title":string,"detail":string},"en":{"title":string,"detail":string}}.'
    )
