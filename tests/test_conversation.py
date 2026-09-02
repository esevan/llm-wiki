import pytest

from llm_wiki.api.app import validate_draft, validate_refinement
from llm_wiki.services.conversation import (
    bilingual_draft_prompt,
    bilingual_refinement_prompt,
    draft_prompt,
    refinement_focus_prompt,
    refinement_prompt,
    response_language_instruction,
    system_prompt,
)


def test_each_workflow_stage_allows_context_before_a_needed_open_question() -> None:
    for stage in ("captures", "problems"):
        prompt = system_prompt(stage)
        assert "no more than 60 words" in prompt
        assert "Ask at most one focused, open-ended question" in prompt
        assert "only when a material ambiguity" in prompt
        assert "open-ended" in prompt
        assert "yes/no" in prompt
        assert "within 1–3 user exchanges" in prompt
        assert "Do not prolong the chat" in prompt
        assert "Leave non-blocking unknowns for Open questions" in prompt
        assert "24 words" not in prompt
        assert "AI refinement" in prompt
        assert "implementation plans" in prompt


def test_solution_chat_collects_decision_ready_detail_without_endless_ping_pong() -> None:
    prompt = system_prompt("features")
    assert "no more than 120 words" in prompt
    assert "within 2–5 user exchanges" in prompt
    assert "intended outcome" in prompt
    assert "scope or non-goals" in prompt
    assert "validation criteria" in prompt
    assert "material risk or trade-off" in prompt
    assert "Do not prolong the chat for low-value detail" in prompt
    assert "detailed, decision-ready refinement" in prompt
    assert "one material gap at a time" in prompt
    assert "one sharp question" in prompt
    assert "ping-pong" in prompt


def test_durable_draft_requests_both_languages_in_one_structured_response() -> None:
    prompt = bilingual_draft_prompt("problems", "Source problem", "Known context")
    assert '"ko"' in prompt
    assert '"en"' in prompt
    assert "both natural Korean and English" in prompt
    assert "human review" in prompt


def test_durable_refinement_requests_aligned_bilingual_versions() -> None:
    prompt = bilingual_refinement_prompt("features", "Current", "Known outcome")
    assert '"ko"' in prompt
    assert '"en"' in prompt
    assert "durable stored content" in prompt
    assert "identical facts" in prompt


def test_live_response_language_instruction_is_single_locale_and_preserves_evidence() -> None:
    korean = response_language_instruction("ko")
    english = response_language_instruction("en")
    assert "only in natural Korean" in korean
    assert "only in natural English" in english
    assert "quoted evidence verbatim" in korean
    assert "quoted evidence verbatim" in english


def test_visible_preview_focus_constrains_the_actual_next_question() -> None:
    prompt = refinement_focus_prompt({"focus": [
        {"key": "outcome", "label": "Intended outcome", "status": "missing"},
        {"key": "validation", "label": "Validation criteria", "status": "weak"},
    ]})
    assert "Intended outcome, Validation criteria" in prompt
    assert "only on this focus group" in prompt
    assert "exactly one sharp, open-ended question" in prompt
    assert "reduce ping-pong" in prompt
    assert "checklist or multiple questions" in prompt


def test_drafting_prompt_is_reviewed_and_nontechnical() -> None:
    prompt = draft_prompt("problems", "Example", "A human goal")
    assert "human review" in prompt
    assert "JSON only" in prompt
    assert "technical implementation plan" in prompt


def test_drafts_require_the_stage_structure() -> None:
    assert validate_draft("captures", {"title": "A clear problem", "detail": "## Context\nKnown facts"}) == {
        "title": "A clear problem",
        "detail": "## Context\nKnown facts",
    }
    with pytest.raises(ValueError, match="outcome"):
        validate_draft("problems", {"title": "Missing fields"})


def test_refinement_stays_on_the_current_stage() -> None:
    assert "Do not advance" in refinement_prompt("captures", "Thought", "")
    assert validate_refinement("features", {"title": "Outcome", "detail": "Detailed intended outcome"})["title"] == "Outcome"
