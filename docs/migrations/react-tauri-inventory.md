# React and Tauri migration inventory

**Audit date:** 2026-09-02
**Baseline:** `187 passed`, no skipped tests in the baseline run

This is the live behavior and coverage ledger for the compatibility-first desktop migration. The
existing implementation, specifications, characterization catalog, tests, and documentation were
treated as mutually supporting evidence. Product behavior is unchanged unless a row explicitly
says otherwise.

The initial per-behavior statuses below capture the Phase 0 baseline. The updated Phase 9 matrix
and final gate results are maintained in [react-tauri-report.md](react-tauri-report.md); its status
supersedes this baseline after implementation begins.

## Architecture before migration

- One server-rendered static entry point, `llm_wiki/static/index.html`, owns HTML, CSS, localization,
  client state, DOM rendering, dialogs, polling, streaming, and every user interaction.
- FastAPI controllers expose the application over loopback HTTP. `web.app.create_app` composes a
  reusable `ApplicationRuntime`; workflow, retrieval, localization, jobs, filesystem, provider, and
  archive behavior already live below the HTTP boundary.
- SQLite stores workflow, indexes, jobs, notifications, and localization metadata. Non-secret app
  settings live in `~/.llm-workbench/settings.json`; provider secrets use the OS keyring. The
  Markdown vault adapter is the only Vault filesystem boundary.
- A CLI supervises the loopback web process, one ephemeral Fast Queue worker, and configurable
  durable workers. SSE announces index/job changes; streamed chat responses retain cancellation.

## System inventory

| Surface | Current owner | Important semantics |
|---|---|---|
| Workbench and Capture | browser DOM + `WorkflowEngine` | Lightweight create, inbox removal after promotion, soft delete/restore, category and importance |
| Explore and Refinement | browser DOM + fast/durable queues | streamed chat, bounded context, stale-result rejection, explicit apply/create, close detachment |
| Conflict Review | durable job + workflow transaction | conservative report, complete human resolution set, source-query freshness gate |
| Solution Work Log | browser DOM + workflow | text/image evidence, comments, checklist, derived translations, async image summary |
| Completion and archive | workflow + archive publisher + vault | review, human verification, atomic Markdown write, external-change block, undo/history |
| Knowledge | retrieval + vault translation cache | English canonical source, request-driven Korean reading, paragraph progress, durable cancellation |
| Search | retrieval + SSE | lexical availability, optional semantic rerank, pagination, changed-file indexing |
| Compass | workflow | goals, immutable milestone contribution events, no worker scoring |
| AI Setup | provider settings + keyring | loopback/default endpoint, model routing, bounded worker count, secret not returned |
| Queue and notifications | job repository + browser poll/SSE | safe payloads, retry/cancel, result destinations, unread/dismiss persistence |
| Localization | packaged JSON + localized store | instant locale switch, authored text preserved, explicit locale persistence |
| Lifecycle | CLI/runtime | loopback-only web, startup composition, worker supervision, termination on shutdown |

There is no general-purpose database outside SQLite and no MCP implementation in the current
repository. Git behavior is limited to source-hash/three-way-safe patch and externally modified
vault-file protection; no Git CLI integration exists in application code.

## Behavior migration map and live coverage

Status values are `PASS`, `PARTIAL`, `MISSING`, `NOT_APPLICABLE`, and `BLOCKED`.

