# Task 중심 MCP 수정 실행 계획

**작성일:** 2026-09-12
**상태:** GPT-6 Astra/high 계획 검토 후 구현·최종 검사·macOS 서명 릴리스·전체 패키지 E2E·앱과 plugin 설치 완료.
실제 결과는 [검증 기록](../testing/mcp-task-workflow-verification.ko.md)에 정리한다.

## 목표와 범위

로컬 MCP와 desktop이 같은 Task 중심 작업을 제공하도록 수정한다. Capture는 가벼운 입력이며,
정제, Work Log, 정확한 Problem revision 연결, Task 관계, 자문 검토, 완료,
Knowledge 초안·발행은 독립된 결정이다. 서버의 조회·전환뿐 아니라 대화 스킬과 설치된
플러그인 전달까지 포함한다. 기존 권한, 식별자, 과거 근거와 사용자 소유 파일을 보존하며
퇴역한 Python/browser 제품을 복구하지 않는다.

선행 DB 검토와 마이그레이션 설계는 [DB Migration Plan](mcp-task-db-migration-plan.ko.md)에
기록한다. 확정한 v9 변경은 기존 세션의 Capture 참조를 nullable로 만드는 것뿐이다. 기존
레코드는 모두 보존하고 나머지는 application-service, 조회, DTO, runtime, 스킬과 플러그인
전달을 수정한다.

## Astra 검토를 반영한 결정

- v8은 옛 Workbench 변경 트리거를 제거한다. SQL 이벤트 트리거나 별도 이벤트·세션 저장소를
  추가하지 않는다. desktop, MCP, 정제 채택과 projector는 같은 트랜잭션 변경 경로를 사용하며
  실제 operation hash와 정확히 한 번의 이벤트 처리를 보장한다.
- canonical 탐색은 세션 없이 한다. 명시적 이어가기는 기존 Task를 검토한 뒤 해당 연결이
  소유하는 Capture 없는 세션, 연결 이벤트·링크와 멱등성 결과를 원자적으로 만든다. 거부하면
  아무것도 만들지 않는다. 기존 Capture 세션은 유지하고 다른 연결의 세션 소유권·상세를
  가져오거나 노출하지 않는다.
- 이어가기는 `session:write`와 해당 탐색 권한을 함께 요구한다. 기존 권한과 명시적 topic
  소속을 보존하고 승인 시 재검사한다. Task 연결이 다른 Task/Problem의 권한을 부여하지
  않으며 추가 대상은 독립적인 권한과 정확한 검토를 요구한다.
- current/overview/session은 전체 의존 데이터를 한 SQLite 읽기 트랜잭션에서 조회한다.
  overview는 현재 board와 attention hash를, current/link 검토는 workspace revision을
  사용한다. 공통 변경 경로가 workspace를 무효화하고 상태, 링크, 관계, Work Log,
  checklist, review, Knowledge까지 포함하는 대상 hash를 반환해야 한다. overview snapshot
  hash도 모든 반환 필드의 의존성을 포함하고 페이지·만료 규칙을 보존한다.
- 신규 Task Knowledge는 `task_knowledge_drafts`와 공통 서비스로 통일한다. 같은 읽기
  snapshot에서 정확한 Task/completion/Problem revision, 과거 링크·관계·근거를 담은 불변
  Lineage를 만든다. 과거 참조 레코드와 검토된 본문·source hash를 검사하며 이후 활성 링크
  변경이 유효한 과거 Lineage를 무효화하지 않는다. 본문 hash와 별도로 Lineage/source hash를
  기존 review JSON에 묶는다.
- 옛 이벤트, 초안, 발행 결정과 파일은 명시된 이력으로 보존한다. 신규 옛 승인 흐름과 묶음
  완료는 거부하며 미해결 옛 제안은 새 canonical 검토를 요구한다. 이미 승인된 옛 발행 job은
  기존 복구 경로에서 원래 검토된 draft/hash와 파일 보호 조건으로만 복구한다. 다른 미해결
  옛 변경 job·제안은 새 canonical 검토 안내를 포함한 안전한 blocked 상태로 두고 모든 기록을
  보존한다. 조용히 버리거나 시작 시 legacy 도메인 쓰기를 재실행하지 않는다.
- MCP의 충돌 추론은 현재 Chat AI가 수행한다. native assistance와 저장·결정 경로를
  통합하면서 숨은 provider 충돌 job을 만들지 않는다.

## 필수 액션·검토 대응표

각 행에 명시적 MCP 스키마·액션, 권한 있는 대상, 트랜잭션 서비스 진입점과 동작 검증이 있어야
한다. 가능한 한 기존 도구를 확장하되 임의 native operation을 그대로 호출하는 통로는 만들지
않는다. 검토는 연결, operation, 액션, 정확한 대상, payload hash, 관련 revision·material hash와
해당 source proposal을 묶고 원자적으로 재검사한다. Task 내용 revision만으로 child·상태
변경을 감지할 수 없다.

