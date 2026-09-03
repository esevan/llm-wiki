# 백엔드 아키텍처

[English](architecture.md) | **한국어**

LLM Wiki 백엔드는 하나의 Web 조립 경계를 둔 Layered Architecture를 사용합니다. 의존성은 전달
계층에서 애플리케이션 로직과 저장 계층 방향으로만 흐르며, 하위 계층은 Controller나 Web 계층을
역참조하지 않습니다.

데스크톱 앱은 React 프레젠테이션과 네이티브 Rust 애플리케이션 계층을 사용합니다. React 기능
모듈은 `ApplicationClient`에 의존하며 데스크톱 어댑터가 기존 호환 경로를 이름이 있는 도메인
작업으로 변환합니다. Tauri는 Workflow, Vault, 설정, Job, System 명령과 취소 가능한 Provider
Streaming 명령을 각각 노출합니다. Rust가 SQLite와 설정된 Vault를 직접 엽니다. 내부 Port를
할당하거나 HTTP 형태 IPC를 전달하지 않으며 Python을 실행하거나 Python 실행 파일을 패키징하지
않습니다.

```text
React 기능 -> ApplicationClient -> HTTP 어댑터 -> FastAPI(별도 브라우저 제품)
                               -> Tauri 어댑터 -> Rust 도메인 명령 -> SQLite/Vault
                                                                  -> 번들 ONNX 임베딩
                                                                  -> 설정한 AI Provider
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
| 네이티브 데스크톱 애플리케이션 | 데스크톱 Workflow, Job, 설정, Vault 작업 실행 | `src-tauri/src/native/` |

네이티브 Vault 모듈은 앱 시작 시 Markdown을 인덱싱합니다. `native/semantic.rs`는 Tauri 리소스의
체크섬 고정 다국어 MiniLM 모델을 필요할 때 로드하고 384차원 벡터를 SQLite에 저장한 뒤 사용자가
선택한 검색 결과를 로컬에서 재정렬합니다. Release 앱은 Runtime에 모델을 내려받지 않으며 재현 가능한
데스크톱 빌드 준비 단계에서만 검증된 자산을 내려받습니다.

공개 조립 지점은 `web.app.create_app`입니다. 여기에서 `ApplicationRuntime` 하나를 만든 뒤
`controllers.application.create_http_app`에 전달합니다. Controller는 Repository나 제공자 Adapter를
직접 만들지 않습니다. Queue 제출은 `services/job_submission.py`, SQLite 생명주기와 상태 전이 규칙은
`repositories/jobs.py`가 담당합니다.

이 Python 조립 지점은 브라우저 전달 모드에만 남습니다. Tauri는 이를 Import하거나 실행하거나
접속하지 않습니다. 데스크톱 조립 지점은 `src-tauri/src/lib.rs`이며 얇은 Tauri Handler가 도메인
작업을 허용 목록으로 제한한 뒤 `src-tauri/src/native/`에 위임합니다. 데스크톱에서 HTTP는 사용자가
명시적으로 설정한 외부 AI Provider 호출에만 사용됩니다.

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