| ID | Feature / actor trigger | Preconditions and current/alternate/error behavior | Side effects and dependencies | Evidence | Target React module | Risk / confidence | React UI | Command | Desktop E2E | Status |
|---|---|---|---|---|---|---|---|---|---|---|
| UI-001 | Navigate between Workbench, Search, Compass, Setup | Click changes one visible view; locale/input state stays intact | none | `test_browser_menu.py`, CB-008 | `app/navigation` | low / high | PASS | NOT_APPLICABLE | PASS | PASS |
| UI-002 | Create Capture | Non-empty text saves; validation blocks empty; backend error is visible | SQLite row; derived translation job | `test_api.py`, CB-001/010 | `features/workbench` | medium / high | PASS | PASS | PASS | PASS |
| UI-003 | Load and organize Workbench | Empty and categorized boards render; organization is async and preserves explicit overrides | SQLite metadata; AI queue | `test_workflow.py`, CB-007 | `features/workbench` | high / high | PASS | PASS | PASS | PASS |
| UI-004 | Promote Capture to Problem | AI draft remains reviewable; manual transition validates statement; linked Capture leaves inbox | workflow transaction | `test_workbench_flow.py`, `test_transitions.py` | `features/refinement` | high / high | PASS | PASS | PASS | PASS |
| UI-005 | Approve Problem / draft Solution | Human approval required; unapproved Problem cannot create Solution | workflow transition + score event | `test_completion_dashboard.py`, CB-002 | `features/workbench` | high / high | PASS | PASS | PASS | PASS |
| UI-006 | Explore with AI | Streams one response, preserves six turns and request-start locale; provider errors do not change state | Fast Queue, provider | `test_workbench_flow.py`, CB-003/012 | `features/explore` | high / high | PASS | PASS | PASS | PASS |
| UI-007 | Refinement Preview | Bounded current context; background draft may fail/retry; stale results cannot replace current view | durable job + polling | `test_browser_menu.py`, CB-018 | `features/refinement` | high / high | PASS | PASS | PASS | PASS |
| UI-008 | Conflict Review | Running/result states; every finding needs a human decision; stale/incomplete resolution is rejected | durable job + atomic workflow update | `test_api.py`, `test_jobs_api.py`, CB-005/015 | `features/conflicts` | high / high | PASS | PASS | PASS | PASS |
| UI-009 | Approve/skip Solution conflict gate | Cited clear review approves; explicit skip requires reason; conflict blocks | SQLite state + score event | `test_transitions.py`, CB-005 | `features/conflicts` | high / high | PASS | PASS | PASS | PASS |
| UI-010 | Move Solution stages | Only declared transitions appear; required conditional fields validate | workflow state | `test_transitions.py` | `features/workbench` | medium / high | PASS | PASS | PASS | PASS |
| UI-011 | Add Work Log evidence | Text or image persists; image summary is asynchronous; source remains authored | SQLite + derived jobs | `test_api.py`, `test_workflow.py`, CB-004/010 | `features/work-log` | high / high | PASS | PASS | PASS | PASS |
| UI-012 | Edit checklist/comments | Save and checked state persist; translation is scheduled; failures preserve source | SQLite + derived job | `test_localization.py`, CB-010, PA-005 | `features/work-log` | medium / medium | PASS | PASS | PASS | PASS |
| UI-013 | Completion Review and verify | Evidence review can fail/retry; only user verification completes; Solution, Problem, and refined Capture close together; one notification published | job, transactional workflow closure, vault playbook, archive, score | `test_completion_dashboard.py`, Rust cascade test, CB-006/013/017 | `features/completion` | high / high | PASS | PASS | PASS | PASS |
| UI-014 | Completion playbook conflict/regenerate/delete | External edit blocks; force path is confirmed; regeneration follows current lineage | atomic vault writes + index | `test_api.py`, `test_patches.py` | `features/completion` | high / high | PASS | PASS | PASS | PASS |
| UI-015 | Search vault | Empty/loading/result/pagination; bundled multilingual semantic rerank; lexical fallback | startup index reads Vault; local ONNX vectors persist in SQLite | `test_retrieval.py`, Rust real-model test, desktop E2E, CB-001, PA-001 | `features/search` | medium / high | PASS | PASS | PASS | PASS |
| UI-016 | Read/translate Knowledge | Canonical opens immediately; managed Korean translation progresses; close detaches without cancel | vault cache + durable job | `test_api.py`, `test_browser_menu.py`, CB-009 | `features/knowledge` | high / high | PASS | PASS | PASS | PASS |
| UI-017 | Locale switch/restore | Static UI and stored overlays change without reload/AI; unsaved inputs and open dialogs remain | locale setting | `test_localization.py`, `test_browser_menu.py`, CB-008 | `features/localization` | high / high | PASS | PASS | PASS | PASS |
| UI-018 | Queue cancel/retry/result | Safe status and progress render; cancellation prevents late publish; retry creates valid attempt | jobs repository | `test_ai_jobs.py`, `test_jobs_api.py`, CB-011/014 | `features/jobs` | high / high | PASS | PASS | PASS | PASS |
| UI-019 | Notifications | Completion Review creates one alert; read/dismiss persists | notifications table | `test_jobs_api.py`, CB-013 | `features/jobs` | medium / high | PASS | PASS | PASS | PASS |
| UI-020 | Compass goals/progress | Goal creates; milestone evidence allocates 10/20/70; no person score | SQLite ledger | `test_completion_dashboard.py`, CB-017 | `features/compass` | medium / high | PASS | PASS | PASS | PASS |
| UI-021 | Provider setup/test | Config saves without exposing key; failed test is readable; routing and worker bounds persist | keyring + settings + provider | `test_api.py`, `test_worker_processes.py` | `features/settings` | high / high | PASS | PASS | PASS | PASS |
| UI-022 | Delete/restore/follow-up | Destructive action confirms; soft delete preserves history; completed Solution can create follow-up Problem | SQLite transitions | `test_api.py`, `test_workflow.py` | `features/workbench` | medium / high | PASS | PASS | PASS | PASS |
| UI-023 | Completed lineage | Deterministic lineage remains on inference failure; corrections preserve revisions | SQLite + optional AI | `test_api.py`, `test_workflow.py`, CB-016 | `features/lineage` | high / high | PASS | PASS | PASS | PASS |
| UI-024 | Startup/shutdown/relaunch | Runtime initializes index/settings/jobs; workers terminate with shell; persisted state reloads | filesystem, SQLite, processes | `test_cli.py`, `test_worker_processes.py` | `app/lifecycle` | high / medium | PASS | PASS | PASS | PASS |
| UI-025 | Reviewed patch apply/undo | Hash mismatch blocks; accepted patch is atomic and reversible | vault filesystem + mirror hash | `test_patches.py`, CB-004 | `features/completion` | high / high | PASS | PASS | PASS | PASS |
| UI-026 | First-install welcome and Vault choice | Genuine new installs show one full-monitor transparent motion surface above the centered app; skip/finish opens the native picker; cancel leaves Vault setup pending without replay; existing installs retain the legacy default | one-time home setting + native window/folder picker; no indexing before selection | CB-022/023, first-run React tests, Rust startup tests, packaged macOS acceptance, desktop E2E | `features/first-run-intro`, `features/vault-setup` | high / high | PASS | PASS | PASS | PASS |
| DB-001 | Upgrade SQLite on startup | Given an unversioned or older DB, startup applies ordered migrations and preserves data; a failed step rolls back; a newer DB is rejected | SQLite schema and `PRAGMA user_version` | Rust migration/command tests, desktop launch and relaunch E2E | `app/lifecycle` | high / high | NOT_APPLICABLE | PASS | PASS | PASS |

