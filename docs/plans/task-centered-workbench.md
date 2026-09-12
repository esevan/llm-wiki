# Task 중심 Workbench 전환 계획

상태: 구현 전 계획
작성일: 2026-09-05
범위: 데이터 모델, 네이티브 API와 MCP, Workbench/Refinement/Conflict/Completion/Knowledge UX, 마이그레이션, 자동화 검증

이 문서는 현재 `Capture → Problem → Solution` 흐름을 `Capture → Task → in_progress → completed` 흐름으로 전환하는 구현 계획이다. Capture는 아직 실행 단위로 정하지 않은 생각을 보존하는 1급 canonical 항목이고, Task는 실행과 재개의 중심이 되는 독립 레코드다. 명확한 Task 입력의 초기 상태 값은 `task`, 시작하면 `in_progress`, 끝내면 `completed`다. Refinement, Problem 연결, Conflict Review, Readiness는 Capture 또는 Task를 더 잘 이해하기 위한 보조 기능이며 Task 생성이나 시작을 막는 승인 게이트가 아니다.

이 변경은 LLM Wiki Problem `7e1bd5c7-7801-42d3-a143-11d0ee18ef71`(Capture에 이미 도출된 Solution을 다시 탐색하는 중복)과 `dc1c8b22-367e-4662-a9db-305e6fac1f85`(단일 사용자 환경의 기계적인 Problem 승인), 기존 Solution `39575c74-9a31-4325-97de-d8e3da61fcb2`를 근거로 한다. 통합 추적 레코드를 새로 열려는 호출은 현재 호스트가 처리할 수 없는 `elicitation_required`를 반환했으므로 새 레코드는 저장되지 않았다. 구현에서는 기존 ID를 provenance로 유지하며, Solution이 이미 적힌 Capture를 refinement할 때 해당 내용을 Task 초안의 시작점으로 보존하고 실제로 빠진 실행 정보만 묻는다.

이 계획은 의도적으로 기존 모델과 API의 호환 shim을 만들지 않는다. 다만 기존 사용자 기록, 특히 Solution Work Log의 텍스트·이미지·댓글·체크리스트와 결정·완료·Lineage 근거는 일회성 마이그레이션과 사전 백업으로 보존한다. 구현 작업은 별도 worktree에서 수행하며, 이 계획 문서 자체는 코드 변경이 아니므로 현재 checkout에 작성한다.

## 결정된 제품 원칙

1. 입력에는 `생각 남기기`와 `Task로 등록`이라는 명시적이고 가벼운 선택이 있다. 기본은 Capture인 `생각 남기기`이며, AI가 모호함을 분류하도록 기다리지 않는다. 사용자가 Task를 선택하거나 명확한 Task 명령을 내리면 즉시 Task가 된다.
2. Capture와 Task 모두 선택적 Refinement를 열 수 있고 자동 저장된다. 사용자는 채팅, 최신 draft, 선택한 탭과 스크롤/커서 위치를 이어서 열 수 있다. Capture 하나의 Refinement가 여러 Problem과 여러 Task를 만들 수 있다.
3. Problem은 Task의 부모 단계가 아닌 독립적이고 revision을 갖는 레코드다. Task와 Problem은 다대다로 연결하며, Problem이 없어도 Task는 유효하다.
4. Work Log는 Task가 소유한다. 첨부, 댓글, 체크리스트, 결정 기록을 포함하며 Task 상태가 `task`여도 작성·열람할 수 있다.
5. Task 관계는 `prerequisite`, `split_from`/`split_to`, `related`를 지원한다. 모든 관계에서 연결된 Task로 이동할 수 있다. prerequisite가 남아 있어도 시작할 수 있지만, 영향을 명시한다.
6. Readiness는 AI 점수나 시작 허가가 아니다. 현재 Task에 적용되는 구체 필드를 `resolved`, `missing`, `not_applicable`로 보여 주고 각 상태의 근거 또는 사용자 결정을 제시한다.
7. Conflict Review는 Refinement와 동시에 실행 가능한 비차단 보조 검사다. 실패·근거 부족·stale 상태를 `clear`로 바꾸지 않으며 Task 시작이나 Work Log를 막지 않는다.
8. Task 완료는 해당 Task만 닫는다. 연결된 Problem의 해결 여부는 별도의 사용자 결정과 근거로 기록한다.
9. Knowledge 발행은 완료와 분리된 명시적 결정이다. 발행 문서는 Task, 연결 Problem revision, Work Log, 완료 근거, Conflict 결과의 lineage를 보존한다.
10. Workbench의 Capture와 Task는 현재 category별 canonical 목록에 함께 남고 사용자 override 동작도 유지한다. category 체계나 정렬 의미를 재설계하지 않는다.

## 현재 구현에서 교체할 경계

현재 SQLite 기준선은 [`src-tauri/src/native/schema.sql`](../../src-tauri/src/native/schema.sql)에 `captures`, `problems`, `features`를 두고 `features.problem_id NOT NULL`로 Solution을 Problem 아래에 강제한다. Work Log는 `solution_progress_entries`, `solution_progress_comments`, `solution_checklist_items`가 `feature_id`로 소유하며, 완료·Conflict·Lineage도 feature/problem ID에 연결된다. [`src-tauri/src/native/workflow.rs`](../../src-tauri/src/native/workflow.rs)는 Problem이 `approved`여야 Solution을 만들고, Solution을 시작하기 전에 `conflict_state='clear'`를 요구한다. 이 두 조건을 제거해야 한다.

Workbench의 React shell은 [`frontend/src/features/workbench/WorkbenchView.tsx`](../../frontend/src/features/workbench/WorkbenchView.tsx)에 있지만, 실제 board 렌더링과 상호작용의 큰 부분은 `frontend/public/runtime/`의 legacy runtime에 있다. [`frontend/public/runtime/explore.js`](../../frontend/public/runtime/explore.js)는 Capture에서 Problem, Problem에서 Solution을 생성하는 `next` 모드와 readiness 표현을 갖고 있고, [`frontend/public/runtime/conflicts.js`](../../frontend/public/runtime/conflicts.js)는 승인·Conflict·상태 이동을 board 클릭에 연결한다. 구현 중에는 이 기능을 React feature 모듈로 옮겨 동일한 도메인 행동이 두 UI 런타임에 남지 않게 한다.

