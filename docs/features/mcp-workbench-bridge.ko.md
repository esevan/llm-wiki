# Codex·ChatGPT 데스크톱에서 Task 작업 이어가기

[English](mcp-workbench-bridge.md) | **한국어**

승인한 로컬 MCP 연결은 데스크톱 Workbench와 같은 Task 중심 작업 기록을 읽고 변경을 제안할 수 있습니다. 연결 범위와 topic 멤버십을 명시적으로 적용하며, Vault·앱 데이터베이스·자격 증명을 제한 없는 파일로 노출하지 않습니다.

## Task 제안과 검토

Chat은 생각을 Capture로 남기거나 Task 동작, checkpoint, 완료 또는 Knowledge 동작을 제안할 수 있습니다. 제안에는 Task와 기대 리비전이 들어갑니다. Problem 맥락이 필요하면 정확한 Problem 리비전을 선택적인 출처로 포함하며, Task에 Problem 부모가 필수는 아닙니다.

host는 정확한 제안을 검토 대상으로 보여 줍니다. 가능한 동작에 따라 사용자가 수락·거절·편집·보류하거나 다시 검토를 요청합니다. AI가 제안을 만들었다는 이유만으로 Task가 바뀌지는 않습니다. 오래됨·취소·만료·거절 상태의 제안도 현재 상태를 덮지 않고 감사 기록으로 남습니다.

## Task 직접 이어가기와 동기화

두 입력은 같은 애플리케이션 서비스, 내구성 이벤트 로그, workflow 레코드, Vault, 백그라운드
projector를 사용합니다. Chat에서 추적을 시작하면 먼저 서버가 검토용 미리보기를 만들고, 사용자가
그 정확한 내용을 승인해야 비공개 Capture와 추적 세션이 생깁니다. 사용자는 기존 Task를 직접 이어갈 수도 있습니다. 검토한 `inbound_work_open`에 `mode:"continue_task"`와 `taskId`를 보내 승인하면 Capture provenance 없이 connection 소유의 captureless session이 생기며, 취소하면 session은 생기지 않습니다. 사용자가
검토한 Task 제안과 정확한 Problem link 결정은 기존 Workbench 레코드에 반영되므로, 나중에 Workbench를 열어도
같은 진행 상태가 보이고 Workbench에서 바꾼 내용도 다음 Chat 조회에서 최신 상태로 읽힙니다.

승인된 Task 변경과 추적 이벤트, session head, 적용된 projection 결과는 같은 트랜잭션에서
커밋됩니다. 데스크톱 Task 변경도 연결된 세션을 작업당 한 번 갱신하며, 재시도는 이벤트를
중복 생성하지 않습니다. 다른 이벤트 projection은 백그라운드에서 잠시 대기 상태로 보일 수
있습니다. 의미 있는 체크포인트는 Chat에 확인을 반환하기 전에 이벤트 로그에 커밋됩니다. 스트림별 watermark는 애플리케이션
시각과 결정적 순서를 사용합니다. 늦게 도착한 이벤트는 감사 기록에는 남지만 현재 상태를 되돌리지
않습니다.

## 한계와 연속성

Workbench, 인앱 Chat, 로컬 MCP는 같은 애플리케이션 경계를 사용하므로, 수락한 동작에는 같은 리비전 검사와 저장 규칙이 적용됩니다. 추적 중인 Chat 작업을 완료해도 Knowledge를 발행하지 않습니다. Knowledge 초안·발행은 각각 별도의 명시적 결정입니다.

현재 bridge는 지원하는 데스크톱 host를 위한 로컬 stdio 연동입니다. ChatGPT 웹, 원격 MCP, 자동 Task 선택, topic을 넘는 제한 없는 접근, AI 제안의 자동 승인은 약속하지 않습니다.

## 로컬 연결 설정하기

