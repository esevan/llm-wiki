# Problem 중심 Workbench

[English](conflict-gated-workflow.md) | **한국어**

> **You talk. The work organizes itself. Organize around problems, not tasks.**

![가벼운 Capture, Problem, Solution과 강조된 In Progress 영역을 보여주는 Workbench](images/02-workbench.png)

## 흐름

1. 분류하지 않고 생각을 **Capture**합니다.
2. AI 대화와 Refinement로 **Problem**을 이해하고 검토합니다.
3. Problem을 승인하고 **Solution**의 결과·경계·검증 기준을 검토합니다.
4. Knowledge 기반 충돌 근거를 확인합니다. 인용된 `clear` 결과만 Solution을 시작할 수 있습니다.
5. Solution Work Log와 완료 흐름 안에서 계속 작업합니다.

Task 단계는 없습니다. Explore는 상태를 바꾸지 않고 AI 초안은 편집 가능하며, 모든 전이는 사람이
결정합니다. Soft delete는 개인 기록이나 vault 파일을 삭제하지 않습니다.

Problem 승인은 카드에서 바로 반응합니다. 처리 중 상태를 표시하고 성공하면 Workbench를 새로
반영하며, 승인을 저장할 수 없으면 사용자가 확인할 수 있는 오류 메시지를 남깁니다.
같은 위임 상호작용 경로로 다음 Solution 탐색을 열고 Conflict Review와 Completion Review를
enqueue하며, Solution을 제안 및 진행 중 상태 사이에서 이동합니다.

관련 Spec Kit: [002 — Conflict-Gated Workflow](../../specs/002-conflict-gated-workflow/spec.md)