추적 대화는 [`src-tauri/src/native/work_tracking_schema.sql`](../../src-tauri/src/native/work_tracking_schema.sql), [`src-tauri/src/application/work_tracking_service.rs`](../../src-tauri/src/application/work_tracking_service.rs), [`src-tauri/src/domain/work_tracking_state.rs`](../../src-tauri/src/domain/work_tracking_state.rs)에 별도 event stream과 projection을 둔다. 현재 event kind와 카드 stage가 Problem/Solution approval 흐름을 표현하므로 Task 중심 event와 projection으로 함께 교체한다. Knowledge 발행의 별도 승인과 exact revision/hash 규칙은 유지한다.

SQLite 마이그레이션은 [`src-tauri/src/native/migrations.rs`](../../src-tauri/src/native/migrations.rs)의 연속된 `PRAGMA user_version`과 migration별 `IMMEDIATE` transaction을 사용한다. 앱 시작은 [`src-tauri/src/native/database.rs`](../../src-tauri/src/native/database.rs)에서 이를 적용한다. 새 스키마는 기존 기준선이나 이미 배포된 migration을 고치지 않고 다음 version으로 추가한다.

## 목표 정보 구조와 UX

### Workbench

한 화면의 주 작업은 “생각을 남기거나 현재 Task를 재개하는 것”이다. 기존 색·타이포·간격 토큰을 재사용하고, 넓은 화면은 상단 집중 영역과 하단 전체 목록, 좁은 화면은 같은 순서의 단일 열로 구성한다.

1. 상단 입력은 한 줄 또는 자연스럽게 늘어나는 짧은 입력과 `생각 남기기`/`Task로 등록` 선택을 보여 준다. 기본값 `생각 남기기`는 Capture만 만들고, `Task로 등록`은 원 Capture provenance와 Task를 한 transaction에 만든다. 명시적 Task 명령도 같은 direct Task 경로를 쓴다. 저장 실패 시 입력과 선택을 유지한다.
2. 첫 번째 집중 영역은 최근 사용자 활동 기준 `in_progress` Task를 최대 N개 보여 준다. 기본값은 3개로 두고 설정 상수로 관리한다. “계속하기”, 체크리스트 진행, 최신 Work Log 시각 근거, unresolved prerequisite를 빠르게 확인한다.
3. 두 번째 집중 영역은 최근 사용자 활동 기준 refinement 진행 중 Capture 또는 Task를 최대 N개 보여 준다. 최신 대화 한 줄, draft 저장 상태, 다음에 다룰 missing field와 “Refinement 계속”을 제공한다.
4. 위 두 영역은 canonical 목록의 shortcut이다. 같은 Task를 하단 목록에서 제거하지 않으며, shortcut 영역끼리는 중복하지 않는다. 첫 영역에 나타난 Task는 두 번째 영역에서 제외한다.
5. 하단 canonical 목록은 Capture와 Task를 현재 category별 그룹에 함께 두고 수동 category override를 그대로 유지한다. Task 카드에는 상태, 제목, 최소 메타데이터, readiness 요약, 관계 경고를 표시하고 Capture 카드는 원문과 `Refinement`/`Task 만들기`를 제공한다. category 알고리즘과 category 편집 경험은 이번 범위에서 바꾸지 않는다.
6. `last_user_activity_at`만 shortcut과 recent 정렬에 사용한다. 사용자의 입력, 편집, 체크, 상태 변경, 관계/결정 기록, 명시적 리뷰 요청은 갱신한다. 백그라운드 AI job의 진행·완료, projection, 자동 번역·색인은 갱신하지 않는다.

Shortcut은 별도 복제 레코드가 아니라 typed canonical item identity(`capture_id` 또는 `task_id`)를 가리키는 projection이다. 접근성 이름에는 항목 제목과 동작을 포함하고, focus-visible, loading, empty, error, 좁은 창, 긴 한국어/영어 제목을 설계·검증한다. 상태는 색만으로 전달하지 않는다.

### Task 상세

Task 상세의 한 시각적 초점은 상태와 다음 행동이다. 헤더에는 제목, `task/in_progress/completed`, 상태 변경, category를 두고 본문은 다음 순서로 배치한다.

- `Overview`: 설명, 기대 결과, 범위/제외 범위, 검증 기준. 빈 필드는 숨기지 않고 readiness와 연결된 `아직 없음` 상태를 보여 준다.
- `Work Log`: 텍스트, 이미지/파일 첨부, 댓글, 체크리스트, 결정 기록. Task 생성 직후부터 사용 가능하다.
- `Relationships`: prerequisite, split, related 관계와 양방향 navigation. 삭제 대신 관계 해제라는 정확한 용어를 사용한다.
- `Problems`: 연결 Problem과 연결에 사용한 Problem revision, 관계 메모, 해결 기여 상태를 보여 준다.
- `Refinement`: autosave된 대화와 draft를 이어서 열고 적용할 제안 단위를 고른다.
- `Reviews`: Conflict 결과, stale/failed/insufficient 상태와 인용 근거를 표시한다.
- `Flow & provenance`: 원 Capture, revisions, Task split/관계, 상태 전환, 결과를 보조 view로 제공한다. 기본 작업 화면보다 시각 강도를 낮춘다.

완료된 Task는 읽기 전용 근거를 우선 표시하지만 `follow-up Task 만들기`, 관계 탐색, Knowledge 초안/발행은 허용한다. 완료 Task 원본을 수정해야 하면 새 revision을 암묵적으로 만들지 말고 reopen 또는 후속 Task라는 명시적 행동을 요구한다.

### 선택적 Refinement

Refinement session은 정확히 하나의 Capture 또는 Task에 연결된 독립 aggregate다. 대화를 시작한다고 Capture가 Task로 바뀌거나 Task 상태가 바뀌지 않는다. autosave 단위는 사용자/assistant message, 내부 Problem draft, draft revision, 적용된 제안, 활성 탭, 스크롤 anchor, 입력 draft다. 입력 중 저장은 500ms debounce 후 로컬 DB에 저장하고, 창 닫기·항목 이동·앱 종료 전 flush한다. 저장 실패 시 입력을 메모리에 유지하고 눈에 띄는 재시도 상태를 보여 준다.

AI 응답은 하나의 replacement draft가 아니라 제안 목록을 만들 수 있다.

- `task_patch`: 현재 Task의 필드를 선택적으로 변경한다.
- `new_task`: Capture 또는 현재 Task에서 별도 Task를 만든다. Task에서 나왔다면 기본 `split_from` 또는 사용자가 고른 관계를 연결한다.
- `problem_snapshot`: 대화에서 정리된 독립 Problem 또는 기존 Problem의 새 revision을 내부 draft로 자동 저장한다.
- `task_problem_link`: 제안 Task와 특정 Problem snapshot의 연결을 함께 표현한다.

