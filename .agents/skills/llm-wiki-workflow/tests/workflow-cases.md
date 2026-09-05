# Workflow behavior cases

These fixtures are manual acceptance cases for Codex and ChatGPT desktop. Responses must stay
conversational and must not expose resource URIs, record IDs, revisions, or raw MCP payloads.

## Concise resume

- User: `뭐 하고 있었지?`
- Resource: one current tracked session with an approved Solution and one recent checkpoint.
- Expected: where the work stands, the latest meaningful change, and one next action in at most
  three short paragraphs. It does not fetch the whole Workbench.

## Ambiguous current selection

- User: `이거 이어서 하자.`
- Resource: no active selection and three recently active sessions.
- Expected: two or three title/stage choices and one question. It does not select by similarity.

## Whole Workbench

- User: `전체 Workbench 상태를 빠짐없이 정리해줘.`
- Resource: multiple stable overview pages.
- Expected: every page is consumed at one snapshot revision; the response gives counts, every
  attention item, all active work, compact coverage of shaping work, and recent completions.

## Conflict review

- User: `기존 Knowledge와 충돌하는지 봐줘.`
- Resource: lexical evidence succeeds; semantic index reports lagging.
- Expected: the current Chat AI compares exact returned passages, states that conceptual coverage
  is incomplete, cites valid evidence, and does not mark the conflict resolved.

## Completion and publication

- User approves `verify_and_complete`.
- Expected: Completed Work is reported first, then the Skill asks once `Knowledge로 발행할까요?`.
  A deferral creates no draft/file and the same completion revision is not prompted again.

## Revoked or insufficient scope

- Resource/tool returns `not_found_or_not_visible`.
- Expected: explain that this connection cannot access the requested context. Do not infer whether
  the record exists and do not suggest a secret identifier as a workaround.
