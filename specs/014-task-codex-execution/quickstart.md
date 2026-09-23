# Quickstart: Task Codex Execution Verification

**Current status**: Implementation, focused automated checks, isolated actual Codex acceptance, the selected packaged scenario, and representative actual macOS UI review passed. The design review and rendered checks are separate; residual coverage limits are listed below.

## Focused automated checks

Run separately:

```bash
npm test
cargo test --manifest-path src-tauri/Cargo.toml
git diff --check
```

Cover migration preservation, replay/conflict, one active Run, dispatch gaps, ownership, stale formal responses, blocking/nonblocking and free-text/multi-question inputs, failed-answer retention, interrupt race, identical deltas, item replay, redaction, manual Work Log preservation, log-only retry, no Task completion, and 100 ms p95 normal projection.

2026-09-22 native service evidence:

- `cargo test --manifest-path src-tauri/Cargo.toml task_execution_service::tests`: 8 passed. This covers atomic creation, 100 repeated submission replays, one active Run, wrong/deleted Task ownership, completed-item replay and ordering, final-phase report selection, credential-pattern redaction, structured-response generation/CAS/options/free text/secret exclusion, retained proposed answers, terminal Run invalidation of unanswered requests, globally increasing connection generations, connection-loss classification, Work Log-only failure and repair without rerun, unchanged manual records and Task state, and invalid folder handling.
- Before integration with the released main v13 Task journey cache, the focused then-v14 migration tests passed. They showed that populated session IDs, entry attachment bytes, manual Work Log body/comment, and foreign keys survived the execution migration, and that an injected collision rolled back every execution-schema alteration before an explicit retry. After integration, sessions and execution moved to v14 and v15 respectively; the current migration checks also cover populated main-v13 journey/manual data through v15.
- On the integrated tree, all three current migration checks passed as part of the full native suite: populated main-v13→v15 journey/manual data preservation, populated v14→v15 session/attachment/manual-data preservation, and v15 collision rollback plus retry.
- `cargo test --manifest-path src-tauri/Cargo.toml --test task_tracking_v9_migration --test work_tracking_migrations`: 5 passed after the schema-version expectation advanced to 14.

2026-09-22 transport and frontend evidence:

- `cargo test --manifest-path src-tauri/Cargo.toml --lib codex_app_server::tests`: 3 passed. The controlled process covered initialization, serialized request/response routing, formal server requests, completed items, terminal events, safe errors, and shutdown.
- `cargo test --manifest-path src-tauri/Cargo.toml --lib`: 101 passed and 1 opt-in live test was skipped before the live run.
- `cargo test --manifest-path src-tauri/Cargo.toml`: 186 passed, 0 failed, and 1 opt-in authenticated live test ignored in the final uncontended suite. The run included the isolated 100 ms Task projection gate.
- `npm test`: 294 passed, including the expanded Task Work Session/TaskDetail boundary cases and 4 native execution-client tests; native-only runtime and application-boundary verification also passed.

These substitute-backed checks do not prove provider authentication or an actual model turn. The isolated check below supplies that evidence. Actual-provider interruption timing and Windows lifecycle remain unverified. Packaged and rendered evidence is recorded below.

## Isolated real Codex acceptance

Use a harmless repository under `.tmp/codex-live/`. Run one opt-in live integration against installed authenticated Codex:

1. Create Task A and existing session with the isolated folder.
2. Run a harmless instruction and observe a real thread/turn.
3. Run a related follow-up and prove same thread/new turn.
4. Reopen and confirm reports/evidence.
5. Confirm one existing Work Log item per Run and no Task completion.
6. Confirm Task B cannot access Task A execution state.

Record Codex version, isolated path, opaque identity equality evidence, and result. Fixture evidence cannot satisfy this gate.

2026-09-22 actual Codex evidence:

- `LLM_WIKI_LIVE_CODEX_TEST=1 cargo test --manifest-path src-tauri/Cargo.toml --lib native::task_execution_runtime::tests::live_two_turn_session_persists_results_and_work_log -- --ignored --nocapture`: 1 passed against installed authenticated Codex 0.154.0 using `gpt-5.6-luna` in a temporary isolated workspace.
- Two real turns used the same opaque thread identity and distinct Run/turn records. The second turn recalled the nonce supplied only in the first turn.
- Both model final reports and completed evidence were durable after reopening the service. Two existing Task Work Log rows projected successful execution reports, cross-Task access was rejected, and the Task remained incomplete.
- No tools or user-file mutations were requested in either model turn. Raw protocol, private paths, credentials, and opaque identities were not recorded here.

## Rendered review and targeted E2E

Review EN/KO and 640 px: empty, running, blocking/nonblocking request, multi-question options, free-text-only, submitting, answer failure/stale/answered, completion/failure/cancellation/interruption, missing report, link-back, keyboard/focus, and long output.

Before substantive UI implementation, perform the required **Astra High** UI/UX design review and incorporate findings in the interaction contract. After code and documentation stabilize, build release once. Use that artifact for separate rendered UI verification and **Luna** for only:

```bash
npm run test:desktop -- --scenario task-codex-execution
```

The controlled fixture covers native/UI lifecycle. The live integration proves actual Codex. Do not run full E2E without a concrete broad risk. Report unavailable Windows, picker, resize, auth, approval, or interruption checks.

## Design review evidence

2026-09-21: Astra High reviewed the existing surface and proposed interaction design. Six high-priority gaps were resolved in the interaction contract and implementation requirements: settings preparation/proximity, historical Runs, answer ownership/persistence, request hierarchy, uncertain retry behavior, and bidirectional tab/focus/live announcements. Explicit thread preparation with no turn/Run/Work Log was accepted. This was design review only; it supplied no rendered UI or runtime evidence. Subsequent runtime and separately performed rendered results are recorded in this document.

## Projection measurement isolation

The 100 ms p95 wall-clock budget is measured by `cargo test --manifest-path src-tauri/Cargo.toml --test task_projection_performance -- --nocapture`, also included automatically in the full Cargo suite. Its fixture contains ten completed execution-linked Work Logs, 250 provider items, and a manual Work Log, and reads through the public native Task boundary. A separate test binary isolates the measurement from concurrent unit-test workloads; the 100 ms limit is unchanged. 2026-09-22 focused result: 1 passed, p95 11.259417 ms. The prior empty-fixture unit measurement was 4.800083 ms in isolation; the 133–271 ms failures occurred under concurrent full-suite load. No application performance change or threshold relaxation was needed.

## Packaged and rendered verification progress

- First release build succeeded. Luna's selected packaged scenario failed before execution because its settings-save helper waited on a disabled Save button that success had removed. The retained DB proved the requested model and folder were saved. Astra corrected the harness to observe live completion state and added a regression; `npm test` then passed 283 tests with auxiliary checks, typecheck, and whitespace validation. The rebuilt second run stopped before execution because the scenario clicked Prepare without registering its rendered observation. Astra corrected rendered observation ordering for six controls, added persisted-response assertions, processed initial subscription state, and corrected the reopen assertion to use Run history. The harness regression suite and full frontend check passed 285 tests; the next run reached native execution. Second-run evidence: `.tmp/task-codex-execution-artifacts-2/`.
- Actual CUA review used a separately signed artifact copy with isolated test data, preserving the user's running app. Observed English/Korean Sessions, saved model/folder, clear separate note/run actions, visible keyboard focus, keyboard note save with mixed-language content, and record restoration after closing/reopening Task detail. No automatic execution occurred.
- Resized native window review reached the application's configured 900 px minimum width. The 640 px case cannot be exercised as a native window with the current minimum; it remains unverified. Subsequent execution/result and Work Log review is recorded in final acceptance below.

