# MCP Workbench Bridge acceptance verification

**Date:** 2026-09-04  
**Environment:** macOS task worktree, isolated temporary SQLite databases and Vaults  
**Verdict:** macOS implementation gates passed; packaged Windows acceptance remains external

This record separates ordinary regression coverage from release acceptance. Historical failures
remain below for traceability; the latest checkpoint supersedes their verdicts.

## Executable release gates

### 2026-09-05 convergence checkpoint (current)

- Native application-boundary acceptance is **9/9 passed** and no longer ignored. React
  user-surface acceptance is **4/4 passed** and no longer opt-in. The actual Chat runtime suite adds
  exact Capture edit/reject/accept, consented checkpoints, stale-review renewal, and late-response
  suppression across Chat switches.
- The full ordinary Rust suite, 10,000-event deterministic replay, frontend suite, typecheck,
  ESLint, runtime/application-boundary checks, formatting, and clippy pass on macOS.
- A debug packaged `.app` E2E passed using isolated data. It launched a real
  `llm-wiki-desktop --mcp` stdio child, forwarded to the GUI-owned protected socket, accepted an
  external Capture Elicitation, resumed it in the real in-app Chat, accepted a Problem card,
  revoked the connection, and verified the child was denied. The same run covered Work Log,
  separate legacy completion/publication, semantic search, and full process relaunch.
- Capture creation, governed workflow transitions, private Knowledge draft save, exact publication,
  and recoverable publication withdrawal all use separate, payload-bound decisions. Reviewed
  publication survives a file/DB interruption through durable jobs; externally changed files are
  never overwritten or withdrawn.
- The GUI process is the sole SQLite/Vault/application-service owner. The stdio bridge ignores DB
  and Vault environment paths. Topic membership is explicit and exact, scoped truncation is
  computed after filtering, and removed membership revokes evidence handles.
- Windows named-pipe ACL code is compiled by the cross-platform workflow, but an installed Windows
  package run cannot be produced on this macOS host. A local MSVC-target check reached native C
  dependencies but could not continue without Windows SDK headers (`assert.h`); it is not counted
  as Windows evidence. T104 remains an external release gate and is not reported as passed.

### Earlier 2026-09-05 implementation checkpoint (historical)

- Native acceptance: **5 passed / 3 failed**, including the new pre-Solution checkpoint
  lifecycle regression. The original seven probes now pass **4/7**. Run with
  `--include-ignored`, because fixed native probes now also run in the ordinary suite.
- React acceptance: **2 passed / 2 failed**. Draft review versus publish buttons and Korean
  component labels pass; live Chat milestone wiring and Workbench rendering still fail.
- Ordinary Rust suite: **60 passed**, including 10,000 reordered events. Final IPC changes
  additionally passed the five-test transport suite. Frontend: **29 passed**; typecheck, ESLint,
  runtime/application-boundary verifiers, and production web-asset build passed.
- A subsequent concurrent-suite run exceeded the 500 ms projection budget; the isolated latency
  test passed. Dependency wakeup was then moved from every event to once per drain batch, and
  unrelated event kinds no longer query the waiting-checkpoint result. Keep the original latency
  threshold; a passing unit run is not a substitute for packaged latency acceptance.
- IPC refuses the reserved native identity, unknown/revoked grants, non-socket endpoints and
  endpoints belonging to another user. Identification has a five-second deadline; connection
  tasks are bounded and owned by the listener. Existing endpoint directories must already be
  private; the listener does not chmod arbitrary caller-selected existing directories.
- Checkpoint reads retain structured fields from the immutable event and server-derived origin.
  Accepted pre-Solution checkpoints wait without retry exhaustion, then materialize once without
  rewinding stream watermarks. MCP checkpoint schemas now accept evidence/artifact/validation fields.
- Lexical scope filtering precedes result limits and truncation. Semantic freshness/revision
  metadata is scoped and result overflow is reported. **Topic membership is still substring-based**;
  this change does not close T099.
- Capture consent, backend publication authority, current selection, live UI wiring, canonical
  lifecycle parity, authoritative topic membership, crash-consistent publication and external
  host/platform acceptance remain open. Neither main merge nor installed-app replacement occurred.