Task 제안은 독립적으로 적용·편집·거절할 수 있다. 내부 Problem draft는 대화 맥락으로 조용히 autosave하며 별도 Problem 승인 화면을 요구하지 않는다. 사용자가 coherent한 refinement 결과를 적용하거나 Task 시작을 검토할 때 exact Problem+Task snapshot을 한 번에 확인하고 확정한다. 이때 선택한 Task와 필요한 Problem revision/link를 원자적으로 durable하게 만들되, draft Problem을 숨기는 추가 수동 gate를 두지 않는다. 현재 Task는 Refinement 결과를 적용하지 않아도 시작·완료할 수 있다.

### Readiness

Readiness는 aggregate score, 백분율, 등급을 저장하지 않는다. `readiness_field_definitions`와 Task별 평가 결과를 통해 다음 필드를 기본 제공한다.

| 필드 | 적용 조건 | resolved 근거 예시 |
| --- | --- | --- |
| intended outcome | 모든 Task | 비어 있지 않은 현재 Task revision |
| validation/evidence | 검증 가능한 결과를 주장하는 Task | 체크리스트 또는 사용자가 기록한 완료 기준 |
| scope boundary | 외부 변경·여러 산출물이 있는 Task | scope/non-goals 또는 `not_applicable` 결정 |
| prerequisite impact | unresolved prerequisite가 있는 Task | 영향 메모 또는 선행 Task 완료 |
| Problem context | Problem 연결이 필요한 것으로 사용자가 표시한 Task | 연결된 특정 Problem revision |
| conflict evidence | 사용자가 Conflict Review를 요청한 Task | 현재 exact Task revision에 대한 성공 결과와 citations |

응답은 각 필드에 `status`, `reason`, `evidence_refs`, `source_revision`, `applicable_reason`을 포함한다. deterministic 규칙으로 확인 가능한 값은 즉시 계산하고, AI가 제안한 상태는 `suggested` provenance를 표시한다. 사용자가 `not_applicable`을 선택하면 이유와 revision을 기록한다. missing 필드를 누르면 해당 필드 편집 또는 Refinement 질문으로 이동하지만, 시작 버튼은 계속 사용할 수 있다.

### 비차단 Conflict Review

Conflict Review는 Refinement와 병렬로 보이는 별도 job이다. 다음 규칙을 구현한다.

1. 사용자의 명시적 `리뷰 실행` 또는 Refinement에서 material Task draft가 안정된 뒤 자동 보조 검사를 허용한다. 자동 trigger는 마지막 관련 변경 후 800ms debounce, Task별 실행 1개, 전역 provider concurrency 한도를 적용한다.
2. 새 material revision이 생기면 queued job은 취소하고 running job에는 cancellation을 요청한다. provider 취소가 늦으면 결과를 받더라도 적용하지 않는다.
3. 입력 identity는 `task_id`, `task_revision`, 정규화된 비교 필드 hash, Vault index revision, evidence grant/scope revision이다. 결과는 이 identity가 정확히 같을 때만 current다.
4. title/outcome/scope/criteria/연결 Problem revision이 바뀌면 stale이다. category, panel 위치, background timestamp 변화는 stale을 만들지 않는다.
5. 결과 상태는 `queued`, `running`, `clear`, `findings`, `insufficient_evidence`, `failed`, `cancelled`, `stale`이다. `clear`는 성공한 검색 범위와 실제 citations가 있고 모델 결과가 명시적으로 clear일 때만 가능하다.
6. timeout, provider 오류, JSON parse 실패, 근거 없음, 취소, 오래된 결과를 `clear`로 표시하지 않는다. 마지막 current 결과가 있으면 그대로 보존하고 새 시도의 실패를 별도로 표시한다.
7. findings는 인용 excerpt와 source identity를 열 수 있어야 한다. 사용자의 resolution은 새 Task revision 또는 별도 decision record가 되며 AI 결과를 덮어쓰지 않는다.
8. Conflict Review는 Task 생성·`in_progress` 전환·Work Log 기록·완료의 강제 gate가 아니다. unresolved finding은 상태 변경 시 비차단 경고와 명시적 `계속` 결정을 남길 수 있다.

## 제안 데이터 모델

새 테이블과 컬럼 이름은 구현 전 migration spike에서 SQLite 제약과 실제 query plan을 확인한 뒤 확정한다. 아래가 기본안이다.

### 핵심 레코드

`tasks`

- `id TEXT PRIMARY KEY`: 기존 feature ID를 이관할 때 그대로 유지한다.
- `origin_capture_id TEXT NULL`: 직접 생성 Task의 원 Capture. 하나의 Capture에서 여러 Task가 나올 수 있으므로 unique가 아니다.
- `current_revision INTEGER NOT NULL`, `state TEXT CHECK(state IN ('task','in_progress','completed'))`.
- `category`, 중요 표시와 우선순위는 기존 override/projection 테이블을 `entity_type='tasks'`로 이관해 category 행동을 보존한다.
- `created_at`, `last_user_activity_at`, `started_at`, `completed_at`, `reopened_at`.

기존 `captures`는 원문, category, 중요 표시, 생성/사용자 활동 시각을 갖는 canonical 레코드로 유지한다. `source_mode` 또는 같은 의미의 필드로 사용자가 `생각 남기기`로 만든 canonical Capture와 direct Task의 입력 provenance를 구분한다. 후자는 별도 Capture 카드로 중복 노출하지 않고 Task lineage에서 읽을 수 있다. canonical Capture는 Task가 파생된 뒤에도 기록과 category 목록에서 사라지지 않는다.

`task_revisions`

- 복합 기본키 `(task_id, revision)`.
- `title`, `detail`, `outcome`, `scope`, `non_goals`, `validation_criteria`, `content_hash`.
- `author_type`, `source_ref`, `created_at`. 기존 feature 값은 revision 1로 이관한다.

`problems`는 identity와 현재 revision pointer만 남기고, mutable 내용은 `problem_revisions(problem_id, revision, statement, detail, content_hash, author_type, created_at)`로 옮긴다. 기존 Problem은 revision 1이 된다. Problem 상태는 `open/resolved/archived`로 두며 Task 상태와 연동하지 않는다.

