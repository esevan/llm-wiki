# Workbench Interface Checklist

**Updated**: 2026-08-19  
**Scope**: The visible Workbench interface and its backing local API.

## Checked and complete

- [x] **Capture inbox** — Save Capture persists immediately; Capture is the first step and stays
  separate from active Problems.
- [x] **Capture refinement** — Explore with AI streams a current-item conversation, shows a
  progress state, and opens a review containing a concise proposed title plus current note/context.
- [x] **Capture → Problem** — Draft next Problem opens a next-stage collection chat; its persistent
  review action produces an editable Problem draft; human finalization creates the draft Problem
  and removes the linked Capture from the active inbox.
- [x] **Problem stage** — Explore/refine, manual update, human approval, and next-Solution drafting
  use the appropriate current-item or next-stage operation.
- [x] **Solution stage** — Explore/refine, explicit cited clear-conflict recording, conflict-gated
  approval, Work Log, validation checks, completion review, manual update, and handoff copy are connected to local APIs.
- [x] **Flow view** — The optional Workbench view presents Problem → Solution lineage.
- [x] **Delete** — Visible delete controls use `/api/items/...`; soft deletion is reversible in the
  persistence layer and does not touch vault files.
- [x] **AI setup** — Endpoint, secure API key, Default and Advanced models, task-level Advanced
  options with Default-model fallback, model health check, and configuration save use
  non-overlapping API routes.
- [x] **Chat ergonomics** — Enter sends, Shift+Enter inserts a line, stream chunks preserve words,
  terminal SSE markers are not rendered, and the Explore dialog has a larger note area.

## Verification evidence

- [x] Mock-provider Workbench flow: Capture → refine → next-stage chat → Problem → approved
  Solution → Work Log and completion review → handoff.
- [x] API tests cover Capture persistence, inbox transition, provider configuration routing, item
  updates, deletion/restore, approval gates, completion, and projections.
- [x] Browser script syntax validation passes.
- [ ] Native Playwright visual click-through remains blocked locally: Chromium is not installed and
  the earlier browser download was blocked by the machine TLS certificate chain. This does not block
  the local API/interface contract tests above; rerun once Chromium is available.