- All 105 task entries are retained. T089–T104 remain open; these improvements do not satisfy
  their complete acceptance criteria.

### Original baseline (2026-09-04; historical failures below)

Run the native application-boundary probes explicitly:

```bash
cargo test --manifest-path src-tauri/Cargo.toml --test work_tracking_release_acceptance -- --ignored --nocapture
```

Result: **1 passed, 6 failed**.

Run the real React-surface probes explicitly:

```bash
VITE_RUN_WORK_TRACKING_ACCEPTANCE=1 npx vitest run frontend/src/features/chat/WorkTrackingReleaseAcceptance.test.tsx --reporter=dot
```

Result: **0 passed, 4 failed**.

The probes are opt-in so known release gaps do not turn the ordinary regression suite into a
permanent red build. They must all pass before this feature can be called complete.

## Reproduced failures

| Area                  | Observed result                                                           | Required result                                                            |
| --------------------- | ------------------------------------------------------------------------- | -------------------------------------------------------------------------- |
| Consent               | `captures = 1` immediately after `work_tracking.open`                     | No durable Capture before an exact accept/edit decision                    |
| Work Log              | `evidenceRefs = null` after accepting a structured checkpoint             | Preserve evidence, validation, artifacts, decisions, origin, and timestamp |
| Publication           | `work_tracking.knowledge.publish` wrote a Knowledge file directly         | A separate interface-bound publication challenge must be accepted          |
| Provenance            | native open persisted `source_interface = external_mcp_chat`              | In-app Chat must retain distinct provenance                                |
| Workbench persistence | linked `item.update` advanced the tracked session head                    | Passed at the persistence boundary                                         |
| Current state         | `activeSelection = null`                                                  | Return a resumable selection with linked work and next decision            |
| Topic privacy         | scoped result returned `truncated = true` because of out-of-scope matches | Scope filtering must precede truncation calculation                        |
| In-app Chat           | tracked-work region contained no milestone card                           | Render live proposal/review cards with actions                             |
| Workbench UI          | refreshed title `Tracked from Chat` was not rendered                      | Show bounded current tracked state                                         |
| Draft review          | offered Knowledge displayed `Publish Knowledge` directly                  | Require review of the exact draft before publication                       |
| Localization          | Korean surface displayed `Reject`, `Edit`, and `Accept`                   | Use localized Korean actions                                               |

## Acceptance-scenario matrix

Evidence labels:

- **Executed pass/fail:** an executable acceptance probe reached the relevant boundary.
- **Partial:** lower-level coverage exists, but the complete specified user workflow was not proven.
- **External gate:** the required host/platform was unavailable in this macOS repository session.

