# 백엔드 아키텍처

[English](architecture.md) | **한국어**

LLM Wiki 백엔드는 하나의 Web 조립 경계를 둔 Layered Architecture를 사용합니다. 의존성은 전달
계층에서 애플리케이션 로직과 저장 계층 방향으로만 흐르며, 하위 계층은 Controller나 Web 계층을
역참조하지 않습니다.

데스크톱 마이그레이션은 도메인 로직을 Rust로 옮기지 않고 React 프레젠테이션 계층과
Tauri 전달 어댑터를 추가합니다. React 기능 모듈은 `ApplicationClient`에 의존하며 HTTP와
Tauri 어댑터가 이 경계를 구현합니다. Tauri는 범위가 좁고 검증된 요청/Streaming/취소 명령
모음만 노출하고, 알려진 애플리케이션 경로군과 안전한 Header만 Loopback Python 사이드카로
전달합니다. 요청 ID가 증분 Chunk와 취소를 연결합니다. Shell은 사이드카를 하나의 전용 Process
Group으로 패키징하고 시작, 상태 확인, 종료합니다. Fast Worker와 지속 Worker는 이 관리 대상
Process 안에서 실행됩니다. 각 동시 지속 Worker는 격리된 애플리케이션 Runtime과 SQLite 연결을
소유하며 일시적인 Writer 경합은 Queue의 제한된 재시도 정책을 따릅니다.
Python Runtime이 유일한 애플리케이션/도메인 구현으로 남으므로 HTTP와 데스크톱 전달 계층에
워크플로 로직이 중복되지 않습니다.

```text
React 기능 -> ApplicationClient -> HTTP 어댑터(웹)
                               -> Tauri 어댑터 -> Rust 요청/Streaming/취소 -> 관리되는 Python 런타임
```

프런트엔드 디자인 토큰, 전역/컴포넌트 스타일, 로컬 번들 폰트는 `frontend/src/theme/`에
있습니다. 주요 화면은 `frontend/src/features/`의 도메인 모듈입니다. React JSX가 탐색,
대화상자와 알림 영역을 소유하며 HTML 진입점에는 Inline Bootstrap Code가 없습니다. 남아 있는
명령형 동작은 `llm_wiki/static/runtime/` 아래의 도메인별 Controller 11개로 분리되어 있습니다.
이 Controller에는 삽입 Style이나 원시 시각 상수가 없으며 Typed React Hook으로 단계적으로
교체하는 작업은 마이그레이션 감사 문서에 기록되어 있습니다.

| 계층 | 책임 | 주요 위치 |
| --- | --- | --- |
| Web | 애플리케이션 Runtime 조립 및 공개 App Factory 제공 | `llm_wiki/web/` |
| Controller | HTTP 입력 검증, 응답 변환, Route와 Use Case 연결 | `llm_wiki/controllers/` |
| Service | Workflow Use Case 실행, Job 제출, 작업 Handler 실행 | `llm_wiki/services/` |
| Repository | SQLite의 지속 Job 상태와 Checkpoint 저장 | `llm_wiki/repositories/` |
| Core | 외부 의존성이 없는 Queue 도메인 값·상태·오류 정의 | `llm_wiki/core/` |
| Adapter | AI 제공자와 외부 메커니즘 연동 | `llm_wiki/adapters/` |

공개 조립 지점은 `web.app.create_app`입니다. 여기에서 `ApplicationRuntime` 하나를 만든 뒤
`controllers.application.create_http_app`에 전달합니다. Controller는 Repository나 제공자 Adapter를
직접 만들지 않습니다. Queue 제출은 `services/job_submission.py`, SQLite 생명주기와 상태 전이 규칙은
`repositories/jobs.py`가 담당합니다.

## AI 작업 모듈 지도

각 지속 작업은 하나의 `TaskDescriptor`와 검색하기 쉬운 단일 작업 모듈을 가집니다.

| 작업 | Handler 모듈 |
| --- | --- |
| Draft | `services/handlers/drafting.py` |
| Refine | `services/handlers/refinement.py` |
| Image Summary | `services/handlers/image_summary.py` |
| Completion Review | `services/handlers/completion_review.py` |
| Knowledge 번역 | `services/handlers/knowledge_translation.py` |
| Capture, Work Log, 댓글, 체크리스트 번역 | `services/handlers/derived_translation.py` |
| 임베딩 갱신 | `services/handlers/embeddings.py` |
| Conflict Review | `services/handlers/conflict_review.py` |
| Workbench 정리 | `services/handlers/organization.py` |
| Lineage 추론 | `services/handlers/lineage.py` |
| Completion Report | `services/handlers/completion_report.py` |

공통 제공자 설정, 대상 검증, Handler 등록, Worker 실행은 각각 `provider.py`, `targets.py`,
`catalog.py`, `registry.py`, `worker.py`에 있습니다. 개별 작업 동작은 이 공통 모듈에 넣지 않습니다.

Conflict Review Handler는 Provider 출력을 정규화하고 `WorkflowEngine`을 통해 검토 단위 충돌을
저장합니다. 브라우저는 사용자가 결정한 전체 해결 방식 모음을 Application Controller로 보내며,
Workflow Service는 이를 검증한 뒤 해결 기록과 기존 충돌 Report/Address Gate를 하나의 SQLite
Transaction으로 저장합니다. Source Query 비교로 오래된 검토를 거부하며 Provider 출력은 사용자의
결정 동작을 포함하거나 저장하지 않습니다.

## 자동으로 강제하는 설계 규칙

아키텍처 테스트는 역방향 계층 Import, 내부 순환 의존성, 중복되거나 잘못 배치된 작업 Descriptor,
삭제한 구형 모듈의 재도입, Queue 관련 백엔드 함수의 과도한 분기를 거부합니다. API 계약 테스트는
공개 Web Factory를 기준으로 실행하므로 내부 구조를 바꿔도 외부 동작을 보존합니다. 관련 검증은
`tests/test_architecture.py`와 `tests/test_ai_task_inventory.py`에 있습니다.

사용자에게 보이는 Queue 동작은 [백그라운드 AI Queue](features/background-ai-queue.ko.md), 검토 결정
흐름은 [충돌 해결 워크플로](features/conflict-resolution-workflow.ko.md)를 참고하세요.