`task_problem_links`

- `(task_id, problem_id, linked_problem_revision)` unique.
- `relationship` 기본값 `addresses`, `note`, `created_by`, `created_at`, `unlinked_at`.
- Problem이 새 revision을 가져도 기존 Task의 근거는 원 revision을 유지한다. 사용자가 current revision으로 갱신하는 행동을 별도로 제공한다.

`task_relationships`

- canonical 방향으로 `(source_task_id, target_task_id, kind)` unique.
- `kind`: `prerequisite`, `split_from`, `related`를 저장한다. `split_to`는 `split_from`의 역방향 projection이다.
- self-link, 중복, prerequisite cycle은 거부한다. related는 정렬된 ID 쌍으로 저장해 대칭 중복을 막는다.
- `created_at`, `created_by`, `removed_at`; 실제 행 삭제 대신 relation history를 남긴다.

### 작업 근거와 결정

기존 Work Log 테이블을 즉시 drop하지 않는다. 새 `task_work_log_entries`, `task_work_log_comments`, `task_checklist_items`, `task_attachments`, `task_decisions`로 복사한 뒤 row count와 content hash를 검증한다.

- entry/comment/checklist ID와 created/updated time을 유지한다.
- base64 이미지가 현재 row 안에 있으면 migration에서는 byte를 재인코딩하지 않고 그대로 옮긴다. 후속 attachment storage 개선은 별도 작업이다.
- `task_attachments`는 entry와 media type, original name, byte/hash, AI summary, source legacy ID를 표현한다.
- `task_decisions`는 상태 전환, readiness `not_applicable`, unresolved conflict를 안고 계속한 결정, Problem resolution 기여, reopen을 기록한다.
- Task 완료 근거는 `task_completions(task_id, task_revision, evidence, report, decided_by, created_at)`로 append-only 저장한다. 최신 완료 decision과 Knowledge 발행 상태를 혼합하지 않는다.
- Problem 해결은 `problem_resolution_decisions(problem_id, problem_revision, rationale, evidence_refs_json, created_at)`에 별도로 기록한다. Task 완료 후 UI가 제안할 수 있지만 자동 실행하지 않는다.

### Refinement, 리뷰, 활동

`refinement_sessions`는 nullable `capture_id`와 `task_id` 중 정확히 하나만 갖도록 CHECK constraint를 두고 `state`, `current_draft_revision`, `active_tab`, `scroll_anchor`, `input_draft`, `last_user_activity_at`, `updated_at`을 가진다. `refinement_messages`와 `refinement_drafts`는 append-only sequence/revision, role, content, 내부 Problem snapshot, proposed operations JSON, provider metadata와 오류를 보존한다. `refinement_proposal_decisions`는 제안별 적용/편집/거절과 원자적으로 확정된 Problem/Task target revision을 기록한다.

기존 Problem만 있고 feature가 없는 기록을 위해 `refinement_items`를 둔다. 이 레코드는 `origin_problem_id`, exact legacy Problem revision, canonical category identity, refinement session을 가리키며 Workbench의 혼합 canonical 목록에서 `Refinement` 항목으로 보인다. 새 대화에서 생성되는 Problem draft를 별도 전역 카드로 노출하기 위한 일반 계층은 아니며, legacy 기록을 숨기지 않기 위한 migration bridge다.

`task_readiness_decisions`는 field key, status override, reason, task revision을 저장한다. 계산 결과는 API에서 current revision에 대해 만들며 오래된 AI suggestion을 재사용하지 않는다.

`task_conflict_review_runs`와 `task_conflict_findings`는 기존 Conflict 테이블의 구조를 확장한다. run에 exact identity 필드와 `trigger_kind`, `cancel_requested_at`, `superseded_by_run_id`, timing을 저장한다. resolution은 finding별 append-only decision으로 둔다.

`user_activity_events`를 별도 append-only 테이블로 두거나 기존 `work_tracking_activity_events`를 일반화한다. recent query는 allowlist된 사용자 operation만 사용하고 job update trigger가 이 timestamp를 변경하지 못하게 테스트한다.

### Lineage와 Knowledge

Lineage stage는 고정 네 단계 배열 대신 graph snapshot으로 바꾼다. node type은 `capture`, `task_revision`, `problem_revision`, `work_log`, `decision`, `completion`, `knowledge_revision`; edge type은 `derived_from`, `refined_into`, `linked_problem`, `prerequisite`, `split_from`, `related`, `evidences`, `completed_by`, `published_as`다. 사용자용 기본 view는 Task별 시간 흐름을 보여 주고 graph/provenance는 보조 view에서 펼친다.

Knowledge draft는 정확한 `task_id/task_revision/completion_id/lineage_snapshot_id`를 참조한다. 완료해도 publication state는 `not_requested` 또는 `offered`이고, 사용자가 exact draft revision/hash를 승인한 뒤에만 `published`가 된다. 완료 문서 재생성은 원 기록과 사용자 교정을 보존하며 외부 수정 source hash 보호를 유지한다.

## API와 MCP 전환

breaking API를 한 release boundary에서 바꾼다. 내부 UI와 MCP가 같은 application service를 사용하고, legacy path는 같은 바이너리에 병행 유지하지 않는다.

| 현재 동작 | 목표 동작 |
| --- | --- |
| `POST /captures`, `POST /captures/:id/promote` | `POST /captures`는 canonical Capture 생성, `POST /tasks`는 명시적 direct Task와 카드로 중복 노출되지 않는 입력 provenance를 원자 생성, Capture Refinement apply는 여러 Task를 만들 수 있음 |
| `POST /problems/:id/features` | `POST /tasks` 또는 Refinement `new_task` 제안 적용 |
| `PUT /problems/:id/approve` | 제거; Problem revision/링크 API로 대체 |
| `PUT /features/:id/approve`, `PUT /features/:id/stage` | `POST /tasks/:id/transitions` with expected revision |
| `/features/:id/progress`, `/checklist`, `/comments` | `/tasks/:id/work-log`, `/tasks/:id/checklist`, `/work-log/:entry/comments` |
| `/features/:id/refinement-context`, chat/next-chat | `/tasks/:id/refinement`, `/tasks/:id/refinement/messages`, proposal decision APIs |
| feature Conflict endpoints | `/tasks/:id/conflict-reviews`와 run/finding resolution endpoints |
| feature completion + problem completion | `/tasks/:id/completions`; Problem resolution은 별도 `/problems/:id/resolutions` |
| feature lineage + problem playbook | `/tasks/:id/lineage`, `/tasks/:id/knowledge/*` |
| `/board` captures/problems/features | `/workbench`의 `activeShortcuts`, typed `refiningShortcuts`, `categories[].items`(`capture`/`task`/legacy `refinement`) |