| Scenario                                              | Evidence                            | Result                                                                                                                   |
| ----------------------------------------------------- | ----------------------------------- | ------------------------------------------------------------------------------------------------------------------------ |
| US1/AC1 consented Capture preview                     | Native probe                        | **Executed fail** — opening persists Capture immediately                                                                 |
| US1/AC2 edit/reject exact Capture proposal            | React/native inspection and probe   | **Executed fail** — no live card; external open has no equivalent decision flow                                          |
| US1/AC3 Problem/Solution proposal and conflict review | React surface probe                 | **Executed fail** — no in-app tracked-work card is rendered                                                              |
| US1/AC4 ten equivalent repeated operations            | Existing stdio idempotency tests    | **Partial** — isolated deduplication exists, not the full lifecycle repeated ten times                                   |
| US2/AC1 structured checkpoint in Solution Work Log    | Native probe                        | **Executed fail** — structured evidence is discarded                                                                     |
| US2/AC2 checkpoint before a Solution                  | Existing domain behavior            | **Partial** — pending attachment logic is not proven through both Chat surfaces                                          |
| US2/AC3 restart and natural resume                    | Existing restart tests              | **Partial** — persistence exists, complete Chat summary/resume does not                                                  |
| US2/AC4 edit/reject checkpoint                        | React surface probe                 | **Executed fail** — no live decision card exists                                                                         |
| US3/AC1 incomplete completion remains open            | Existing state tests                | **Partial** — exact user-visible flow is not covered                                                                     |
| US3/AC2 completion edit and re-preview                | React/native flow                   | **Executed fail** — exact edit/re-preview interface is absent                                                            |
| US3/AC3 evidence-backed completion gate               | Existing lower-level checks         | **Partial/fail** — complete Lineage and selected accepted evidence are not enforced end to end                           |
| US3/AC4 Skill asks whether to publish                 | Codex/ChatGPT Desktop               | **External gate** — real Skill host is required                                                                          |
| US3/AC5 decline publication                           | Native probe                        | **Executed fail** — direct publication bypasses a user challenge                                                         |
| US3/AC6 exact reviewed draft publication              | Native probe                        | **Executed fail** — append rejects publication authority, but the publish operation remains direct                       |
| US4/AC1 external Chat lifecycle parity                | Native probes                       | **Executed fail** — consent and publication boundaries fail                                                              |
| US4/AC2 in-app Chat lifecycle parity                  | React probe                         | **Executed fail** — the card region is empty                                                                             |
| US4/AC3 distinct provenance with canonical state      | Native probe                        | **Executed fail** — native open is labeled as external MCP Chat                                                          |
| US4/AC4 Workbench mutation reflected in tracked state | SQLite trigger and native probe     | **Partial** — session head advances, but provenance/pending decisions are not rendered through one complete service path |
| US5/AC1 enter Workbench during Chat work              | React Workbench probe               | **Executed fail** — refreshed tracked state is not displayed                                                             |
| US5/AC2 edit in Workbench, resume in Chat             | Native persistence probe            | **Executed pass at persistence boundary** — head advances; Chat refresh is not proven end to end                         |
| US5/AC3 simultaneous non-overlap/conflict recovery    | Existing CAS tests                  | **Partial** — no complete refresh, preserve-input, compare, retry, or renewed-review flow                                |
| US5/AC4 already-decided proposal                      | Stored decisions plus UI inspection | **Partial/fail** — decision data exists but is not rendered                                                              |
| US6/AC1 Skill presents compact Workbench summary      | Codex/ChatGPT Desktop               | **External gate**                                                                                                        |
| US6/AC2 topic/current-work resume detail              | Native probe                        | **Executed fail** — active selection is null and required detail is absent                                               |
| US6/AC3 stable pagination over large Workbench        | Real Skill conversation             | **External gate**                                                                                                        |
| US6/AC4 link current work with exact revisions        | Existing challenge checks           | **Partial/fail** — entity and workspace revisions are not bound                                                          |
| US6/AC5 cursor recovery in conversation               | Existing cursor code                | **Partial** — real conversational recovery is not executable here                                                        |
| US7/AC1 current-session lexical and semantic review   | Current orchestration inspection    | **Partial/fail** — tools exist, but a hidden server-side model job remains                                               |
| US7/AC2 cited resolution in current Chat              | Codex/ChatGPT Desktop               | **External gate**                                                                                                        |
| US7/AC3 topic-scoped privacy and metadata             | Native probe                        | **Executed fail** — out-of-scope overflow leaks through `truncated`                                                      |
| US7/AC4 semantic fallback disclosure                  | Existing backend fallback test      | **Partial** — AI disclosure is only a manual Skill case                                                                  |

## Remaining external gates (current)

The following cannot be claimed from this run:

- a newly started Codex task loading the locally installed plugin and executing its packaged Skill;
- a real ChatGPT Desktop host executing the packaged Skill and elicitation flow;
- packaged Windows installation, stdio launch, Unicode Vault paths, restart, and revocation;
- installed-app smoke after merging the verified release bundle.

## Final macOS package result — 2026-09-05

`npm run tauri:build` produced the release `.app`, followed by one `npm run test:desktop` run.
The release bundle passed the same real stdio-child → GUI-owner → in-app Chat acceptance path,
connection revocation, workflow/Work Log/search checks, and full process relaunch. The test used an
isolated temporary Vault, database, settings home, and deterministic local provider; no personal
Vault data was used.

The convergence tasks in [tasks.md](tasks.md#phase-11-convergence) remain the authoritative list.
T104 is intentionally left open until equivalent installed Windows evidence exists.