**AI 설정**의 **채팅 연결**에서 연결 이름을 입력하고 필요한 grant만 선택한 뒤, 필요하면 허용할 topic ID를 입력해 연결을 만듭니다. **허용된 접근 범위**을 펼치면 정확한 grant와 `llm-wiki-desktop --mcp --connection <id>` 형식의 로컬 명령을 확인할 수 있습니다. 지원하는 로컬 데스크톱 host의 stdio MCP 설정에 이 명령을 넣으세요. 실행 파일이 PATH에 없으면 설치한 앱의 실행 파일 전체 경로를 지정하세요. stdio bridge는 실행 중인 GUI로 요청을 전달하므로 앱을 열어 두어야 합니다. 앱이 없으면 요청을 차단합니다. 같은 화면에서 topic 멤버십을 관리하고, **연결 해제**로 연결을 즉시 비활성화할 수 있습니다.

[Task 중심 Workbench](conflict-gated-workflow.ko.md)와 현재 계약의 역사 맥락인 [명세 012](../../specs/012-task-centered-workbench/spec.md)를 참고하세요.

## 사용자 검토와 충돌 확인

Task 전환, 정확한 Problem link/해결, 충돌 검토, 완료, 발행 전환은 정확한 사용자 검토가 필요합니다. 최신 MCP host에서는
연결·세션·원본 이벤트·payload·revision·만료·일회용 challenge에 결합된 다중 왕복 Elicitation을
사용합니다. 호출자가 보낸 `confirmed` 같은 필드는 권한이 될 수 없습니다.

충돌 판단은 현재 Chat을 담당하는 AI가 수행합니다. LLM Wiki는 범위가 제한된 lexical/semantic
검색과 revision을 검증한 근거 구절만 제공합니다. MCP server는 숨은 모델을 실행하지 않습니다.
semantic coverage가 준비되지 않았더라도 lexical 검색은 계속 사용할 수 있고, AI는 근거가 충분하지
않음을 밝혀야 합니다. 검색 결과의 불투명 evidence handle은 연결·scope에 묶여 10분 뒤 만료되며,
근거를 읽을 때 소유권과 원본 revision을 다시 확인합니다.

## 완료와 발행은 별도 단계

Task를 완료하면 비공개 Completed Work만 생기며 Knowledge 파일은 만들어지지 않습니다.
workflow 스킬은 이어서 한 번만 “Knowledge로 발행할까요?”라고 묻습니다. 동의하면 먼저 버전이
있는 비공개 초안을 만듭니다. 발행은 검토한 초안 ID, revision, content hash만 받는 두 번째 명시적
작업입니다. 외부에서 Markdown 파일이 바뀌었다면 충돌로 중단하고 덮어쓰지 않습니다.
발행 철회도 정확한 검토를 거칩니다. 파일은 색인되지 않는 로컬 복구 사본으로 이동하고 Completed
Work와 결정 이력은 그대로 유지됩니다.

Task Work Log, refinement, 관계, Problem link는 서로 별도 결정이며 conflict review는 자문 기능으로
Task 작업을 막지 않습니다. 새 Knowledge는 immutable lineage snapshot을 포함한 Task draft 흐름을
사용하고, 기존 `knowledge_drafts` record는 legacy history로 읽을 수 있습니다. 유지된 Knowledge
도구 alias도 canonical Task DTO와 review wrapper를 동일하게 사용합니다.
Canonical refinement, advisory, lineage, Knowledge 도구는 각각 `task_refinement_*`,
`task_advisory_*`, `task_context_read`, `task_lineage_read`, `task_knowledge_*` 이름을 사용합니다. Body/content hash와
source/lineage hash는 서로 바꾸어 사용할 수 없습니다.
Knowledge 수정·발행·철회에는 정확한 초안 `expectedContentHash`와 정확한 lineage
`expectedSourceHash`가 모두 필요하며, source hash가 없으면 추측하지 않고 거부합니다.
기존 초안도 저장된 source hash를 유지합니다. `task_context_read`는 필드마다 최대 4,000자와
배열 100개로 제한하고, binary attachment는 MCP로 보내지 않고 로컬에만 둡니다.

정확한 프로토콜은 [Task MCP 계약](../../specs/012-task-centered-workbench/contracts/mcp-task-contract.md)과
기존 [전송 계약](../../specs/011-mcp-workbench-bridge/contracts/mcp-server.md)을 참고하세요.
결과는 [보완 검증 기록](../testing/mcp-task-workflow-verification.ko.md)에 정리합니다.
