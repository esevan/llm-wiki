# 백그라운드 AI Queue

[English](background-ai-queue.md) | **한국어**

LLM Wiki는 상호작용의 반응성을 유지하면서 복구 가능한 작업을 잃지 않도록 AI 실행을 두 개의
프로세스 경로로 분리합니다.

- **Fast Queue**는 FIFO Worker 하나만 사용합니다. Chat과 즉시 반응이 필요한 상호작용을 전역
  요청 제한 경로로 처리하며, 데이터베이스 상태·Queue UI·재시도 기록·알림을 만들지 않습니다.
- **Asynchronous Queue**는 지속 가능한 AI·번역·임베딩 Job을 SQLite에 저장합니다. AI Setup에서
  Worker 수를 조절하며, Worker는 Lease와 Heartbeat로 작업을 점유합니다.

오른쪽 아래 Queue에서 백그라운드 작업의 진행률, 안전한 오류, 취소, 재시도와 작업별 결과 동선을
확인할 수 있습니다. Draft와 Refine 결과는 시작한 창에만 연결되고 창을 닫으면 취소됩니다. Image
Summary는 스크롤 위치를 유지한 채 정확한 Work Log 항목에 붙습니다. Completion Review는 사용자
결정이 필요하므로 잠시 표시되는 Toast와 읽지 않은 항목을 보존하는 종 알림을 함께 만듭니다.

Knowledge 번역은 문단 Checkpoint에서 재개하고 번역본을 Vault에 게시한 뒤 SQLite 작업 행을
삭제합니다. Capture와 Work Log 본문은 저장 즉시 파생 번역을 enqueue하며 사용자가 작성한 원문은
덮어쓰지 않습니다. 임베딩 갱신도 지속 가능한 작업이며 실행 중에도 어휘 검색을 사용할 수 있습니다.

Worker는 제한 재시도, 지수 Backoff, Source Hash, 유효 Lease Token, 취소, 중복 방지 알림을
사용합니다. AI 결과는 제안 또는 파생 표현이며 Workflow 상태, 승인, 완료, Knowledge 결정 권한은
사용자에게 있습니다.

[기능 명세](../../specs/009-background-ai-queue/spec.md)와
[Worker 계약](../../specs/009-background-ai-queue/contracts/worker-contract.md)을 참고하세요.
