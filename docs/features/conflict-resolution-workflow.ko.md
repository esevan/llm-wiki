# Task 충돌 검토

[English](conflict-resolution-workflow.md) | **한국어**

충돌 검토는 Task에 연결된 조언용·리비전 기준 근거입니다. Task 상세에서 검토를 실행하면 패널에서 최신 결과, 출처, 상태, 이전 시도를 확인합니다. 검토가 대기·실행·실패·취소되었거나 발견 사항을 보고해도 Task는 계속 진행할 수 있습니다. 검토가 Task를 승인·차단·완료·발행하지는 않습니다.

![근거 부족·재시도·시도 이력을 보여 주는 Task 충돌 검토](images/task-review.png)

## 검토 읽기와 재시도

결과에는 검토한 Task 리비전과 Vault 맥락이 표시됩니다. 관련 Task 또는 Vault 근거가 바뀌면 이전 결과는 오래된 상태가 되므로, 이를 근거로 삼기 전 새 검토를 실행하세요. 실패하거나 취소한 시도도 상태와 함께 남아 재시도할 수 있습니다. 출처는 사용자의 판단을 돕는 근거이며 AI가 결과를 결정하거나 출처를 만들어 내지 않습니다.

현재 Task 상세에는 충돌별 수용·적용 제어나 clear 충돌 Gate가 없습니다. 이러한 제어는 보존한 레거시 Queue/보고서 기록에만 해당하며 현재 Task 워크플로를 정의하지 않습니다.

[Task 중심 Workbench](conflict-gated-workflow.ko.md)와 역사 기록인 [충돌 해결 명세](../../specs/010-conflict-resolution-workflow/spec.md)를 참고하세요.