모든 수정은 `expectedTaskRevision` 또는 exact draft/run revision을 요구하고 `head_conflict`에 현재 revision과 충돌 필드를 돌려준다. 생성은 operation ID로 idempotent하게 한다. 응답 timestamp는 storage update와 user activity를 구분한다. endpoint 이름과 DTO는 [`frontend/src/types/application.ts`](../../frontend/src/types/application.ts)와 Tauri command router [`src-tauri/src/native/mod.rs`](../../src-tauri/src/native/mod.rs)에 먼저 typed contract로 정의한 뒤 UI를 연결한다.

MCP는 `ProblemDraft`/`SolutionDraft` 중심 event kind를 `CaptureCreated`, `TaskCreated`, `TaskRevisionProposed`, `TaskLinkedToProblem`, `WorkLogCheckpoint`, `TaskTransitionProposed`, `TaskCompleted`, Knowledge events로 바꾼다. 내부 Problem draft autosave는 workflow 전진이 아니며, Task/Problem snapshot을 durable하게 적용하거나 상태를 바꿀 때 사람의 exact proposal 승인 경계를 유지한다. `workbench_current`와 `workbench_overview`는 typed Capture/Task/refinement item, Task 상태·recent user activity·Problem links·relationships를 반환하며 기존 topic/evidence scope와 cursor snapshot 규칙을 유지한다. 연결 scope 이름에 Solution이 노출되어 있지 않으므로 기존 scope를 최대한 유지하되 tool schema/설명, resource payload, workflow skill reference와 contract test는 한꺼번에 갱신한다.

## 일회성 마이그레이션과 복구

마이그레이션은 레코드를 삭제하지 않고 다음 순서로 진행한다.

1. migration 대상 version이면 먼저 새 writer를 막고 현재 writer를 quiesce한 전용 연결에서 SQLite의 일관된 backup API를 사용한다. 필요하면 같은 배타적 절차 안에서 WAL checkpoint protocol을 적용하며 live DB/`-wal`/`-shm` 파일을 단순 복사하지 않는다. 앱 데이터 디렉터리의 timestamped `migration-backups/task-workbench-<schema>-<timestamp>/`에 만든 backup manifest에는 파일 size/hash, schema version, app version을 기록하고 fsync/독립 연결 재열기 검사까지 성공해야 migration을 시작한다.
2. 새 테이블을 만든다. 기존 `captures`는 canonical 항목으로 그대로 보존한다. 기존 `features` 각각에 같은 ID의 Task와 revision 1을 만들고 실제 legacy 상태 전체를 명시적으로 매핑한다. 기본값은 `proposed→task`, `approved/in_progress→in_progress`, `completed→completed`이며, `archived`는 별도 archive flag/time을 보존해 활성 Task로 되살리지 않는다. 알 수 없는 상태는 추측하지 않고 migration을 중단한다.
3. 기존 `problems` 각각에 Problem revision 1을 만들고 각 feature의 `problem_id`를 exact revision 1의 Task–Problem link로 만든다.
4. orphan Capture는 Task로 바꾸지 않고 원문·category·중요 표시를 가진 canonical Capture로 유지한다. 이미 Problem으로 승격된 Capture도 provenance이자 canonical Capture로 유지한다. Problem만 있고 feature가 없는 경우에는 자동 Task를 만들지 않고 exact Problem revision을 가리키는 migration `refinement_item`을 만들어 기존 category 목록에서 즉시 찾고 Refinement를 재개할 수 있게 한다.
5. Work Log, comments, checklist, completion, Conflict, Lineage, localization, category/priority/deleted marker, work-tracking link를 ID 보존 복사한다. 기존 Problem 완료가 여러 Solution을 함께 닫았던 경우 각 Task completion과 Problem resolution decision을 원 timestamp/이유로 복원하고 `legacy_group_completion` provenance를 남긴다.
6. table별 row count, foreign-key check, ID set, canonical field hash, attachment byte hash, 완료 Task별 Work Log/checklist 수를 검증한다. 검증 전에는 schema version을 올리지 않는다.
7. 검증 성공 후 새 schema version을 commit하고 앱을 새 모델로 연다. 구 테이블은 같은 release에서 drop하지 않고 `legacy_*`로 읽기 금지 보존하거나 backup에만 보존할지 migration spike에서 DB 크기와 SQLite rename 위험을 기준으로 결정한다. 어느 경우든 자동 삭제는 하지 않는다.

오류가 나면 migration transaction을 rollback하고 새 연결을 열지 않는다. startup 화면은 실패한 단계, backup 경로, 안전한 재시도와 `백업으로 복원`을 보여 준다. 복원은 현재 실패 DB를 별도 recovery copy로 이동한 뒤 manifest hash가 맞는 backup을 원자적으로 되돌리고 재검증한다. 사용자가 승인하지 않은 레코드/파일 삭제는 없다. 디스크 부족, copy 중단, corrupt WAL, hash mismatch, 중복 ID, invalid legacy state, 재실행을 자동 테스트한다.

## 구현 단계와 완료 기준

각 단계는 앞 단계가 만든 contract를 사용한다. 단계별 구현 commit을 여러 개 만드는 대신 최종 PR은 저장소 규칙에 따라 하나의 commit으로 정리한다.

### 0. 거버넌스와 executable contract 정렬

- Constitution의 “Organize Around Problems, Not Tasks”, Problem/Solution approval·conflict start gate를 새 제품 결정에 맞게 정식 amendment한다. semantic version, 이유, migration note, Sync Impact Report를 갱신한다.
- Product Spirit 영/한 문서, README의 핵심 흐름, Workbench/Conflict/Completion/Lineage 기능 문서와 인덱스를 새 용어로 설계한다.
- Capture/Task 명시적 입력 모드, Task lifecycle, 상태 전이, revision conflict, relationship cycle, readiness, publication 분리의 domain contract test를 먼저 작성한다.

완료 기준: 기존 governance가 새 흐름을 금지하지 않고, 사용자 권한·private process·publication 경계는 더 약해지지 않는다.

### 1. 스키마와 보존 마이그레이션

