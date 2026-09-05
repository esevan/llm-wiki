# Codex·ChatGPT 데스크톱에서 작업 추적하기

**한국어** | [English](mcp-workbench-bridge.md)

LLM Wiki는 Workbench 화면을 열지 않아도 인앱 Chat이나 로컬 MCP Chat에서 작업을 추적합니다.
1차 릴리스는 Codex와 ChatGPT 데스크톱을 지원하며, ChatGPT 웹과 원격 MCP 전송은 지원하지
않습니다.

## 하나의 workflow와 두 Chat 입력

두 입력은 같은 애플리케이션 서비스, 내구성 이벤트 로그, workflow 레코드, Vault, 백그라운드
projector를 사용합니다. Chat에서 추적을 시작하면 먼저 서버가 검토용 미리보기를 만들고, 사용자가
그 정확한 내용을 승인해야 비공개 Capture와 추적 세션이 생깁니다. 사용자가
검토한 Problem·Solution 제안은 기존 Workbench 레코드에 반영되므로, 나중에 Workbench를 열어도
같은 진행 상태가 보이고 Workbench에서 바꾼 내용도 다음 Chat 조회에서 최신 상태로 읽힙니다.

의미 있는 체크포인트는 Chat에 확인을 반환하기 전에 이벤트 로그에 커밋됩니다. Workbench 반영은
백그라운드에서 진행되어 잠시 대기 상태로 보일 수 있습니다. 스트림별 watermark는 애플리케이션
시각과 결정적 순서를 사용합니다. 늦게 도착한 이벤트는 감사 기록에는 남지만 현재 상태를 되돌리지
않습니다.

## 데스크톱 Chat 연결

**AI setup → Chat 연결**에서 이름을 지정해 연결을 만들고 권한 범위를 확인한 뒤, 표시된 연결 ID로
로컬 호스트를 설정합니다.

```text
llm-wiki-desktop --mcp --connection <connection-id>
```

번들 플러그인은 `LLM_WIKI_CONNECTION_ID` 환경 변수도 읽습니다. 데스크톱 호스트는 얇은 bridge와
stdio로 MCP를 주고받습니다. bridge는 macOS/Linux의 사용자 전용 Unix domain socket 또는 Windows의
named pipe를 통해 실행 중인 LLM Wiki GUI 프로세스로 전달합니다. GUI 프로세스만 애플리케이션
서비스, SQLite, Vault adapter, projector를 소유합니다. MCP를 사용하기 전에 LLM Wiki를 실행해야
합니다. 설정에서 연결을 즉시 폐기할 수 있으며, 세션 ID 자체는 자격 증명이 아닙니다.

권한은 세션 읽기·쓰기, topic, 현재/전체 Workbench 요약, lexical 검색, semantic 검색, 근거 읽기,
Knowledge 초안, Knowledge 발행으로 분리됩니다. topic 접근은 연결을 만들 때 선택한 topic ID로 한 번
더 제한됩니다.
토픽 소속은 AI setup에서 명시적으로 관리합니다. 제목이나 문서에 같은 단어가 있다는 이유만으로
자동 포함하지 않으며, 소속을 제거하면 해당 항목에 발급했던 근거 접근 권한도 해제됩니다.
일반적인 이어하기는 현재 세션만 읽고, 사용자가
명시적으로 요청할 때만 전체 Workbench를 가져옵니다.

## 사용자 검토와 충돌 확인

Problem, Solution, 충돌, 완료, 발행 전환은 정확한 사용자 검토가 필요합니다. 최신 MCP host에서는
연결·세션·원본 이벤트·payload·revision·만료·일회용 challenge에 결합된 다중 왕복 Elicitation을
사용합니다. 호출자가 보낸 `confirmed` 같은 필드는 권한이 될 수 없습니다.

충돌 판단은 현재 Chat을 담당하는 AI가 수행합니다. LLM Wiki는 범위가 제한된 lexical/semantic
검색과 revision을 검증한 근거 구절만 제공합니다. MCP server는 숨은 모델을 실행하지 않습니다.
semantic coverage가 준비되지 않았더라도 lexical 검색은 계속 사용할 수 있고, AI는 근거가 충분하지
않음을 밝혀야 합니다. 검색 결과의 불투명 evidence handle은 연결·scope에 묶여 10분 뒤 만료되며,
근거를 읽을 때 소유권과 원본 revision을 다시 확인합니다.

## 완료와 발행은 별도 단계

추적 세션을 완료하면 비공개 Completed Work만 생기며 Knowledge 파일은 만들어지지 않습니다.
workflow 스킬은 이어서 한 번만 “Knowledge로 발행할까요?”라고 묻습니다. 동의하면 먼저 버전이
있는 비공개 초안을 만듭니다. 발행은 검토한 초안 ID, revision, content hash만 받는 두 번째 명시적
작업입니다. 외부에서 Markdown 파일이 바뀌었다면 충돌로 중단하고 덮어쓰지 않습니다.
발행 철회도 정확한 검토를 거칩니다. 파일은 색인되지 않는 로컬 복구 사본으로 이동하고 Completed
Work와 결정 이력은 그대로 유지됩니다.

정확한 프로토콜은 [기능 명세](../../specs/011-mcp-workbench-bridge/spec.md)와
[MCP 계약](../../specs/011-mcp-workbench-bridge/contracts/mcp-server.md)을 참고하세요.
