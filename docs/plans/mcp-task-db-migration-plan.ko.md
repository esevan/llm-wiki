# MCP Task 중심 DB 마이그레이션 계획

상태: v9 구현 완료; 데이터가 있는 migration·rollback·recovery fixture 통과
범위: 스키마와 데이터 마이그레이션 검토만 포함하며 구현은 별도 작업
검토일: 2026-09-12

## 결정 요약

최초 검토 당시 지원하는 최신 네이티브 스키마는 `CURRENT_SCHEMA_VERSION=8`이었다. live DB는
검사하지 않았다. 이번 구현은 아래의 좁은 v9 마이그레이션을 추가한다. 당시 `migrations.rs`는 1–8을 순서대로 적용하고
각 트랜잭션 안에서 commit 전에 `PRAGMA user_version`을 설정한다(`migrations.rs:649–652`). v8
`migrate_task_centered_workbench`는 Task aggregate와 assistance 테이블을 만들고, 기존
feature/work 기록을 복사하며, legacy ledger를 `legacy_task_migration_records`에 보존하고,
카운트·필드 일치·외래 키를 검증한다. 따라서 MCP 보완 작업은 기존 테이블로 강제할 수 없는
영속 불변식이 확인될 때만 좁은 버전 마이그레이션으로 진행해야 한다. 확인된 필수 Capture
제약 때문에 직접 Task를 이어가기 위한 아래의 정확한 v9가 필요하다. 과거 테이블 삭제,
이벤트 재작성이나 두 번째 대규모 변환을 요구할 근거는 없다.

주요 관찰 gap은 query/projection 및 contract/skill 문제다. `current_workbench`는
`work_tracking_sessions`만 읽고, `overview`는 Task board와 pending event를 따로 읽으며,
workflow skill에는 legacy action 용어가 남아 있다. 이 경로들은 현재 native/MCP에서 실제로
도달 가능하므로 DB 변경 없이 고칠 수 있다. 검토된 스키마 변경은 더 좁다. v8은 모든 legacy
Workbench mutation trigger를 제거하므로 v9에 SQL mutation-event trigger를 다시 만들지 않는다.
승인된 v9는 `work_tracking_sessions.capture_id` 제약을 재구성하는 것뿐이며, 나머지는
service/projection/contract/skill 작업이다.

## 확인한 사실

- `migrations.rs`: `CURRENT_SCHEMA_VERSION=8`; 1–8 순서; v8은 `problems`의 capture unique
  제약을 재구성으로 제거하고, feature ID를 같은 Task ID와 revision 1로 옮기며,
  Work Log/checklist/completion을 복사하고, count/hash/foreign-key 불변식을 검사한다.
- `database.rs`: WAL, foreign key, 업그레이드 전 `VACUUM INTO` 백업, manifest/hash/integrity
  검사와 검증된 restore가 있다. 실패 시 버전을 올리지 않고 안전한 단계 marker를 남긴다.
- `task_schema.sql`, `work_tracking_schema.sql`, `task_assistance_schema.sql`: Task revision,
  exact Problem link, append-only completion/decision, MCP session/event, projection
  outbox/result/watermark, idempotency, refinement/review/Knowledge 저장 구조가 이미 있다.
- 서비스/SQLite adapter: scope check, head CAS, request hash, projection, current/overview,
  review challenge, publication hash guard가 있다. `current_workbench`는 session-capture
  join이고, `overview`는 board와 미결정 proposal event를 합친다.
- 이 검토에서는 live database를 열거나 변경하지 않았다.

## 버전, canonical/legacy, 도달성

버전 순서는 1 native baseline, 2 legacy normalization, 3 AI job defaults, 4 dual-chat work
tracking, 5 bound reviews, 6 Workbench timestamp, 7 solution-child timestamp, 8 Task 전환이다.
v8 이후에도 `features`, `solution_progress_*`, `completions`와 ledger가 삭제되지 않는다.
canonical 읽기는 `tasks`/`task_revisions`와 Task 소유 work table을 사용한다. 기존 테이블이
남아 있다는 사실만으로 모든 legacy route가 여전히 도달 가능하다고 가정하면 안 된다.

v8은 `proposed→task`, `approved|in_progress→in_progress`, `completed→completed`로 옮기고,
알 수 없는 상태는 중단한다. Problem은 revision 1이 되고 feature의 `problem_id`는 exact
revision-1 link가 된다. Problem-only 기록은 refinement item이 된다. 이 매핑은
`MigrationSnapshot::validate`가 보호한다.

`mcp_connections`는 scope/topic grant를, `work_tracking_sessions`는 connection과 대화
lineage를 소유한다. event는 session revision/stream sequence unique이고, decision은
source event와 payload hash에 묶이며, projection job/result/watermark가 replay와 late event를
기록한다. 이 구조는 MCP scope에는 충분하지만 session이 없는 desktop direct Task를
자동으로 나타내지는 않는다.

work-tracking schema에는 linked legacy trigger 정의가 남아 있지만 v8의
`drop_legacy_workbench_triggers`가 problem/feature/progress/checklist/completion trigger를
모두 제거한다. 따라서 활성 mutation producer가 아니다. v9에 SQL trigger를 추가하지 않고,
현재 Task 변경은 shared application mutation helper/append 경로로 기록한다. 기존 event는
그대로 보존한다.

