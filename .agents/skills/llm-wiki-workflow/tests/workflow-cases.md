# Workflow behavior cases

These fixtures are manual acceptance cases for Codex and ChatGPT desktop. Responses must stay
conversational and must not expose resource URIs, record IDs, revisions, or raw MCP payloads.

## Concise resume

- User: `뭐 하고 있었지?`
- Resource: one current tracked session bound to a Task, its current revision and Work Log.
- Expected: where the work stands, the latest meaningful change, and one next action in at most
  three short paragraphs. It does not fetch the whole Workbench.

## Ambiguous current selection

- User: `이거 이어서 하자.`
- Resource: no active selection and three recently active sessions.
- Expected: two or three Task title/status choices and one question. It does not select by similarity.

## Whole Workbench

- User: `전체 Workbench 상태를 빠짐없이 정리해줘.`
- Resource: multiple stable overview pages.
- Expected: every page is consumed at one snapshot revision; the response gives counts, every
  attention item, all active Tasks, lightweight Captures, and recent Task completions. Tasks without
  a Chat session remain visible within the granted scope.

## Continue an existing Task without Capture

- User: `이 Task를 Chat에서 이어서 하자.`
- Resource: a visible Task with Work Log and exact Problem links, without a Capture or Chat session.
- Expected: read bounded Task context and present an exact continuation preview. Rejection creates
  no session. Acceptance binds one new session to the existing Task without inventing a Capture,
  Problem or Task. Attachments remain local; truncated context is stated.

## Conflict review

- User: `기존 Knowledge와 충돌하는지 봐줘.`
- Resource: lexical evidence succeeds; semantic index reports lagging.
- Expected: the current Chat AI compares exact returned passages, states that conceptual coverage
  is incomplete, cites valid evidence, and does not mark the conflict resolved.

## Completion and publication

- User accepts the exact `task.complete` review.
- Expected: report the canonical Task completion, then ask once `Knowledge로 발행할까요?`.
  Completion does not resolve linked Problems or publish Knowledge. A deferral creates no
  draft/file and the same completion revision is not prompted again. A Task completed in the
  desktop can also prepare a private draft without an invented Chat completion event.

## Exact Problem resolution and Knowledge publication

- User separately requests resolution of a linked Problem or publication of a private draft.
- Expected: preview the exact Problem revision or exact draft body and source hash. Only the
  corresponding accepted review performs that action. Changed source requires a fresh preview;
  rejection remains possible. Historical publication records remain readable and recoverable
  without reopening retired Capture → Problem → Solution actions.

## Revoked or insufficient scope

- Resource/tool returns `not_found_or_not_visible`.
- Expected: explain that this connection cannot access the requested context. Do not infer whether
  the record exists and do not suggest a secret identifier as a workaround.
