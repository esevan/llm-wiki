# 백그라운드 AI Queue

[English](background-ai-queue.md) | **한국어**

완료된 네이티브 작업에는 결과 열기 버튼이 표시됩니다. 이전 버전에서 인라인 미리보기로 저장된
기록도 포함됩니다. 충돌 검토·완료 검토·완료 보고서는 완료 시 결과 버튼이 활성화됩니다.
진행률이 가득 차고 **완료**로 표시되면 대기 중이 아니라 끝난 작업입니다. 결과를 열어 검토하세요.
충돌 검토 재클릭은 저장된 결과를 열며, 새 근거 분석은 **새 검토 실행**으로 요청합니다.

LLM Wiki는 상호작용의 반응성을 유지하면서 복구 가능한 작업을 잃지 않도록 AI 실행을 두 개의
프로세스 경로로 분리합니다.

![지속 작업의 목적, 대상, 상태, 결과 목적지를 읽을 수 있게 보여주는 백그라운드 Queue](images/08-background-queue.png)

- **Fast Queue**는 FIFO Worker 하나만 사용합니다. Chat과 즉시 반응이 필요한 상호작용을 전역
  요청 제한 경로로 처리하며, 데이터베이스 상태·Queue UI·재시도 기록·알림을 만들지 않습니다.
- **Asynchronous Queue**는 지속 가능한 AI·번역·임베딩 Job을 SQLite에 저장합니다. AI Setup에서
  Worker 수를 조절하며, Worker는 Lease와 Heartbeat로 작업을 점유합니다.

![충돌 검토가 백그라운드에서 이어짐을 알리는 지속 Job 시작 화면](images/09-background-job-queued.png)

오른쪽 아래 Queue는 각 백그라운드 작업의 대상과 목적을 설명하고, 이해하기 쉬운 상태·단계별
진행률·시스템 타임존 기준 시간·안전한 오류·취소·재시도를 표시합니다. 결과는 작업에 맞는 화면이나 간결한 요약으로
열리며 내부 Job JSON을 사용자 결과로 보여주지 않습니다. 결과가 있는 작업은 실행 중에도 목적지를
표시하고, 완료되면 눈에 띄는 **결과 페이지 열기** 버튼을 활성화합니다. 별도 결과 동선이 필요 없는
작업에는 결과 버튼을 표시하지 않습니다. Draft와 Refine 결과는 시작한 창에만 연결되고 창을 닫으면
취소됩니다.
Image Summary는 스크롤 위치를 유지한 채 정확한 Work Log 항목에 붙습니다. Completion Review는
사용자 결정이 필요하므로 잠시 표시되는 Toast와 읽지 않은 항목을 보존하는 종 알림을 함께 만듭니다.

Knowledge 번역은 문단 Checkpoint에서 재개하고 번역본을 Vault에 게시한 뒤 SQLite 작업 행을
삭제합니다. Capture와 Work Log 본문은 사용자가 저장할 때 활성화된 언어를 기준으로 즉시 파생 번역을
enqueue하며 사용자가 작성한 원문은
덮어쓰지 않습니다. Queue 카드는 내부 ID 대신 Capture 본문, Work Log 항목, 댓글, 체크리스트 항목을
구분하여 번역 대상을 설명합니다. 임베딩 갱신도 지속 가능한 작업이며 실행 중에도 어휘 검색을 사용할
수 있습니다.

Worker는 제한 재시도, 지수 Backoff, Source Hash, 유효 Lease Token, 취소, 중복 방지 알림을
사용합니다. 동시 Worker는 격리된 애플리케이션/SQLite 연결을 사용하며 SQLite Writer 경합은 최종
실패로 게시하지 않고 재시도합니다. 선택 사항인 Semantic Runtime이 없으면 임베딩 작업은 Semantic
Coverage 0과 어휘 검색 Fallback 결과로 완료됩니다. AI 결과는 제안 또는 파생 표현이며 Workflow
상태, 승인, 완료, Knowledge 결정 권한은
사용자에게 있습니다.

![Endpoint와 model routing은 표시하면서 자격 증명 값은 native secret storage에 유지하는 AI 설정](images/07-ai-settings.png)

[기능 명세](../../specs/009-background-ai-queue/spec.md)와
[Worker 계약](../../specs/009-background-ai-queue/contracts/worker-contract.md)을 참고하세요. 모든 지속
작업의 단일 Handler 모듈과 자동으로 검증하는 의존성 규칙은
[백엔드 아키텍처 안내](../architecture.ko.md)에 정리되어 있습니다.
