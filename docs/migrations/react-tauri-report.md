# React and Tauri migration report

**Audit date:** 2026-09-02
**Overall status:** **COMPLETE**

This follow-up resolved the governance, React navigation/dialog, IPC streaming/cancellation,
packaged sidecar, worker lifecycle, native-test-harness, runtime god-file, runtime style ownership,
and AI command-coverage gaps from the first audit. Every migration gate now passes. Repeated real
macOS WebView runs exposed and verified fixes for asynchronous React navigation, background-job
quiescence, SQLite writer contention, optional semantic fallback, and frozen-sidecar shutdown.

## 1–8. Architecture

1. **Before:** a 240 KB static page owned markup, state, transport, rendering, styling, and every
   workflow interaction; FastAPI was the only delivery boundary.
2. **After:** React owns application composition, navigation, primary screens, dialogs, and status
   docks. Features depend on `ApplicationClient`; HTTP and Tauri implement it. Eleven ordered,
   domain-named compatibility controllers preserve mature dynamic interaction semantics without
   duplicating domain logic; no controller exceeds 60 KB. Further conversion of these controllers
   to declarative hooks is a redesign opportunity rather than a behavior-parity requirement.
3. **React structure:** `app/` composition/navigation; `features/` Workbench, Search, Compass,
   Settings, and overlays; `components/` reusable controls; `services/` transports; `types/` typed
   boundaries; `theme/` tokens and shared styles.
4. **Theme:** `tokens.css` owns semantic colors, typography, spacing, radii, borders, shadows,
   controls, layers, motion, focus, and disabled state. Bundled fonts and shared layout styles are
   isolated from feature behavior.
5. **Application client:** normal queries use a typed request command. Chat, SSE, and any request
   with an `AbortSignal` use a request-ID channel with incremental byte chunks and native cancel.
6. **Tauri/Rust:** the shell validates loopback origin, route family, path, method, and safe headers;
   owns sidecar startup/readiness/process-group shutdown; and exposes thin request, stream, cancel,
   and test-only reporting commands.
7. **Python retained:** the stable workflow, retrieval, jobs, localization, vault, patch, archive,
   provider, and persistence implementation. PyInstaller packages it as a 20 MB sidecar. Fast and
   durable workers run as threads inside that one shell-supervised process, with one isolated
   application/SQLite connection per worker. The removal path is
   domain-by-domain extraction behind `ApplicationClient`, with behavior tests moved before each
   extraction; no full rewrite is planned.
8. **FastAPI retained:** all routes support the existing loopback web product and the internal
   sidecar boundary. The desktop UI never addresses HTTP or Python directly.

## 9. API → application → Tauri map

| API family | Classification | Application capability | Tauri adapter | Retention |
|---|---|---|---|---|
| Capture/item/transition/workbench | command/query | workflow lifecycle | validated request | retained internal/web |
| Chat | streaming command | Explore conversation | channel + request ID + cancel | retained internal/web |
| Draft/refine/conflict/translation/completion jobs | background command/query | durable AI work | enqueue/query/cancel through request adapter | retained internal/web |
| Work Log/checklist/comments | command/query | Solution evidence | validated request | retained internal/web |
| Completion/archive/patch/lineage | command/query | publish and reversible vault mutation | validated request | retained internal/web |
| Search/index/Knowledge | query/command/background | vault retrieval | request; cancellable channel when signalled | retained internal/web |
| Dashboard/goals | query/command | Compass | validated request | retained internal/web |
| Jobs/notifications | query/command/stream | durable lifecycle | request + SSE channel | retained internal/web |
| Provider/locale/i18n | query/command | configuration/localization | validated request | retained internal/web |
| Health/index events | lifecycle/query/stream | readiness/change signal | request + SSE channel | retained internal/web |

## 10. Existing API test → command test map

All API tests remain. Rust integration tests use the real Python application and isolated SQLite and
vault paths; they cover the broad command surface and real Fast Queue Chat and durable AI work
against a deterministic OpenAI-compatible server.

| Existing API behavior | New command evidence | Status |
|---|---|---|
| Health, Capture, board, validation | real sidecar create/query/422 | PASS |
| Index/search/filesystem citation | real vault file/index/search | PASS |
| Locale resource/setting persistence | native gateway save/read | PASS |
| Capture promotion and inbox lineage | native gateway promote/board | PASS |
| Manual update and item detail | native gateway update/read | PASS |
| Soft delete/restore | native gateway delete/restore/reload | PASS |
| Problem approval and vault projection | native gateway approve/project | PASS |
| Solution create/conflict/approve | native gateway lifecycle | PASS |
| Work Log/comment/checklist persistence | native gateway create/update/read | PASS |
| Job enqueue/list/cancel | native gateway durable job lifecycle | PASS |
| Knowledge and provider queries | native gateway real runtime | PASS |
| Conflict-resolution validation, complete set, restore, and stale rejection | real sidecar/provider plus retained API fixtures | PASS |
| Bilingual AI draft/refinement and Knowledge translation | real sidecar, workers, provider double, persistence | PASS |
| Image and derived translation | real sidecar, workers, provider double, localized persistence | PASS |
| Completion review/report | real sidecar, workers, provider double, notification, vault output | PASS |
| Patch/completion-document conflicts and Lineage | real sidecar filesystem/Lineage plus retained correction fixtures | PASS |
| Notification publication/read/dismiss | real sidecar job notification/read plus application fixtures | PASS |
| Provider-backed Chat | real sidecar, worker, deterministic provider, and streamed native gateway | PASS |
| Provider-backed draft/refine/failure/retry | real sidecar, workers, deterministic provider, manual recovery | PASS |
| Provider timeout variants | real sidecar timeout mapped to retryable command state | PASS |

