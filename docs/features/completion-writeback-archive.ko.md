# 완료와 Knowledge

[English](completion-writeback-archive.md) | **한국어**

Task에는 Work Log·첨부·댓글·체크리스트·결정을 함께 기록합니다. 작업이 끝나면 근거와 함께
명시적으로 완료합니다. 완료는 해당 Task의 상태를 바꿀 뿐, 연결된 Problem이나 다른 Task를
해결·완료하거나 Vault 문서를 발행하지 않습니다. Capture와 연결된 정확한 Problem 리비전은
출처 정보로 유지됩니다.

## 검토와 발행을 별도로 결정하기

완료한 작업에서 비공개 Knowledge 초안을 만들고, 읽기 쉬운 Markdown 미리보기를 검토·수정한 뒤 발행합니다.
생성은 즉시 지속 가능한 **AI Queue**에 들어가며, Queue에서 대기·실행·실패·재시도·완료 상태를 확인할 수 있습니다.
완료된 Queue 결과를 열면 생성한 정확한 초안 리비전이 열립니다. 이후 초안을 수정·게시하거나 새 리비전을 만들면 원래 결과는 읽기 전용으로 유지됩니다.
이미 저장된 비공개 초안도 다시 열면 Markdown 미리보기와 편집 가능한 원본으로 표시됩니다.
생성된 Markdown에는 Task에 기록한 목표·맥락·범위·제외 범위·검증·작업 근거·체크리스트·결정·
완료 근거·정확한 출처가 포함됩니다. 기록하지 않은 항목은 임의의 내용으로 채우지 않고 미기록으로
표시합니다. Queue Job은 요청한 Task 리비전을 기록하며, 실행 중 원본 스냅샷이 바뀌면 검토하지 않은 새 작업으로 초안을 만들지 않고 거부합니다.

발행은 검토한 초안에 대해 별도로 선택하는 명시적인 동작입니다. Vault에 Markdown을 쓰며,
문서는 LLM Wiki 밖에서도 사용할 수 있습니다. 초안 리비전과 원본 해시로 발행할 내용을 식별하고,
검토하지 않은 리비전을 조용히 대신 발행하지 않습니다.

## 수정·재생성·철회

초안은 사용자가 의도적으로 수정합니다. 재생성은 현재 기록된 작업과 출처를 바탕으로 초안을
갱신합니다. 발행된 파일이 외부에서 수정됐다면 정확한 파일·해시 검사를 통해 그 수정을 조용히
덮어쓰지 않도록 합니다.

철회도 별도 결정이며 복구 가능한 로컬 사본을 보존합니다. 완료한 Task나 비공개 작업 근거를
지우지 않습니다. 발행·재생성·철회는 각각 결과와 오류를 가지며, Task 완료나 재개만으로 자동
실행되지 않습니다.

출처 구조는 [Lineage Knowledge Layer](lineage-knowledge-layer.ko.md), 실제 검증 근거는
[인터랙티브 기능 검증 기록](../testing/interactive-coverage.md)을 참고하세요.

관련 Spec Kit: [012 — Task 중심 Workbench](../../specs/012-task-centered-workbench/spec.md).
이전 계약: [003 — Completion, Writeback, and Archive](../../specs/003-completion-writeback-archive/spec.md).