| 기능 | canonical 진입점·동작 | 결정 경계 |
| --- | --- | --- |
| 탐색·이어가기 | current/overview/session DTO, 기존 Task의 명시적 이어가기 | scoped 읽기와 정확한 연결 검토; 가짜 Capture 없음. |
| Capture·정제 | `capture.create`, `task-refinement.*` message/workspace/proposal/decision | Capture 수락과 제안 채택 분리; 자동 Task/Problem 채택 없음. |
| Task·Problem revision | `task.create/revision`, `problem.create/revision` | 정확한 불변 revision 제안 채택. |
| Problem 연결 | `task.problem-link.create/delete` | 정확한 Problem revision·과거 link ID; 해제해도 이력 보존. |
| Task 관계 | `task.relationship.create/delete` | 양쪽 대상 권한·검토, related 정렬·split 역관계·중복/자기연결/선행 순환 거부. |
| 준비도·작업 근거 | readiness get/decision, Work Log get/create, comment·attachment 근거, checklist create/update, Task decision | 의미 있는 checkpoint 정책 유지; governed 결정은 정확한 근거·대상 hash 사용. |
| 자문 검토 | review create/get/history/cancel/decision, 현재 Chat의 인용된 findings | 자문·취소·stale 처리; 필수 완료 관문이나 숨은 충돌 모델 없음. |
| 상태·완료 | `task.transition`, reopen, `task.completion.create` | 정확한 aggregate·근거 검토; Problem 해결·발행과 독립. |
| Problem 해결 | `problem.resolution.create` | 정확한 Problem revision·권한·명시적 근거; Task 완료가 해결하지 않음. |
| Lineage·Knowledge | `task.lineage`, draft/correction/regenerate/publish/withdraw | Task 초안 저장소 통일, 정확한 revision+본문/source hash; 발행 후 편집은 새 초안, 발행·회수 분리. |

첨부는 허용된 근거 참조나 승인된 host 입력으로 처리하고 MCP가 임의의 절대 파일 경로를
요구하지 않는다. Capture·정제와 일반 checkpoint의 기존 독립 정책을 보존하여 모든 대화
턴을 검토 UI로 만들지 않는다.

## 실행 순서

1. 선행 단계 완료: 소스 기반 DB 검토, DB Migration Plan과 Astra 집중 검토를 수행했다.
   반영된 결정과 정확한 v9 계약을 기준으로 애플리케이션을 수정한다.
2. `scripts/create_task_worktree.sh mcp-task-workflow fix/mcp-task-workflow`로 전용 worktree를
   만든다. 기본 checkout의 AGENTS.md, commit 스킬, CONTINUATION, 원래 Task 중심 계획과
   두 UI/UX 계획을 포함한 기존 변경을 모두 보존한다. 적용되는 최신 지침을 worktree에
   명시적으로 전달하고 그곳에서 구현·검증한다.
3. desktop에서 직접 만든 Task의 MCP 노출, 정확한 Problem 연결, Task 관계, 보류 중인 결정,
   일관된 overview 페이지와 오래된 검토 거부를 재현하는 실패 테스트부터 추가한다. 이전·현재
   기록이 섞인 fixture와 도구 스키마에 대한 원본·설치 스킬 계약 검증을 포함한다.
4. nullable-Capture v9를 조회·전환보다 먼저 구현한다. 다른 컬럼, FK, unique, 레코드,
   child ID와 살아 있는 session trigger/index를 보존한다. v9 transaction 전 connection-local FK enforcement를 끄고 모든 결과에서 재활성화를
   검증하는 SQLite 공식 replacement/copy/drop/rename 순서를 사용하고 child·parent session chain을 검증한다. user_version은 commit 전에
   같은 트랜잭션 안에서 설정한다. 폐기 가능한 fixture로 백업·복구를 증명하고 Capture 필수
   join/DTO/open 경로를 기존·captureless 세션 모두에 맞게 수정한다.
5. 제한된 application-service 조회를 통해 current/session/overview 응답을 수정한다. 연결
   권한, 명시적 topic 소속과 snapshot 페이지 일관성을 보존한다. Task 상태·revision, 정확한
   Problem 연결, Task 관계, 의미 있는 활동과 보류 결정을 포함한다. desktop 직접 생성 Task도
   이전 단계 흐름을 강제하지 않고 읽고 이어갈 수 있어야 한다.
6. 액션·검토 대응표의 모든 행을 Task 서비스로 구현한다. 신규 `adopt_problem`,
   `approve_problem`, `adopt_solution`, `approve_solution`, 필수 충돌 관문과 묶음 완료
   `verify_and_complete`는 거부한다. 과거 기록과 canonical 이어가기,
   operation 멱등성과 head/대상 revision 충돌을 보존한다. 완료가 Problem 해결이나 Knowledge
   발행을 암묵적으로 수행해서는 안 되며 자문 검토가 필수 승인 관문이 되어서는 안 된다.
   consumer 수정 전에 신규 Knowledge를 Task 초안 서비스와 불변 source snapshot으로 통일하고
   옛 job·이력은 위 호환 정책으로 보존한다.
7. 앱 내 추적 카드, MCP 스키마·액션 설명, 대화 스킬과 overview/conflict 참조를 정렬한다.
   실제 지원 액션과 payload를 구체적으로 설명한다. 서버 DTO만으로 통합 성공을 가정하지 않고
   host가 임의 JSON patch와 근거 객체를 표현할 수 있는지 확인한다.
