# Quickstart: Validate Two Chat Inputs Without Workbench

## Prerequisites

- macOS or Windows with repository Tauri prerequisites
- Temporary Vault, settings, and database; never use personal data
- Deterministic in-app provider, fake MCP host, and pinned Inspector/conformance tooling
- One MCP 2026-07-28 host fixture with Elicitation/MRTR and one legacy/no-Elicitation fixture
- Fixtures for another connection's session, malformed events, stale challenges, and closed work

## 1. Run stable checks

Run separately from the feature worktree:

```bash
npm test
```

```bash
npm run typecheck
```

```bash
npm run lint
```

```bash
cargo test --manifest-path src-tauri/Cargo.toml
```

```bash
git diff --check
```

Expected: shared domain, both Chat adapters, Elicitation, idempotency, revision conflicts, existing
Chat streaming/history, active Workbench continuation, and workflow tests pass.

## 2. Configure an external Chat connection

Start LLM Wiki, create a `work_tracking` connection in the isolated app settings, and copy its
installed-executable stdio configuration into the supported host. The stdio child connects to the
running GUI's protected local IPC endpoint and fails without opening persistence if the GUI is
absent.

Expected:

- MCP was disabled before the explicit grant;
- setup names the required 2026-07-28 Elicitation capability;
- setup says the lifecycle finishes in Chat and Workbench is optional;
- setup identifies Codex and ChatGPT desktop as Phase 1 hosts and does not advertise ChatGPT web;
- no API key, Vault path, transcript, or bearer secret appears in configuration;
- revocation remains available.

## 3. Inspect the protocol surface

Use the pinned Inspector wrapper to inspect tools and resources.

Expected:

- tools include work open/append/advance, separate lexical/semantic Vault search, bounded evidence
  read, Knowledge draft save, and Knowledge publish;
- session resources expose only connection-owned sessions, while `llm-wiki://workbench/current`
  appears only with the separately granted current-Workbench scope;
- `llm-wiki://workbench/overview` is separately callable for a stable paginated full-board snapshot;
- arbitrary Vault access, generic workflow tools, remote transport, and ChatGPT-web setup are absent;
- an architecture check fails if the MCP server or bridge imports SQLite/repository/schema/database-path/Vault adapters, or if `run_mcp` composes persistence instead of forwarding to the GUI;
- stdout contains protocol JSON only;
- tool annotations never substitute for authorization or user consent.

## 4. Complete a lifecycle in external Chat only

Do not open Workbench. In the supported external Chat:

1. Ask to track the conversation and accept the exact Capture preview.
2. Let the assistant append a Problem draft; edit it in the Elicitation form and adopt it.
3. Approve the Problem, adopt a Solution, resolve any conflict, and approve the Solution through
   separate in-chat reviews.
4. Append three checkpoints and explicitly accept any evidence used for completion.
5. Review a completion proposal, reject it once, add follow-up work, then verify and complete it.
6. Verify that no Knowledge file exists. Let the Skill ask “Knowledge로 발행할까요?”, defer once,
   then later save/review an exact draft and publish it through the separate action.

Expected: one continuous Capture→Problem→Solution→Completed Work lineage, followed only after a
second decision by Knowledge publication. Deferral preserves completion and causes no repeated nag,
duplicate write, or instruction to open Workbench.

## 5. Verify Elicitation binding

For every governed action, inspect the first `InputRequiredResult`, the host-rendered form, and the
retry carrying `requestState` plus structured user response.

Expected:

- the current Chat shows the exact payload, target, evidence, conflicts, and consequence;
- payload substitution, changed action, wrong connection/session/principal, stale revision, expiry,
  cancellation, replay, and revocation all produce zero workflow mutations;
- a user edit that materially changes meaning receives a fresh exact preview;
- challenge consumption and workflow mutation commit atomically;
- `confirmed:true` without a valid challenge is rejected.

## 6. Resume entirely in external Chat

Restart the host and application, call open in resume mode, and read the session resource.

Expected: bounded state is sufficient for the assistant to explain the current Problem, Solution,
latest checkpoints, pending decisions, and next actions. The resource contains neither the full
transcript nor unrelated/private records.

## 7. Complete the equivalent lifecycle in in-app Chat

Use LLM Wiki's existing Chat composer and streaming response path. Start tracking from the current
conversation, then perform the same Capture, Problem, Solution, checkpoint, conflict, and completion
actions through inline milestone cards. Do not open Workbench.

Expected:

- composer, history, streaming, cancellation, context routing, and completed-work Chat still work;
- only explicit card actions advance workflow; model text alone cannot do so;
- cards show the same payload, evidence, conflicts, consequences, and choices as MCP Elicitation;
- the same domain states, gates, Work Log shape, and Completed Work lineage result;
- provenance is `in_app_chat` rather than `external_mcp_chat`/`mcp_elicitation`.

## 8. Run adapter parity tests

Replay a canonical scenario through both adapters with equivalent accepted payloads.

Expected: normalized Capture, Problem, Solution, Work Log, decision, completion, Knowledge draft,
and separate publication records match. Only connection, interface, Chat session, event,
decision-channel, and timing provenance differ. Concurrent updates use the same revision and
projection rules.

## 9. Validate the representative workflow Skill

Run the Skill validator, then exercise the equivalent Skill in Codex and ChatGPT desktop with: “뭐 하고
있었지?”, “이거 이어서 하자”, “진행 상황 남겨줘”, and “이제 끝난 것 같아”. Cover an existing
session handle, one active Workbench selection, multiple active items, stale state, and revoked
current-Workbench scope.

Expected:

- the Skill validates with no scaffold placeholders;
- it prefers an explicit session handle, then the bounded current Workbench resource;
- ordinary replies are concise conversational briefs, not board-shaped dumps;
- internal IDs, resource URIs, revisions, and empty stages stay hidden unless recovery requires them;
- it asks no more than one workflow question at a time;
- it communicates authoritative state, draft status, evidence, blockers, and next action accurately;
- it asks the user to choose among ambiguous items and performs no guessed link;
- it records only meaningful milestones and never treats model text as user approval;
- denied/revoked scope reveals no current-work contents.
- after completion it asks once whether to publish and never treats that question as permission;
- ordinary continuation stays session-scoped, topic scope is explicit, and whole Workbench is used
  only for the explicit overview request.

Also ask “Workbench 전체 상태를 정리해줘” with more than one resource page and mutate the board
between page reads.

Expected:

- the Skill retrieves every page from one snapshot before claiming complete coverage;
- the response starts with complete counts, then needs-attention, active Solutions, shaping work,
  recent completions, and one useful next question;
- every non-archived item is represented directly or by an explicit aggregate count;
- every blocker, conflict, and pending decision is surfaced;
- a changed/expired snapshot restarts once and never mixes revisions or silently drops overflow.

## 10. Test unsupported hosts, privacy, and revocation

Run the legacy/no-Elicitation host, cross-connection handles, oversized/transcript-like payloads,
absolute paths, malformed provenance, database failure, concurrent retries, and revocation during a
review.

Expected:

- the legacy host may record private proposals/checkpoints but receives `elicitation_required` for
  governed transitions and is not redirected to Workbench;
- inaccessible and absent sessions are indistinguishable;
- no partial, duplicate, cross-session, or post-revocation writes occur;
- logs contain no content, credentials, raw response, request-state plaintext, SQL, or internal path.

## 11. Validate current-session conflict review

From both Chat interfaces, request a conflict review against a selected topic. Run lexical search,
semantic search, and bounded evidence reads. Repeat with a lagging/unavailable semantic index,
fabricated evidence IDs, changed revisions, prompt-like text inside a note, and out-of-scope canary
passages.

Expected:

- the current Chat's selected AI performs synthesis; the application/MCP server makes zero model calls;
- every factual finding cites an exact returned passage/revision/hash;
- lexical remains available and semantic gaps yield `insufficient evidence`, never a false clear;
- no passage, count, or error leaks data outside the selected session/topic/Workbench scope;
- note content is treated as evidence, not executable instruction.

## 12. Switch Chat → Workbench → Chat

Start a tracked session in external Chat, create and approve the Problem, then optionally open
Workbench. Edit and approve the Solution, append progress, and return to the Chat to complete it.
Repeat with in-app Chat. Also start simultaneous edits from Chat and Workbench at the same revision.

Expected:

- Workbench immediately shows the same session, source, records, Work Log, evidence, and pending actions;
- Workbench can directly edit, review, approve, add progress, resolve conflicts, and complete work;
- each Workbench commit advances the same session head and is visible on the next Chat read;
- returning to Chat continues from the Workbench-updated state without import or synchronization;
- the first concurrent commit wins, while the stale surface preserves unsaved input and reconciles;
- an MCP challenge tied to the old revision becomes stale and cannot create a duplicate decision.
- passive refresh preserves unsent input, IME composition, focus/caret/selection, scroll anchor,
  disclosures, and streaming output;
- a stale save refreshes first and retries at most once only for unchanged non-overlapping intent.

## 13. Validate event ordering and projector recovery

Generate in-order, duplicate, equal-time, and late older events across independent session streams.
Crash a projector after claim and after Work Log insertion, then restart with both app and MCP
workers eligible to claim the same durable outbox item.

Expected:

- append/head/idempotency/outbox commit together before `accepted/queued` is returned;
- per-stream trusted `(occurredAt, sourceSequence, eventId)` ordering is deterministic;
- late events remain auditable as `ignored_late` but do not change current state, watermark,
  Lineage, Work Log, or completion evidence;
- leases recover after restart and unique provenance produces exactly one materialization.

## 14. Run performance, build, and packaged tests

Verify readiness under 1.5 seconds, durable append p95 under 50 ms, projection visibility within
500 ms for 95% of local runs, bounded context, and idle regression under 15%. Then run once:

```bash
npm run tauri:build
```

```bash
npm run test:desktop
```

Repeat external Chat, in-app Chat, Chat↔Workbench switching, concurrency, relaunch, revocation, and
Unicode-path checks on packaged macOS and Windows builds. The installed executable must work as the
stdio command without a Node/Python runtime or TCP/HTTP localhost listener. The protected Unix
domain socket or Windows named pipe is owned by the GUI process.

## Release verification record — 2026-09-05

- Native application-boundary acceptance: 9/9 passed and now runs in the ordinary Rust suite.
- React user-surface acceptance: 4/4 passed and now runs in `npm test`; the real runtime/React
  integration additionally covers exact Capture editing, checkpoint consent, stale-review renewal,
  and ignoring a response arriving after Chat switches.
- A debug packaged macOS E2E passed through a real stdio child, the GUI-owned private socket,
  external Capture Elicitation, in-app Problem card acceptance, connection revocation, and process
  relaunch. The final release-bundle run is recorded in
  [acceptance-verification.md](acceptance-verification.md).
- Windows packaging was not executable from this macOS host. CI/Windows acceptance must run the
  same stable commands, `scripts/package_windows.ps1`, packaged desktop E2E, and an installed
  `llm-wiki-desktop.exe --mcp --connection <id>` stdio smoke test with Unicode Vault paths.
