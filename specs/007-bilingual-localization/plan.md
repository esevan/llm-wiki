# Implementation Plan: Korean-English Localization

**Branch**: `feat/bilingual-localization` | **Date**: 2026-08-21 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/007-bilingual-localization/spec.md`

## Summary

Add one persisted Korean/English application locale, centralized UI resources, locale-bound single-language live AI responses, bilingual sidecar storage for newly approved AI-generated Problems, Solutions, and explicitly requested Image Summaries, and English-canonical managed Knowledge with hash-keyed on-demand Korean translations. Existing workflow rows, Image Summaries, and Vault files remain untouched and fall back to their stored original when no requested-language version exists.

## Technical Context

**Language/Version**: Python 3.12+ and browser JavaScript  
**Primary Dependencies**: FastAPI, Pydantic, SQLite, existing OpenAI-compatible provider and MarkdownVaultAdapter; no new runtime dependency  
**Storage**: Existing local SQLite database plus English-canonical Markdown in the configured Vault  
**Testing**: pytest, FastAPI TestClient, Playwright browser tests, browser-script syntax validation  
**Target Platform**: Local macOS and Windows application; CI coverage also runs on Ubuntu  
**Project Type**: Local single-user web application with a FastAPI backend and dependency-free browser UI  
**Performance Goals**: Static locale switches update visible UI within 100 ms on a representative populated workbench; stored-version reads and language switches perform zero provider calls; Image Summary continues to use one explicit provider request; existing capture and 1,000-note indexing budgets remain unchanged.
**Constraints**: Preserve legacy rows, Image Summaries, and Vault files without backfill; hot capture and Work Log persistence stay AI-free; machine values, paths, citations, code, and authored Work Log evidence are not translated; canonical Knowledge writes remain reviewed, atomic, and conflict-guarded through MarkdownVaultAdapter; locale resources have exact key parity.
**Scale/Scope**: Two locales, one local user preference, one browser shell, all primary UI surfaces, AI-generated Problem/Solution fields, AI-generated Work Log Image Summaries, and app-managed Knowledge documents.

## Constitution Check

| Gate | Assessment |
|---|---|
| Product Spirit I & II | Pass. One global language control removes repeated interpretation burden; raw capture remains immediate and AI-free. |
| Product Spirit III | Pass. The locale persists, language switching preserves current context, and stored Image Summary variants let visual Work Log evidence remain understandable without reconstruction. |
| Product Spirit IV | Pass. Localization does not add a workflow stage or alter Problem/Solution lineage. |
| Product Spirit V | Pass. Only human-approved app-managed Knowledge remains authoritative English canonical content; Korean output is a clearly marked, linked derived reading under `Translations/ko` and is excluded from canonical retrieval. |
| Product Spirit VI | Pass. No worker score, rank, or judgment is added. |
| Measured performance | Pass. The plan adds locale-switch/provider-call checks, keeps Image Summary at one explicit provider request, and retains existing capture/index budgets. |
| Independent adapters | Pass. Provider calls stay behind OpenAICompatibleProvider and all canonical Vault access stays behind MarkdownVaultAdapter. |
| Human authority & evidence | Pass. Generated bilingual drafts require existing review/apply actions; failures preserve source content; canonical writes keep atomic conflict guards. |
| Local/cross-platform & privacy | Pass. SQLite and portable Markdown remain local; no new external service or platform-only dependency is introduced. |
| Minimal complexity | Pass. Vanilla JS, JSON resources, and two small SQLite sidecar tables avoid a frontend framework or migration dependency. |

**Post-design review**: Pass. The entity and interface design retains each gate, including read-time canonical hash validation for externally edited Knowledge.

## Project Structure

### Documentation (this feature)

```text
specs/007-bilingual-localization/
├── spec.md
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/localization-api.md
├── checklists/requirements.md
├── checklists/localization.md
└── tasks.md
```

### Source Code (repository root)

```text
llm_wiki/
├── api/app.py
├── services/
│   ├── conversation.py
│   ├── localization.py
│   ├── settings.py
│   └── workflow.py
└── static/
    ├── index.html
    └── i18n/
        ├── en.json
        └── ko.json

tests/
├── test_api.py
├── test_browser_menu.py
├── test_conversation.py
├── test_localization.py
└── test_workflow.py

docs/
├── features/bilingual-localization.md
├── features/bilingual-localization.ko.md
├── product-spirit.md
└── product-spirit.ko.md
```

**Structure Decision**: Keep the existing single-package architecture. A focused localization service owns locale validation and localized field overlays for workflow records and Image Summaries, bilingual draft normalization, and Knowledge translation cache rules; existing workflow, provider, vault, API, and single-file UI boundaries remain intact.

## Complexity Tracking

No constitution violations or additional complexity justifications are required.

## Progressive Reading Addendum (2026-08-24)

Keep the canonical read endpoint as the sub-second, provider-free path and add a newline-delimited JSON translation stream for cache misses. The browser owns one `AbortController` and server request id at a time; superseding navigation both aborts the response and signals the server cancellation registry. The server translates stable Markdown paragraph blocks, emits only complete blocks, verifies the canonical hash before committing the assembled result to the existing cache, and emits a recoverable failure event without changing canonical Markdown.

The progress surface is a sticky high-contrast live region above the document. Each paragraph transition temporarily stacks the English and Korean versions in the same layout slot, then uses opposing left-to-right clipping masks over roughly 900 ms before removing the English layer. Reduced-motion preferences continue to suppress decorative movement.

Persist completed readings through `MarkdownVaultAdapter` as atomic derived Markdown at `Translations/ko/<canonical-path>`. The derived frontmatter carries an Obsidian canonical link and exact source identity. Vault discovery excludes the complete `Translations/ko` tree so derived readings cannot enter canonical retrieval. A compatibility wrapper promotes matching legacy SQLite rows to files on first read, then all cache writes and invalidation use the Vault representation.
