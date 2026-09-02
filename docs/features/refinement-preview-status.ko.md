# 맥락을 보존하는 Refinement Preview

[English](refinement-preview-status.md) | **한국어**

> **Resume where you left off.** Refinement가 일을 의미 있게 만든 맥락을 지워서는 안 됩니다.

![Problem Refinement에서 계보·이전 결정·근거·제약을 대화 옆에 유지하는 화면](images/05-refinement-preview.png)

Capture, Problem, Solution 카드를 선택하면 별도의 Explore 액션 없이 대화와 Preview가
한 작업 공간에 열립니다. AI와 이야기하는 동안 Preview는 현재 상세, 최근 대화, 이전 초안,
Capture → Problem → Solution 계보를 계속 보여줍니다. AI는 제안을 정리하지만 실제 반영은
사용자만 할 수 있습니다.

## Refinement 흐름

1. Capture, Problem 또는 Solution 카드를 선택합니다.
2. **Context**에서 현재 항목과 계보를 복구합니다.
3. 가장 중요한 빈틈에 집중한 AI의 개방형 질문 하나에 답합니다.
4. 최신 구조화 초안을 생성하는 동안 작업 공간에 생성 상태가 표시됩니다. Preview에는 작업 상태가,
   채팅에는 애니메이션 `...`가 표시됩니다.
5. 초안이 Preview에 표시된 뒤에 **Context**와 **Detail**을 비교합니다. 이때만 채팅에
   `✅ Ready. Your AI refinement is ready to review.`가 표시됩니다.
6. 내용이 충분히 정확할 때만 **Apply Refinement**를 선택합니다.

Problem Detail은 context, impact, evidence, desired outcome, boundaries, open questions를 다룹니다.
Solution Detail은 intended outcome, scope, non-goals, prior evidence, trade-offs, dependencies,
validation criteria, risks, open questions를 다룹니다. 모르는 값은 모르는 상태로 명시됩니다.

## 이어지는 Context

Preview는 추가 요약 요청 없이 로컬 저장 기록으로 결정적으로 구성됩니다. 현재 제목과 상세, 최근
Explore 대화와 초안, Problem의 원본 Capture, Solution의 상위 Problem을 포함합니다. 오래된 이력이
현재 작업을 압도하지 않도록 최근 맥락을 제한된 범위로 보여줍니다. Explore를 다시 열면 최신 초안이
복구되고 이미 적용한 내용은 **APPLIED**로 표시됩니다.

## 백그라운드 상태와 실패 처리

| 상태 | 의미 |
| --- | --- |
| `LIVE CONTEXT` | Context가 준비되어 대화를 시작할 수 있습니다. |
| `REFINING…` | 새 초안을 준비 중이며 대화는 계속 사용할 수 있습니다. |
| `DRAFT READY` | Detail을 비교하고 검토할 수 있습니다. |
| `APPLIED` | 표시된 초안이 이미 반영되었습니다. |
| `NEEDS ATTENTION` | 생성에 실패했으며 기존 Context와 초안은 유지됩니다. |

Context를 불러오지 못하면 빈 Preview는 닫히고 사용 가능한 대화창에 접근 가능한 경고가 남습니다.
다른 항목으로 이동하면 이전 요청을 폐기해 늦은 응답이 새 Preview를 덮어쓰지 못하게 합니다.

Ready 메시지는 초안 생성을 시작한다는 뜻이 아니라, 생성된 초안이 Preview에 반영되어 검토할 수
있다는 완료 신호입니다.

## 다음 Solution 탐색

승인된 Problem은 Task 단계나 별도 검토 모달 없이 같은 작업 공간에서 다음 Solution을 탐색합니다.

![보존된 Problem Context 옆에서 검토 가능한 다음 Solution Detail을 준비하는 화면](images/06-next-solution-preview.png)

AI는 응답이 끝날 때마다 title, intended outcome, non-goals, validation criteria가 포함된 제안을
갱신합니다. **Create Solution**은 실제 생성에 필요한 명시적인 사용자 동작입니다. 다시 열면 마지막
제안이 복구되고 이미 사용한 제안은 **CREATED**로 표시됩니다. 생성에 성공하면 Explore 모달이
자동으로 닫히며, 생성 처리 중이거나 실패한 경우에는 모달이 열린 채로 유지됩니다.

Capture는 Problem이 되기 전에 Preview가 포함된 **Explore Problem** 작업 공간을 엽니다. 대화와
Refinement로 제안을 다듬어도 Capture 상태는 바뀌지 않으며, **Create Problem**을 사용자가
명시적으로 선택해야 Problem으로 승격되고 대화가 계보로 보존됩니다.
승격 성공이 확인된 뒤에만 Explore 모달이 자동으로 닫히므로, 검증 또는 생성 오류가 발생하면
모달에서 오류를 확인하고 다시 시도할 수 있습니다.

관련 Spec Kit: [005 — Refinement Preview Status](../../specs/005-refinement-preview-status/spec.md)