`task_completions`는 Task revision에 묶인 append-only 근거이고, `knowledge_drafts`와
`task_knowledge_drafts`는 completion/draft hash와 lineage를 보존한다. 그러나 assistance
Knowledge 테이블에는 Problem revision 외래 키가 없다. 검토된 계약은 정확한 Task
revision/completion에서 Problem revision, 역사적 link/relationship,
evidence를 immutable lineage snapshot으로 캡처하고 참조의 불변 ID·revision·내용을 검증하는
것이다. 현재 active link나 변할 수 있는 unlink/retirement metadata와 비교하지 않는다.
`content_hash`는 body만 해시하고 source/lineage hash는 review JSON에
별도로 둔다. 같은 revision의 미발행 수정도 exact revision/hash를 묶고, 발행 후 수정은 새 draft다.
기존 `knowledge_drafts` history는 보존하며 모든 신규 Task 흐름은 desktop과 같은 서비스에서
canonical `task_knowledge_drafts`를 사용한다. 미해결 옛 제안·job만 새 검토 전까지 안전하게
막고 기존의 검토된 발행·복구 기반 구조를 재사용한다.

현재 `overview`에는 Problem link, Task decision, relationship, completion/publication lineage가
없고, `current_workbench`에는 tracked session에 없는 desktop Task가 없다. 두 문제는 저장
데이터 부재가 아니라 bounded service projection/query 계약의 문제다.

## Snapshot과 변경의 최신성

overview는 board와 pending attention hash를 사용한다(`adapters/sqlite/mod.rs:2110–2117`).
current 선택·link 검토는 `work_tracking_workspace.revision`을 사용한다. 현재 overview는 서로
다른 connection에서 구성 요소를 읽고 desktop Task 변경은 workspace counter를 증가시키지
않는다(`task_repository.rs:49–65`). 이는 스키마 부재가 아닌 애플리케이션 문제다. 전체 조회를
한 SQLite 읽기 transaction에서 수행하고 모든 반환 의존성을 hash에 포함한다. 공통 변경
경로가 workspace와 대상 material 최신성을 원자적으로 무효화해야 한다. 링크·관계·상태·
Work Log·checklist·review·Knowledge를 포함하며 Task 내용 revision만으로는 부족하다.
연결된 세션에는 변경을 한 번만 기록하고 notification projection이 도메인 변경을 중복
실행하지 않는다. 세션 없는 Task는 가짜 세션 없이 canonical 이력을 유지한다.

## 변경 분류

| 문제 | 스키마 필요성 | 확정 처리 |
| --- | --- | --- |
| session 없는 desktop Task의 current 표시 | 탐색에는 없음 | canonical Task와 tracked session을 중복 없이 scoped 조회한다. |
| Capture 없는 기존 Task의 명시적 이어가기 | 아래의 정확한 v9 필요 | 기존 session Capture FK만 nullable로 하고 기존 Task link와 pre-session 검토를 사용한다. |
| overview의 Problem link/decision/relationship | 없음 | 현재 Task table과 exact revision을 사용하는 bounded DTO로 확장한다. |
| legacy advance와 payload noun | 없음 | 실제 도달성을 확인한 뒤 새 legacy 흐름은 거부하거나 history read만 유지한다. |
| skill의 legacy event 용어 | 없음 | skill/contract/fixture를 고친다. 이벤트 backfill은 하지 않는다. |
| Task 변경의 event/outbox 기록 | 새 SQL trigger 없음 | shared mutation helper/append 경로를 사용하고 historical event byte를 보존한다. |
| Knowledge의 exact Problem lineage | 새 normalized FK 없음 | immutable snapshot을 캡처·검증하고 current active link와 비교하지 않는다. |
| legacy table/event | 없음 | drop/rewrite하지 않고 migration ledger와 read-only compatibility를 유지한다. |

## 검토된 v9 계약

v9의 목적은 Capture를 만들지 않고 기존 Task를 명시적으로 이어가는 것이다. 탐색은 세션
없이 유지한다. 따라서
`work_tracking_sessions.capture_id TEXT NOT NULL UNIQUE`를 `TEXT UNIQUE`(nullable)로
재구성한다. task/session/event/lineage table이나 `task_id` column은 추가하지 않는다. 기존
`work_tracking_links`가 Task binding으로 남는다.

writer를 quiesce하고 기존 verified backup pattern을 사용한다. 초기 foreign key integrity를
확인한 뒤 v9 transaction 시작 전에 migration connection의 enforcement를 끄고
replacement-copy-drop-old-rename을 수행한다. 모든 column/value, child/self-parent chain,
foreign key, unique constraint, session index를 보존하고 revision/timestamp trigger와 index를
명시적으로 재생성한다. 먼저 index/trigger를 목록화하고 child FK가 임시 replacement table을
가리키도록 바꾸지 않는다. row/ID equality,
immutable event byte/hash, idempotency/decision/job/
link, session head, index/trigger, FK와 SQLite integrity를 검증한다. backfill과 historical
event 재작성은 없다. `user_version=9`는 transaction 안에서 설정하며 실패하면 v8로 rollback한다.
성공·실패 모두 enforcement를 다시 켜고 pragma와 전체 FK를 확인한 뒤에만 application을
재개한다. writable_schema나 원래 parent table을 먼저 rename하는 방식은 사용하지 않는다.

