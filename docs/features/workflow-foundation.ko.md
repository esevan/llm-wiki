# Workflow 기반 계약

[English](workflow-foundation.md) | **한국어**

LLM Wiki는 구조화된 AI 결과를 만든 버전별 prompt를 기록하고, 결과를 저장하거나 적용하기 전에 계약을 검증합니다. Prompt 계약은 기능별 정의 모듈을 가진 하나의 registry에 있으므로 Capture, Task, journey, 검색, Knowledge 기능은 안정적인 ID와 검증 규칙을 공유하면서 독립적으로 발전할 수 있습니다.

AI 실행과 적용은 별도 상태입니다. Queue Job 실행이 성공해도 제안이 검토를 기다리거나, 더 최신 원본 때문에 대체되었거나, 콘텐츠를 변경하지 않는 결과일 수 있습니다. Queue는 기존 상태와 결과에 더해 prompt ID와 버전, 정확한 원본 리비전, 실행 결과, 적용 결과를 제공합니다.

중요한 백그라운드 distillation과 색인 작업은 지속됩니다. 같은 활성 작업은 하나로 합치고, 중단된 작업은 재시작 후 복구하며, 일시적 실패는 제한된 횟수만 자동 재시도합니다. 최종 실패는 사용자가 명시적으로 다시 시도할 수 있습니다. 선택적 speculative search는 임시 작업입니다. 새 요청이 이전 요청을 취소하며 재시도 기록을 만들지 않습니다. 사용자가 요청한 생성 작업은 상태와 재시도 동작을 계속 제공합니다.

생성된 각 버전은 기존 Task, Refinement, Knowledge의 정식 리비전 테이블을 대체하지 않고 provenance와 정확한 문서 참조를 연결할 수 있습니다. 문서 참조에는 안정적인 문서 ID, 정확한 버전, 선택적 section, excerpt, claim ID가 들어갑니다. 이전 콘텐츠를 복원하면 복원 출처를 기록한 새 정식 버전을 만듭니다. 정확한 head 검사는 오래된 결과가 최신 작업을 덮어쓰지 못하게 합니다.

이 계약은 AI에 Task 상태, Problem 해결, 결정, Knowledge 게시 권한을 주지 않습니다. 각 기능은 하나의 transaction에서 도메인 변경과 원본 버전 검사를 수행한 뒤 결과가 적용되었는지 또는 대체되었는지를 기록합니다.

[기능 명세](../../specs/015-workflow-foundation/spec.md)와 [consumer API 계약](../../specs/015-workflow-foundation/contracts/workflow-foundation-api.md)을 참고하세요.