- canonical Capture, 새 Task/Problem revision/relationship/Work Log/refinement/review/activity/lineage 스키마와 migration을 구현한다.
- backup manifest, 시작 전 검증, rollback/recovery UI command를 구현한다.
- 작은 fixture, 큰 fixture, 손상/중단 fixture로 migration tests를 작성한다.

완료 기준: 기존 모든 ID와 Work Log byte/hash가 보존되고, 반복 실행이 중복 레코드를 만들지 않으며, 실패 후 구 schema로 열 수 있다.

### 2. 도메인 서비스와 typed API

- Task 생성·revision·상태·관계·Problem link·Work Log·완료/해결·Knowledge publication 서비스를 구현한다.
- readiness deterministic evaluator와 `last_user_activity_at` 갱신 allowlist를 구현한다.
- Tauri route와 frontend DTO를 새 contract로 교체하고 legacy Problem/Solution gate path를 제거한다.

완료 기준: provider 없이도 Task의 전체 lifecycle과 migration된 record 읽기/수정이 동작한다. background job은 recent 순서를 바꾸지 않는다.

### 3. Refinement와 비차단 Conflict orchestration

- autosave/resume session, 다중 proposal, 독립 적용 decision을 구현한다.
- Refinement와 Conflict job의 debounce, per-Task cancellation, concurrency, exact revision stale 판정을 구현한다.
- 실패 결과 보존과 citation evidence navigation을 구현한다.

완료 기준: 사용자가 Refinement/Conflict를 기다리지 않고 Task 작업을 계속할 수 있고, 늦거나 실패한 결과가 current/clear로 보이지 않는다.

### 4. Task 중심 React Workbench와 상세 화면

- `WorkbenchView`의 input, 두 shortcut 영역, canonical category 목록을 React state/query로 구현한다.
- Task 상세의 Work Log, checklist, decisions, relationships, Problems, Refinement, Reviews, provenance를 구현한다.
- 관련 legacy runtime rendering/event binding을 제거하고 shared tokens/components로 상태를 표현한다.
- 넓은/좁은 창, 한국어/영어, 긴/빈/오류/로딩 상태, keyboard와 reduced motion을 검증한다.

완료 기준: 모든 핵심 상태 전환이 실제 렌더된 UI에서 가능하고 shortcut은 canonical 목록과 같은 Task identity를 사용하며 중복 규칙을 지킨다.

### 5. MCP와 Knowledge/Lineage 전환

- work-tracking event/projection, MCP tools/resources, scope descriptions를 Task contract로 교체한다.
- graph lineage snapshot과 Task 기반 Knowledge draft/publish/regenerate/withdraw를 구현한다.
- 완료와 Problem resolution, 완료와 publication의 독립성을 UI/API/MCP에서 동일하게 보장한다.

완료 기준: Chat에서 만든 Task와 데스크톱 Task가 같은 record/revision을 보고, exact 승인 없이 상태 전환이나 발행이 일어나지 않는다.

### 6. 문서, 최종 빌드, 패키지 E2E와 실제 모델 검토

- 아래 문서 영향 목록과 `tests/CHARACTERIZATION.md`를 실제 새 contract/coverage ledger에 맞춘다.
- unit/integration을 통과시킨 뒤 release Tauri build를 한 번 수행한다.
- UI review runner가 rebuild하지 않고 같은 final artifact를 재사용하도록 경로를 추가한 뒤 packaged desktop E2E와 GPT 실제 앱 탐색을 그 artifact에서 수행한다.

완료 기준: deterministic suite와 real-model 평가 보고서가 모두 남고, 문서가 실제 앱에서 생성·발행된 결과와 일치한다.

## 행동과 자동 테스트 추적표

| 사용자 행동/규칙 | 최소 자동 검증 |
| --- | --- |
| 기본 `생각 남기기` → canonical Capture | domain persistence, React interaction, category list E2E, restart persistence |
| `Task로 등록`/명시적 Task 명령 → 즉시 Task | atomic Capture provenance + Task test, React interaction, packaged pointer/keyboard E2E |
| 입력 저장 실패 시 원문 유지 | component failure test, packaged injected DB failure |
| Task `task→in_progress→completed`, reopen | domain transition/revision conflict, UI E2E, audit event assertion |
| Problem 없이 Task 시작/완료 | service + packaged E2E |
| Problem 다대다와 exact revision link | migration/integration, UI navigation E2E, stale linked revision 표시 |
| Work Log 텍스트·이미지·댓글·체크리스트·결정 | DB hash/bytes, UI create/edit/check, restart, completed read-only evidence |
| prerequisite/split/related 생성·탐색·해제 | cycle/duplicate unit tests, 양방향 projection, keyboard navigation E2E |
| shortcut 중복 제거와 canonical 유지 | selector/component tests, activity order E2E |
| background job은 recent 순서를 바꾸지 않음 | clock-controlled repository integration + packaged E2E |
| Refinement autosave/resume | fake timer/component, DB restart, mid-stream cancel/reopen packaged E2E |
| Capture Refinement에서 여러 Task/Problem snapshot | deterministic provider integration, coherent snapshot review와 제안별 accept/edit/reject E2E |
| A→B→A 항목 전환 후 refinement 복원 | unsent input, active tab, scroll anchor, current draft를 각각 assertion하는 packaged E2E |
| readiness resolved/missing/not applicable와 근거 | evaluator table tests, localization/accessibility UI test, no start gate E2E |
| Conflict debounce/cancel/concurrency | fake clock/job scheduler tests |
| exact Task/Vault revision만 current | integration race tests, late response packaged E2E |
| provider/parse/timeout/no evidence ≠ clear | deterministic provider fault matrix + UI state E2E |
| Conflict 중에도 Refinement/Work Log 가능 | packaged concurrent scenario |
| Task 완료 ≠ Problem 해결 | service and UI E2E |
| 일부 Problem만 해결 → follow-up Task | exact 해결 근거와 unresolved Problem link가 follow-up으로 이어지는 packaged E2E |
| Task 완료 ≠ Knowledge 발행 | filesystem assertion before publish, exact revision/hash publish E2E |
| 외부 수정된 Knowledge 보호 | source-hash integration, packaged recovery choice |
| migration/backup/recovery/no deletion | fixture migration suite, kill/restart scenario, file inventory/hash diff |
| MCP와 desktop revision parity | MCP stdio integration + desktop oracle comparison |

## Packaged desktop E2E 시나리오 행렬