## 11. BDD coverage matrix

| Behavior | BDD scenario | Existing API test | React UI | Tauri command | Desktop E2E | Status |
|---|---|---|---|---|---|---|
| Launch/initial load | Given launch, when ready, then data loads | PASS | PASS | PASS | PASS | PASS |
| Navigation | Given a screen, when navigation is selected, then one view is active | N/A | PASS | N/A | PASS | PASS |
| Capture create/validation/persist | Given text/empty text, when saved, then persist/reject | PASS | PASS | PASS | PASS | PASS |
| Workflow transitions | Given gates, when transitioned, then eligibility is preserved | PASS | PASS | PASS | PASS | PASS |
| Explore stream/cancel | Given Chat, when chunks arrive/close, then stream/cancel | PASS | PASS | PASS | PASS | PASS |
| Refinement/stale result | Given source context, when result returns, then only current applies | PASS | PASS | PASS | PASS | PASS |
| Conflict resolution | Given findings, when resolved, then complete human set is required | PASS | PASS | PASS | PASS | PASS |
| Work Log/checklist | Given evidence, when saved, then source and checks persist | PASS | PASS | PASS | PASS | PASS |
| Completion/archive/patch | Given changed/unchanged files, when published, then write/block | PASS | PASS | PASS | PASS | PASS |
| Search/index/Knowledge | Given a vault note, when searched/read, then evidence returns | PASS | PASS | PASS | PASS | PASS |
| Locale restoration | Given a locale, when changed/reloaded, then authored state remains | PASS | PASS | PASS | PASS | PASS |
| Queue/notifications | Given durable work, when state changes, then progress/actions appear | PASS | PASS | PASS | PASS | PASS |
| Compass/settings | Given goals/config, when saved, then values persist safely | PASS | PASS | PASS | PASS | PASS |
| Relaunch persistence | Given a Capture, when the desktop process relaunches, then it restores | PASS | PASS | PASS | PASS | PASS |

## 12–21. Test and removal ledger

12. **React tests added:** 10 total after the streaming, cancellation, HTTP-adapter, shell, and
    React-owned navigation cases.
13. **Tauri command tests added:** 12 total: six Rust unit/security/cancellation tests and six
    real-sidecar integration tests, including broad workflow/state/side-effect coverage and real
    Fast Queue Chat, bilingual draft/refine, translation, completion, notification, failure, and
    retry behavior against a deterministic OpenAI-compatible double.
14. **Desktop E2E added:** `scripts/run_desktop_e2e.py` launches the release `.app` with isolated
    vault/database state and a deterministic external-provider double. Its in-WebView scenario
    performs React navigation and Capture submission; streamed Chat and durable AI work; workflow,
    Work Log, checklist, completion, filesystem projection, Lineage, follow-up, delete/restore,
    Search, locale restoration, secret-safe configuration, full sidecar shutdown, and persistence
    across a second desktop process through the real Tauri command and packaged sidecar stack.
15. **Existing tests retained:** every Python unit, integration, API, architecture, transition, and
    browser test.
16. **Tests removed:** none.
17. **FastAPI endpoints removed:** none; the web mode and packaged Python application boundary both
    intentionally use them.
18. **FastAPI endpoints retained:** all, loopback-only. No external/public remote API was identified.
19. **Temporary migration shims:** eleven domain-named files under `llm_wiki/static/runtime/`
    preserve imperative behavior. The 181 KB god file, raw overlay HTML, runtime-injected styles,
    direct `EventSource`, inline HTML bootstrap, separately launched sidecar, and buffered Chat IPC
    shims were removed.
20. **Known limitations:** behavior-preserving controllers remain imperative and have a documented
    domain-by-domain React-hook removal path. This is a future redesign opportunity, not an
    incomplete migration gate. Semantic embeddings remain optional; their absence now completes as
    an explicit lexical-fallback result instead of a failed background job.
21. **Decisions:** MIG-001/002/004/005 used the requested recommended defaults and are resolved.
    MIG-003 selected the repository-owned real-WebView harness instead of adding Appium. No product
    decision or migration implementation item remains.

## 22–30. Final verification record

| Gate | Result |
|---|---|
| 22. Lint | PASS (Ruff, ESLint, Clippy with warnings denied) |
| 23. Typecheck | PASS (TypeScript and Rust check) |
| 24. Frontend production build | PASS |
| 25. Tauri build/check | PASS; `.app` contains native executable and packaged sidecar |
| 26. Unit/integration tests | PASS (195 Python + 10 React + 12 Rust) |
| 27. Existing API tests | PASS (included in 195 Python tests) |
| 28. Command tests | PASS (12 Rust tests; six use the real Python application) |
| 29. BDD/UI tests | PASS (37 browser scenarios plus React BDD tests) |
| 30. Desktop E2E | PASS twice consecutively against the release `.app`, packaged sidecar, real commands, deterministic provider, and full process relaunch |

## TODO / Decisions Needed From User

None.
