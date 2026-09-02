# Implementation Plan: Direction Dashboard

**Status**: Implemented baseline; this is the as-built design summary.

## Current design

Compass goals, importance assessments, immutable score events, and refreshed period totals are
SQLite-owned and exposed by the local dashboard API. Importance carries alignment, impact, urgency,
leverage, and required evidence; it is deliberately separate from milestone completion.

## Scoring boundary

The workflow writes 10% on approval, 20% on verification, and 70% on completion. Events are never
rewritten to make a later dashboard total look better; period totals are rebuilt from the ledger.
Compass is a local fallback for retaining direction and evidence when the AI provider is unavailable,
not an automatic ranking engine.

## Verification surface

`tests/test_completion_dashboard.py`, `tests/test_workflow.py`, and `tests/test_api.py` cover
goals, evidence-backed importance, score events, aggregates, and dashboard responses.
