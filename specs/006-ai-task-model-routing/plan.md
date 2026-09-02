# Implementation Plan: Task-Level AI Model Routing

**Branch**: `006-ai-task-model-routing` | **Date**: 2026-08-21 | **Spec**: [spec.md](spec.md)

## Summary

Replace stage-specific AI model configuration with two model tiers and task-level routing. People provide a Default and an optional Advanced model, choose the Advanced tier per named AI task in a collapsed settings section, and always receive Default-model fallback when the Advanced model is unavailable.

## Technical Context

**Language/Version**: Python 3.13 and browser JavaScript  
**Primary Dependencies**: FastAPI, Pydantic, SQLite, OpenAI-compatible provider adapter  
**Storage**: Local SQLite configuration; OS keyring for API key  
**Testing**: pytest and browser-script syntax validation  
**Target Platform**: Local macOS and Windows application  
**Project Type**: Local web application  
**Performance Goals**: Configuration lookup adds no network request and no meaningful delay to an AI request; capture persistence remains AI-free.  
**Constraints**: Private API key stays in the OS keyring; all provider access remains behind the provider adapter; blank Advanced model falls back to Default model.  
**Scale/Scope**: One local configuration record, twelve supported AI task identifiers, one settings screen.

## Constitution Check

| Gate | Assessment |
|---|---|
| Product Spirit I & II | Pass. Two model fields and a collapsed task list remove stage taxonomy and keep optional detail hidden. |
| Product Spirit III | Pass. Saved task preferences make the selected quality policy visible when a person returns. |
| Product Spirit IV–VI | Pass. No workflow state, knowledge publication, or worker scoring behavior changes. |
| Independent adapters | Pass. Routing chooses a model before using the existing provider adapter; no provider-specific behavior enters workflow code. |
| Human authority & privacy | Pass. The person explicitly selects task tiers; API keys remain outside configuration responses and local storage. |
| Performance & complexity | Pass. One local settings lookup and no new dependency or hot-path AI invocation. |

**Post-design review**: Pass. The design maintains the two-tier, task-level choice without exposing legacy workflow-stage configuration.

## Project Structure

```text
specs/006-ai-task-model-routing/
├── spec.md
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/provider-config.md
├── checklists/requirements.md
└── tasks.md

llm_wiki/
├── api/app.py
├── services/settings.py
└── static/index.html

tests/
├── test_api.py
└── test_provider.py
```

**Structure Decision**: Keep configuration, routing, and UI in their existing layers. Stable task identifiers belong at the settings boundary; API endpoints ask for an identifier rather than a workflow stage.

## Complexity Tracking

No constitution violations or additional complexity justifications are required.