- Third selected run reached native execution and approval plus two structured-question controls, then timed out before first-turn completion. Retained state showed approval accepted but the question still pending; Astra reproduced and corrected the harness interaction/React-commit boundary. Evidence: `.tmp/task-codex-execution-artifacts-3/`.
- Actual restart review of that isolated state exposed a product defect: the Run correctly became Needs attention, but its old pending question still offered enabled response controls. Native recovery and defensive UI invalidation were corrected and regression-tested. The same review confirmed saved Run history, recorded approval, missing-final-report and uncertain-outcome copy, and the session-to-Work-Log tab/scroll route. A later final-artifact review verified reverse-link heading focus, as recorded below.

- Final UI regression check after the restart guard: `npm test` passed 41 files / 294 tests plus auxiliary checks; typecheck and whitespace checks passed. Tests cover multi-question aggregation, optional-other input, terminal request read-only behavior, saved-state reconnect, live-status/recovery labels, and exact bidirectional DOM focus. JSDOM focus evidence does not establish native accessibility-tree behavior.

- Startup recovery now atomically marks actionable formal requests stale and synchronizes recovered Work Log projections. The UI also makes requests read-only for terminal Runs and no longer describes answered requests as waiting. Native service tests: 13 passed; transport tests: 6 passed; final full Cargo: 186 passed, 0 failed, 1 ignored live opt-in.
- Work Log now labels localized lifecycle status, model-authored report versus observed evidence, unavailable final reports, artifacts, and unresolved limitations. Final frontend verification: 41 files / 294 tests passed, typecheck and whitespace checks passed.

## Final acceptance and remaining limits (2026-09-22)

- Final signed release build passed. Luna ran only `npm run test:desktop -- --scenario task-codex-execution`; it passed. Final evidence: `.tmp/task-codex-execution-artifacts-5/results.json` and `interactive-coverage.json`; isolated retained state: `.tmp/desktop-e2e-task-codex-execution-4c1Vgc/`.
- The selected controlled scenario proved preparation, two distinct Runs on the exact same thread, provider-derived approval, option plus free-text multi-question submission, reports, exactly two canonical Work Log projections, Task isolation, unchanged Task state, and detach/reopen restoration. Six targeted execution controls were rendered, exercised, and effect-asserted. This was a fixture-backed native/UI test, not actual-provider evidence; the separate authenticated two-turn test supplies that evidence.
- No full E2E suite was run. The selected scenario covers the material UI/native persistence and formal-response risk left after unit/integration tests. Additional builds were driven by concrete harness failures, the observed recovery defect, and final summary display cleanup; no blind repetition or release installation occurred.
- Actual CUA review used a separate signed bundle identifier and isolated data, preserving the installed user application. It observed EN/KO sessions, keyboard note save and reopen, the 900 px native minimum width, two restored completed Runs, historical Run selection, read-only recorded approval and two-part answer, final report and observed evidence, the compact final Work Log summary, and both navigation directions. Returning from Work Log focused the exact Run heading in the native accessibility tree.
- A separate real application restart of interrupted fixture data verified the corrected Needs attention state, stale question with no response controls, retained accepted approval, missing-report message, and no permanently pending Work Log sync. These are real rendered/restart observations with controlled provider data.
- CUA screenshots were inspected in-session, not saved as screenshot files. JSDOM additionally covers live progress labels, multi-question/other answers, submitting/answered/error/stale handling, terminal read-only guards, bidirectional focus, and saved-state reconnect within one second.
- Remaining coverage limits: native 640 px width is blocked by the existing 900 px minimum; Windows was not run; actual authenticated-provider approval, interruption race, and auth failure were not induced. Every transient formal-request/error/cancellation state was not individually inspected in CUA; those states have automated coverage. The native Work-panel accessibility tree was intermittently absent after a deep link, while direct tab access and final inspection exposed the controls; a full assistive-technology audit is not claimed.
- Existing saved attachments remain valid note records; this increment does not send them to Codex. No Task completion, Problem resolution, or Knowledge publication is automatic.
