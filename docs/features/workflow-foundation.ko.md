# Workflow 기반 계약

[English](workflow-foundation.md) | **한국어**

LLM Wiki는 구조화된 AI 결과를 만든 버전별 prompt를 기록하고, 결과를 저장하거나 적용하기 전에 계약을 검증합니다. Prompt 계약은 기능별 정의 모듈을 가진 하나의 registry에 있으므로 Capture, Task, journey, 검색, Knowledge 기능은 안정적인 ID와 검증 규칙을 공유하면서 독립적으로 발전할 수 있습니다.

AI 실행과 적용은 별도 상태입니다. Queue Job 실행이 성공해도 제안이 검토를 기다리거나, 더 최신 원본 때문에 대체되었거나, 콘텐츠를 변경하지 않는 결과일 수 있습니다. Queue는 기존 상태와 결과에 더해 prompt ID와 버전, 정확한 원본 리비전, 실행 결과, 적용 결과를 제공합니다.

중요한 백그라운드 distillation과 색인 작업은 지속됩니다. 같은 활성 작업은 하나로 합치고, 중단된 작업은 재시작 후 복구하며, 일시적 실패는 제한된 횟수만 자동 재시도합니다. 최종 실패는 사용자가 명시적으로 다시 시도할 수 있습니다. 선택적 speculative search는 임시 작업입니다. 새 요청이 이전 요청을 취소하며 재시도 기록을 만들지 않습니다. 사용자가 요청한 생성 작업은 상태와 재시도 동작을 계속 제공합니다.

생성된 각 버전은 기존 Task, Refinement, Knowledge의 정식 리비전 테이블을 대체하지 않고 provenance와 정확한 문서 참조를 연결할 수 있습니다. 문서 참조에는 안정적인 문서 ID, 정확한 버전, 선택적 section, excerpt, claim ID가 들어갑니다. 이전 콘텐츠를 복원하면 복원 출처를 기록한 새 정식 버전을 만듭니다. 정확한 head 검사는 오래된 결과가 최신 작업을 덮어쓰지 못하게 합니다.

이 계약은 AI에 Task 상태, Problem 해결, 결정, Knowledge 게시 권한을 주지 않습니다. 각 기능은 하나의 transaction에서 도메인 변경과 원본 버전 검사를 수행한 뒤 결과가 적용되었는지 또는 대체되었는지를 기록합니다.

[기능 명세](../../specs/015-workflow-foundation/spec.md)와 [consumer API 계약](../../specs/015-workflow-foundation/contracts/workflow-foundation-api.md)을 참고하세요.

## Provider JSON Schema 계약

등록된 구조화 프롬프트 28개 모두 공통 요청 스키마와 응답 타입 검사 경로를 갖습니다. Knowledge, 보관 구성, Distillation 스키마는 파싱에 사용하는 같은 Rust 구조체에서 만듭니다. 스키마 검사는 기존 출처·정확한 인용·revision·승인 검증을 보완하며 권한을 부여하지 않습니다.

전송 스키마는 strict입니다. 객체는 닫힌 구조이며 선언한 모든 필드를 요구하고, 계약이 허용하는 곳에서 빈 값이나 명시적 nullable 값을 사용합니다. 호환되는 기존 응답의 누락 필드 복구는 유지하지만, 전달된 잘못된 타입은 거부합니다. 작업 프리뷰의 `scope`는 문자열이며 배열을 텍스트로 강제 변환하지 않습니다.

동적 journey 제목 키는 기록된 이벤트 ID로 제한합니다. Distillation target map은 알려진 semantic ID를 나열하며, 사용하지 않는 target의 null은 원래 sparse map으로 복원할 때 제거합니다. 구형 Task의 sparse patch도 선언된 null 자리 표시자를 생략한 뒤 기존 revision 검증을 거칩니다. 구형 완료 검토·발행 메타데이터·검색 제안·사용자 제안의 자유 형식 JSON 필드는 객체를 재귀적 `_entries` key/value 전송 형식으로 표현하고, 도메인 검증 전에 원래 JSON 타입으로 복원합니다. 항목의 key/value 누락과 중복 키는 거부합니다. 임의 키 객체도 내용을 버리지 않고 strict 스키마로 표현하기 위한 전송 형식입니다.

공통 도움말 생성, 이미지 포함 대기열 작업, 별도 Task Journey 추론, Problem 보강에 적용합니다. 자유 대화와 Codex 작업 실행은 구조화 결과 소비 경로가 아닙니다. Codex App Server는 turn별 `outputSchema`를 지원하지만, 현재 앱의 Codex turn은 자연어 작업 보고서를 요청하며 JSON으로 파싱하지 않습니다. 추가 생성이나 재시도는 넣지 않습니다.