현재 [`frontend/src/test/desktopScenario.ts`](../../frontend/src/test/desktopScenario.ts)는 여러 흐름을 하나의 장기 시나리오에서 실행하고 일부 상태 전환을 application JSON으로 직접 주입한다. 새 suite는 fixture/setup과 read-only oracle에 API를 사용할 수 있지만 검증 대상 전환은 렌더된 UI로 수행한다. `clickRenderedButton` 같은 합성 `.click()`만으로 끝내지 않고 OS가 지원하는 native pointer/keyboard adapter 시나리오를 둔다. 시나리오마다 독립 DB/Vault/home, 개별 timeout, 실패 시 screenshot·UI tree·앱/sidecar log·DB copy·Vault diff를 보존한다.

| 시나리오 묶음 | 주요 변형 |
| --- | --- |
| `E2E-CAPTURE-01` 입력 모드 | 기본 Capture, 명시적 Task, mouse, keyboard-only, 한국어/영어, 긴/빈 입력, persistence failure |
| lifecycle | direct start, missing readiness 경고 후 계속, complete, reopen, restart at each state |
| Work Log | text, image, comment, checklist check/uncheck, decision, attachment failure, migrated evidence |
| `E2E-CAPTURE-MULTI-01` Capture→여러 Task | 하나의 Capture refinement, 내부 Problem snapshot autosave, Task 2개 이상, coherent review, partial accept/edit/reject |
| `E2E-REFINE-RESUME-01` | Task A에서 unsent input/탭/scroll 저장 → Task B → Task A, exact 위치/draft 복원; close/reopen, restart, streaming 취소 |
| Problems | no Problem, one Task-many Problems, one Problem-many Tasks, old revision link, 별도 승인 gate 없는 snapshot 확정 |
| `E2E-PROBLEM-FOLLOWUP-01` | Task 완료 시 연결 Problem 일부만 근거로 해결, 나머지는 unresolved 유지, 해당 맥락에서 follow-up Task 생성·왕복 탐색 |
| relationships | prerequisite chain, cycle rejection, split navigation, related symmetry, removed link history |
| Workbench | active/refining shortcuts, no shortcut duplicates, canonical category presence, manual category preservation, user/background recency |
| Conflict | clear with citations, findings, insufficient, provider failure, timeout, malformed response, stale Task, stale Vault, rapid edits/debounce, refinement concurrency |
| completion/publication | complete with unresolved relationship/review, private completion, draft edit, explicit publish, defer, external-file change, regenerate/withdraw |
| migration | every legacy state, orphan Capture, Problem without Solution, multiple Solutions, completed group, images/comments/checklists/localization/category/deleted markers |
| MCP parity | chat-created Task, desktop resume, head conflict, bounded current/overview, scope denial, explicit completion/publication approval |
| resilience/accessibility | offline provider, crash/restart, narrow/wide, focus order, visible focus, reduced motion, long translated text |

지원 OS의 GUI runner에서 `npm run test:desktop`을 CI에 추가한다. 현재 cross-platform workflow는 checks와 `tauri:build -- --no-bundle`까지만 수행하므로 E2E를 실행하지 않은 OS를 통과했다고 표현하지 않는다. macOS/Windows 중 실제 GUI runner와 bundling 조건이 마련된 플랫폼만 증거를 남기고, 나머지는 “packaged E2E unavailable”로 명시한다.

## 검증 전략

빠른 반복 검증은 변경 범위에 맞춘 Vitest와 Rust test target을 사용한다. 최종 코드·문서가 고정된 뒤 다음 명령을 각각 별도로 실행한다.

1. `npm test`
2. `npm run typecheck`
3. `npm run lint`
4. `cargo test --manifest-path src-tauri/Cargo.toml`
5. `git diff --check`
6. release `npm run tauri:build` 한 번
7. 같은 release artifact에 `npm run test:desktop`
8. 같은 artifact에 build 재실행 없는 UI/GPT review runner

기존 `npm test`가 포함하는 Vitest, fake provider, signing/runtime/native boundary validator를 새 contract에 맞게 갱신한다. `tests/CHARACTERIZATION.md`의 퇴역 Python 경로와 obsolete approval gate 설명은 실제 소유 경계와 coverage ledger로 교체한다. 단위 테스트 통과를 packaged native input의 증거로 간주하지 않는다.

성능 budget은 구현 전에 fixture 크기와 측정 harness를 확정한다. Constitution의 capture persistence p95 50ms 기준을 canonical Capture와 Task 원자 생성에 승계한다. 계획 목표는 warm Workbench 1,000 mixed canonical item 조회 p95 100ms 이하, local autosave commit p95 50ms 이하, 사용자 입력 후 UI 저장 상태 표시 p95 100ms 이하, Conflict trigger enqueue p95 100ms 이하로 둔다. Conflict의 queued/running 표시 시간과 실제 첫 evidence/finding 표시 시간을 별도 측정한다. 이는 아직 측정값이 아닌 **구현 목표**이며 hardware, 표본 수, cold/warm 조건과 exact 결과를 보고한다. 기존 binding budget 대비 15%를 넘는 회귀는 수정하거나 governance amendment가 필요하다.

## 실제 모델 앱 탐색과 품질 평가

deterministic fake provider suite를 release gate로 먼저 통과시킨다. 이후 별도 real-model run은 제품의 기본/설정 모델과 실제 packaged app을 사용하고, 결과를 fixture expectation처럼 고정하지 않는다. 모든 run은 모델 ID, 설정, corpus version, Task/Vault revision, 시작/종료 시각, latency, token/오류, citations와 사용자-visible 결과를 기록한다. 민감한 사용자 Vault 대신 고정된 평가 Vault를 사용한다.

평가 corpus는 최소 다음을 포함한다.

- 명확한 direct Task와 모호하지만 시작 가능한 Task.
- 한 대화에서 호환되는 여러 Task/Problem이 나오는 refinement.
- 기존 Knowledge와 명백히 모순되는 Task, 양립 가능한 유사 작업, 근거가 없는 작업.
- Task 수정과 Vault 수정으로 각각 stale이 되는 늦은 응답.
- 한국어, 영어, 혼합 언어와 긴 근거.
- Work Log 이미지/체크리스트/결정에서 완료 문서를 만드는 사례.

두 명 또는 사전 합의한 adjudication rubric으로 expected `clear/findings/insufficient`, 필수 citation span, 허용 가능한 proposal facts를 표시한다. 아래 수치는 측정 결과가 아닌 **제안 목표**다.