## Endpoint classification

- **Commands:** create/update/delete/restore items, transitions, organize/category/importance, conflict
  decisions, progress/comments/checklist, goals, completion/verification, lineage corrections,
  patch apply/undo, provider/locale settings, project/archive, job cancel/retry, notification updates.
- **Queries:** health, board, item/record/detail, transitions, archive/completed context, search,
  dashboard, progress, lineage/evidence, handoff, provider config, locale/resources, Knowledge, jobs,
  notifications.
- **Streaming/background:** index events, job events and polling, live chat, draft/refine/job enqueue,
  conflict review, image summary, completion review, translation, embedding and lineage work.
- **Transport-only:** HTTP status/header/CORS/FileResponse mapping and SSE framing.
- **Intentionally retained external web API:** none. The FastAPI browser product was retired after
  native parity and remains available only in Git history at `caef236`.
- **Obsolete candidates after parity:** the packaged Python boundary, generic HTTP-shaped Tauri
  request command, loopback port allocation, sidecar lifecycle, and PyInstaller bundle were removed.
  Browser routes, HTTP DTOs, the HTTP adapter, and Python tests were removed together in the
  explicit retirement commit above the parity snapshot.

## Known insufficient coverage before migration

- Before migration, no browser scenario ran through a native desktop command boundary; the checked-in
  release-app scenario now covers that boundary and process relaunch.
- The retired Playwright tests exercised the real DOM but stubbed HTTP for most workflows. Native
  command and packaged E2E coverage supersede them in the current product.
- Before migration, no app relaunch/persistence test existed at the desktop shell level; it now does.
- Immediate provider abort, semantic corpus-wide search, checklist retranslation, progressive
  Conflict Review deduplication, Lineage inference failure UI, queue accessibility, and job-history
  TTL remain pending in `tests/CHARACTERIZATION.md`.
- Constitution 2.1 now permits the narrowly scoped Tauri package and one supervised local
  application boundary; the final status and remaining implementation gaps are in the Phase 9
  report.