8. 과거 v8 동작을 재작성하지 않고 EN/KO 기능 문서와 현재 API·마이그레이션 계약을 갱신한다.
   생성된 설치 캐시 파일을 직접 수정하지 않고 플러그인 개발 절차로 로컬 플러그인을 갱신한다.
   기존 연결 ID, 설정과 권한을
   보존한다. 새 작업에서 스킬 로딩을 확인하고 elicitation·외부 host 제한은 소스 검증 성공과
   구분하여 기록한다.
9. 마지막 소스·문서 변경 후 필수 검증과 최종 릴리스 빌드·packaged E2E를 한 번 수행한다.
   정확한 artifact와 fixture 식별, 새 MCP/플러그인 근거, 마이그레이션 결과와 외부 검증 항목을
   기록한다. 소스 검증을 설치된 플랫폼 검증으로 표현하지 않는다.

## 검증과 완료 조건

- `npm test`, `npm run lint`, `npm run typecheck`, `npm run build`를 실행한다.
- Rust format 검사, strict all-target clippy와
  `cargo test --manifest-path src-tauri/Cargo.toml`을 실행한다.
- `npm run tauri:build` 후 최종 artifact에 `npm run test:desktop`을 실행하고 근거를 task
  worktree에 보존한다. 문서의 `--full` 모드를 사용하고 변경된 MCP/desktop 동작에 대한
  packaged 시나리오를 추가한다.
- `git diff --check`를 실행한다. 전체 lint에는 기존 21건이 보고되어 있으므로 실제 최종 결과와
  수정 범위 결과를 기록하고 기존 오류가 남으면 전체 통과라고 표현하지 않는다.
- 검토된 DB 계획에 따라 빈 DB·최신·이전 버전, 실패·중단·재시도·복구 fixture를 검증한다.
  정확한 참조와 과거 이벤트 식별자가 보존되는지 증명한다.
- desktop 직접 Task 생성 → 권한 있는 MCP 읽기·이어가기 → Task 수정 → desktop 갱신,
  취소, 정확한 검토·오래된 검토, Problem 해결·발행과 독립된 완료, EN/KO 추적 문구,
  권한 거부·회수와 desktop/Chat 동시 변경을 검증한다.
- 이 host에서 확인하지 못한 Windows/Linux 설치, OS reduced motion, 실제 provider corpus와
  host elicitation은 외부 검증 항목으로 명시한다. 사용자 데이터, keychain, TCC를 초기화하지 않는다.

확인된 모든 MCP 결함을 수정하거나 구체적인 호환 결정과 검증을 남겨야 완료다. commit을
작성할 때 저장소 commit 규칙을 따른다. 설치·push는 세션의 사용자 승인 범위와 검증된
artifact에 근거해야 하며 이 계획은 이미 수행했다고 주장하지 않는다.

## 단계별 실행 모델

| 단계 | 모델·강도 | 이유 |
| --- | --- | --- |
| DB·공통 변경 경로 구현 | GPT-5.6 Terra / high | SQLite FK 재구성, 권한·트랜잭션 CAS·발행 복구는 데이터 무결성에 영향을 준다. |
| 나머지 서비스·DTO·runtime 통합 | GPT-5.6 Terra / medium | 검토된 대응표를 여러 모듈에 통합한다. |
| 스킬·EN/KO 문서·기계적 metadata | GPT-5.6 Luna / low | 확정된 용어·계약 변경이며 독립 파일에 한정한다. |
| 동작 검증·릴리스 근거 | GPT-5.6 Terra / medium | 명시된 불변식으로 마이그레이션·동시성·packaged 동작을 검증한다. |

같은 파일은 동시에 수정하지 않는다. 의존 단계는 구현 에이전트를 재사용하고 일상 후속
작업은 강도를 낮춘다. 확인된 정확성 병목만 상향한다. Astra 자문은 완료했으므로 일상적인
구현·테스트를 Astra에게 계속 맡기지 않는다.

## 검토 기록

2026-09-12 GPT-6 Astra/high가 소스와 DB·실행 초안을 검토했다. 트리거 활성 여부와 버전
commit 사실을 수정하고 정확한 v9, scoped 이어가기, 일관된 읽기 snapshot, 공통 변경 경로,
Task Knowledge 권위와 불변 Lineage 결정을 반영하는 조건으로 승인했다. 해당 수정을 두
계획에 반영했다. 이는 설계 검토 근거이며 migration/runtime/release 검증 결과가 아니다.

구현 무결성 보완: 참조된 Task·Problem 대상의 범위 권한과 모든 하위 자료 스냅샷을 연결하고 승인 트랜잭션 안에서 다시 확인한다. 공개 이외의 통제된 변경도 같은 트랜잭션에서 적용해 실패 시 검토를 pending 상태로 보존한다. 공개는 파일 쓰기 전에 승인된 복구 작업을 저장하며, 완료된 operation 재시도는 추가 승인 없이 저장된 결과를 반환한다.