| 지표 | 제안 목표 |
| --- | --- |
| 명백한 모순 recall | ≥ 90% |
| `clear` precision | ≥ 95%; failed/insufficient/stale의 false clear는 0건 |
| finding citation entailment | ≥ 95% |
| citation source/revision 정확성 | 100% |
| unsupported material claim | ≤ 2% |
| 다중 refinement 제안의 요구 보존 | ≥ 90% rubric pass |
| Conflict time-to-visible-state | p95 ≤ 1초(queued/running 표시) |
| Conflict time-to-first-evidence | p50 ≤ 5초, p95 ≤ 12초; 근거가 없으면 `insufficient_evidence` 첫 판정까지 측정 |
| real-model conflict completion | p50 ≤ 8초, p95 ≤ 20초 |
| cancel/stale suppression | 100% |
| 완료 문서 필수 section/근거 link | 100% |
| Task/Problem/결정 사실 정확성 | 100%; AI 해석은 provenance 표시 |

표본 수가 작으면 비율만 쓰지 않고 `통과/전체` exact count와 Wilson interval을 함께 제시한다. latency 목표를 넘겨도 UI가 비차단이고 즉시 running 상태를 보여야 한다. 실제 모델 탐색은 최소 다음 사용자 경로를 끝까지 수행한다: direct Task 생성, Refinement 재개와 다중 제안, Conflict와 동시 Work Log, stale 결과 억제, 완료, private Knowledge draft, 명시적 발행, 생성 Markdown 재열기. 최종 검토자는 앱에서 문서를 열어 제목·결과·Problem revision·결정·Work Log/첨부·Conflict citations·completion evidence·publication lineage가 DB 원 기록과 맞는지 확인한다.

## 문서와 거버넌스 영향 목록

구현과 같은 task에서 영문/한국어 쌍을 맞춰 갱신한다.

- `.specify/memory/constitution.md`: Principle IV, Guardrail D의 approval/start gate, Product Spirit gate, version과 migration note.
- `docs/product-spirit.md`, `docs/product-spirit.ko.md`: Task 중심 조직, optional Problem/refinement, Task Work Log.
- `README.md`, `README.ko.md`: 핵심 workflow와 화면 설명.
- `docs/features/conflict-gated-workflow.md`, `.ko.md`: 파일명과 내용 모두 `task-centered-workbench`로 교체하고 feature index 링크 갱신.
- `docs/features/refinement-preview-status.md`, `.ko.md`: autosave/resume, 다중 proposal, field readiness.
- `docs/features/completion-writeback-archive.md`, `.ko.md`: Task completion과 Problem resolution/Knowledge publication 분리.
- `docs/features/lineage-knowledge-layer.md`, `.ko.md`: graph lineage와 Problem revision link.
- `docs/features/mcp-workbench-bridge.md`, `.ko.md`: Task event/tool/resource contract.
- `docs/features/background-ai-queue.md`, `.ko.md`: debounce/cancel/stale/failure와 recent activity 분리.
- `docs/database-migrations.md`, `.ko.md`: backup manifest, restore, Task migration.
- `docs/architecture.md`, `.ko.md`: Task aggregate, adapters, React/runtime 소유 경계.
- `docs/features/visual-guide.md`, `.ko.md`와 새 screenshots: 상단 shortcuts, canonical category, Task 상세.
- `tests/CHARACTERIZATION.md`, `docs/testing/button-coverage-matrix.md`: 새 truth/coverage ledger와 native pointer/keyboard 증거.
- `docs/CONTINUATION.md`: rollout 후 남은 제한이나 일부 OS E2E 미실행이 있을 때만 기록.
- `.agents/skills/llm-wiki-workflow/SKILL.md` 및 직접 참조 문서/contract tests: Chat 흐름이 더 이상 Problem/Solution 승인 순서를 강제하지 않게 갱신.

기존 Spec Kit 기록은 당시 구현의 역사 자료이므로 이 작업에서 소급 수정하지 않는다. 새 구현이 별도 feature spec을 필요로 한다면 이 계획에서 파생한 새 spec ID를 만들되, 사용자가 요청하지 않은 전체 Spec Kit scaffold를 계획 단계에서 미리 생성하지 않는다.

## 위험, rollout 판단, 열린 세부사항

- 가장 큰 위험은 기존 `features`/`problem_id` 가정이 DB, legacy UI runtime, job payload, localization, completion, lineage, MCP projection에 분산된 점이다. schema만 먼저 바꾸지 않고 vertical slice와 migration fixture를 함께 유지한다.
- 기존 완료 동작은 한 Problem의 모든 Solution을 닫는다. migration은 기록된 결과를 보존하지만 새 완료 semantics와 다르므로 `legacy_group_completion`을 사용자에게 사실 그대로 표시한다.
- direct Task 입력 provenance와 사용자가 `생각 남기기`로 만든 canonical Capture를 같은 화면 카드로 취급하면 중복이 생긴다. 저장 모델에서 source mode를 구분하고 direct Task provenance는 lineage에서만 보여 주며, 사용자가 만든 Capture는 canonical category 목록에 계속 둔다.
- 일반 Problem draft는 Refinement 내부에서 계속 보존하고, Task와 연결된 확정 Problem은 Task 상세·provenance에서 탐색한다. legacy Problem-only record는 migration `refinement_item`으로 기존 category 위치를 보존한다. 별도 전역 Problem 화면은 이번 범위에 추가하지 않는다.
- `task`라는 state 명칭은 제품 문구에서는 “할 일” 또는 locale 번역을 사용하되 API enum은 요구 흐름과 맞춰 `task`로 둔다. 구현 전 contract test에서 이 값을 고정한다.
- legacy table을 성공 migration 직후 rename 보존할지 backup에만 둘지는 실제 DB 크기, startup 비용, support 복구 절차를 측정해 결정한다. 어느 선택도 자동 영구 삭제를 포함하지 않는다.
- attachment를 DB blob/base64에서 파일 저장으로 옮기는 일은 기록 보존 위험을 키우므로 이번 migration에서는 표현만 일반화하고 byte storage 변환은 별도 검증된 작업으로 남긴다.

이 계획의 구현 완료 판정은 “새 화면이 보인다”가 아니라 기존 기록 보존, Task 중심 도메인 일관성, 비차단 AI 상태의 정확성, UI로 수행한 packaged E2E, 실제 모델의 근거 품질, 생성 문서의 원 기록 일치가 모두 증거로 남았을 때 가능하다.