구현 중 정정: 원래 Astra가 검토한 FK-ON/deferred 방식은 자식 레코드가 있는 교체에서
foreign_key_check가 정상이더라도 COMMIT에 실패했다. 재현 결과는
`.tmp/sqlite-parent-rebuild-repro.json`(진단 SQLite 3.51.0)에 보관하며 application은 SQLite
3.46.0을 번들한다. [SQLite 공식 ALTER TABLE 절차](https://www.sqlite.org/lang_altertable.html#otheralter)에 따라 정정한다. FK 정의는 보존하고 quiesced migration connection의 검사만
일시 중단한다. 추가 Astra 자문은 agent thread 한도로 실행되지 않았다. 이 정정의 근거는
공식 절차와 동작 테스트이며 추가 Astra 승인으로 표현하지 않는다.

세션 없는 탐색은 정해진 bounded 읽기 권한과 해당 topic 소속을 요구하며 읽기 권한만으로
이어가기를 승인할 수 없다. 명시적 이어가기는 active `session:write`와 authorized discovery
scope를 함께 요구한다. 수락 시 grant·membership·target ID·revision·material hash를
다시 확인한다. exact target acceptance 때 connection-owned captureless session과 initial
event/link/idempotency를 generic pre-session review 경로에서 원자적으로 만든다. 취소하면
session을 만들지 않고, 다른 connection session을 탈취하거나 Capture/lineage를 만들지 않는다.
초기 event는 기존 Task 연결을 뜻하며 그 Task나 Capture를 새로 만들었다고 기록하지 않는다.
Capture 필수 join/open/DTO를 두 세션 형태에 맞게 수정한다. 다른 Task/Problem 연결에는 별도
권한과 정확한 검토가 필요하다. session 상세·event는 owner scope를 유지한다. session 없는
Task history는 fake session 없이 canonical하게 읽는다.

지원되는 older/version-zero DB는 순서대로 upgrade한다. 구버전 binary는 v9에서 fail closed하되
지원되는 낮은 schema를 모두 거부하지 않는다. 신버전은 필수 table/invariant가 없으면 fail
closed하며 downgrade는 없다.

## 불변식, 복구, 검증 fixture

기존 canonical field ordering과 SHA-256 hash를 그대로 쓴다. 같은 operation ID와 request hash는
저장된 JSON을 그대로 replay하고, 다른 hash는 conflict다. event head CAS, event, idempotency는
한 transaction이다. completion과 publication은 분리하며 completion이 publication을 뜻하지
않는다. API key/provider secret은 event payload, manifest, lineage, activity에 넣지 않는다.
설정은 `settings.json`에 원자적으로 저장하며 keychain 보장을 이 계약에 포함하지 않는다.

empty v8, 완전한 legacy 변환, orphan Capture, Problem-only, 다중 Task/Problem, duplicate
operation, revision drift, grouped completion, 큰 attachment, localization/override/deleted,
legacy event, session 없는 desktop Task, linked session, pending/late/duplicate event,
publication hash drift, invalid state/FK, 중단 backup/migration, corrupt WAL/SHM, low disk,
hash mismatch, retry/restore, older binary, captureless continuation acceptance/cancel, wrong-connection
takeover, unauthorized target/revision/grant, session-head preservation, current link가 바뀐 뒤의
historical lineage, 다중 connection 이어가기, grant/topic 회수, 모든 session child 참조·parent
chain, index/trigger 복구 동작, desktop 직접 변경의 snapshot 무효화와 중단 발행 복구 fixture를
준비한다. row loss, fabricated Task/Problem/Capture, historical
event byte 변경, hash/revision, atomic rollback, secret 없는 safe error를 단정한다.

## Astra 검토 기록

Astra가 2026-09-12 high reasoning으로 검토했다. 위 수정 사항을 반영한 conditional approval이며,
검토 당시에는 runtime 구현과 live DB 검증을 수행하기 전이었다. 구현에 요구한 검증은 legacy adopt/approve
Problem/Solution route 도달성 audit, 새 legacy action 거부와 historical completed event
readability 확인, unresolved old proposal의 fresh canonical review다. 새 adoption/approval,
mandatory conflict, grouped completion은 replay하지 않는다. 이미 승인된 legacy publication
job만 원래 reviewed draft/hash와 외부 파일 guard를 사용해 기존 handler로 복구할 수 있다.
그 외 unresolved legacy mutation job/proposal은 보존하고 fresh canonical review 전까지
안전하게 막는다.

구현 시 사용자 정의 세션 인덱스·트리거뿐 아니라 세션 테이블을 참조하는 뷰도 SQL을 보존하고 재생성한다. 실제 v8 fixture에서 뷰 조회와 트리거 실행을 검증한다.
