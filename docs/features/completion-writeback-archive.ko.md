# 완료와 Knowledge

[English](completion-writeback-archive.md) | **한국어**

독립 Task에는 Work Log·첨부·댓글·체크리스트·결정을 함께 기록합니다. 작업이 끝나면 근거와 함께
명시적으로 완료합니다. 완료는 해당 Task의 상태를 바꿀 뿐, 연결된 Problem이나 다른 Task를
해결·완료하거나 Vault 문서를 발행하지 않습니다. Capture와 연결된 정확한 Problem 리비전은
출처 정보로 유지됩니다.


### Subtask의 부모 Knowledge 자동 발행

명시적인 Subtask를 적용하면 자식을 근거와 함께 완료할 때마다 부모 Knowledge를 자동 갱신·발행합니다. 같은 부모 문서의 각 리비전에 완료한 자식의 근거를 누적합니다. 모든 자식이 완료되어야 부모도 완료되고, 자식을 다시 열면 완료된 조상도 다시 엽니다. 발행 대기 기록으로 재시도하며 외부 파일 수정은 보존합니다. 독립 Task는 기존의 별도 검토·발행 흐름을 유지합니다. [Subtask 수명 주기](refinement-preview-status.ko.md)를 참고하세요.

## 명시적인 발행 한 번

비공개 Knowledge 초안을 생성하거나 편집한 뒤 **Knowledge 발행**을 한 번 누릅니다.
품질·인용·출처 최신성 확인 사항은 차단 없는 메타데이터로 유지합니다. 사용자 경험은
출처이며 외부 인용은 선택 사항입니다. 이전·실패 초안도 재생성 없이 발행할 수 있습니다.
저장·해시·버전 충돌 보호는 유지합니다. [발행과 복구](knowledge-archive.ko.md)를 참고하세요.

## 수정·재생성·철회

초안은 사용자가 의도적으로 수정합니다. 재생성은 현재 기록된 작업과 출처를 바탕으로 초안을
갱신합니다. 발행된 파일이 외부에서 수정됐다면 정확한 파일·해시 검사를 통해 그 수정을 조용히
덮어쓰지 않도록 합니다.

철회도 별도 결정이며 복구 가능한 로컬 사본을 보존합니다. 완료한 Task나 비공개 작업 근거를
지우지 않습니다. 발행·재생성·철회는 각각 결과와 오류를 가지며, 독립 Task 완료나 재개만으로 자동
실행되지 않습니다.

출처 구조는 [Lineage Knowledge Layer](lineage-knowledge-layer.ko.md), 실제 검증 근거는
[인터랙티브 기능 검증 기록](../testing/interactive-coverage.md)을 참고하세요.

관련 Spec Kit: [012 — Task 중심 Workbench](../../specs/012-task-centered-workbench/spec.md).
이전 계약: [003 — Completion, Writeback, and Archive](../../specs/003-completion-writeback-archive/spec.md).
