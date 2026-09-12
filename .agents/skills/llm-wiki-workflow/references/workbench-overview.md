# Whole Workbench Overview

Use this mode only when the user explicitly asks for the entire Workbench, overall status, all
current work, or a portfolio-style summary. An ordinary “continue” request should use the smaller
current-work projection instead.

## Retrieve a complete snapshot

Read `llm-wiki://workbench/overview` and follow its snapshot cursor until `nextCursor` is absent.
Keep the first page's `snapshotRevision` on every subsequent read. If the snapshot expires or
changes, restart once; do not combine pages from different revisions. Stop and explain briefly if a
second attempt cannot produce one consistent snapshot.

The overview covers every non-archived Workbench item visible to the granted one-request overview
scope. Reading it does not authorize changes or silently broaden later Chat turns. Do not fetch full
item/session detail unless the user selects an item or the overview marks it as requiring a decision.

## Present the whole state cleanly

Give complete coverage with bounded detail:

1. **At a glance** — counts for Captures, Tasks by state, Problem context, in-progress work,
   blocked/conflicted work, pending decisions, and recently completed work.
2. **Needs attention** — every blocker, conflict, stale item, or user decision currently requiring
   action. Put this before routine work even if it belongs to another stage.
3. **In progress** — every active Task, one compact line each: intended outcome, latest
   meaningful progress, and next step.
4. **Still shaping** — all Captures, Tasks, and optional Problem context grouped by lifecycle stage;
   use title plus a short reason/context, not full record bodies.
5. **Recently completed** — the bounded recent-completion list returned by the resource.

Every visible non-archived item must be represented either by its own compact line or by an explicit
stage/category count. Never silently omit a page or imply that a partial page is the whole
Workbench. When a section is empty, omit it instead of printing an empty heading.

Prefer lifecycle stage as the primary grouping. Mention Workbench category only when it helps
distinguish similarly named work or the user asks for a category view. Never rank the user, invent
priority, or interpret recency as importance.

End with one useful conversational question, normally which item the user wants to open or what
needs attention first. Do not end with a generic offer to help.

## Example shape

Adapt the language and omit irrelevant sections:

```markdown
Workbench 전체를 보면 Capture 3개, Task 2개, 진행 중인 작업 2개가 있어요. 지금 확인이
필요한 건 1건입니다.

**확인이 필요한 작업**

- 결제 오류 재현 — 검증 결과가 서로 달라 완료 판단이 보류되어 있어요.

**진행 중**

- MCP 지원 — 외부 Chat 연결 설계까지 완료했고, 다음은 도구 계약 구현입니다.
- 검색 개선 — 색인 성능을 측정 중이며 다음 단계는 Windows 결과 확인입니다.

**아직 구체화 중**

- Capture 3개: 모바일 메모 가져오기 외 2개
- Problem 2개: 권한 오류 추적, 번역 누락

먼저 완료 판단이 보류된 결제 오류부터 볼까요?
```

This is a conversational overview, not a fixed template. Preserve exact titles and state meanings,
but summarize supporting detail in the user's language.
