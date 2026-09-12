# Codex·ChatGPT 데스크톱에서 Task 작업 이어가기

[English](mcp-workbench-bridge.md) | **한국어**

승인한 로컬 MCP 연결은 데스크톱 Workbench와 같은 Task 중심 작업 기록을 읽고 변경을 제안할 수 있습니다. 연결 범위와 topic 멤버십을 명시적으로 적용하며, Vault·앱 데이터베이스·자격 증명을 제한 없는 파일로 노출하지 않습니다.

## Task 제안과 검토

Chat은 생각을 Capture로 남기거나 Task 동작, checkpoint, 완료 또는 Knowledge 동작을 제안할 수 있습니다. 제안에는 Task와 기대 리비전이 들어갑니다. Problem 맥락이 필요하면 정확한 Problem 리비전을 선택적인 출처로 포함하며, Task에 Problem 부모가 필수는 아닙니다.

host는 정확한 제안을 검토 대상으로 보여 줍니다. 가능한 동작에 따라 사용자가 수락·거절·편집·보류하거나 다시 검토를 요청합니다. AI가 제안을 만들었다는 이유만으로 Task가 바뀌지는 않습니다. 오래됨·취소·만료·거절 상태의 제안도 현재 상태를 덮지 않고 감사 기록으로 남습니다.

## 한계와 연속성

Workbench, 인앱 Chat, 로컬 MCP는 같은 애플리케이션 경계를 사용하므로, 수락한 동작에는 같은 리비전 검사와 저장 규칙이 적용됩니다. 추적 중인 Chat 작업을 완료해도 Knowledge를 발행하지 않습니다. Knowledge 초안·발행은 각각 별도의 명시적 결정입니다.

현재 bridge는 지원하는 데스크톱 host를 위한 로컬 stdio 연동입니다. ChatGPT 웹, 원격 MCP, 자동 Task 선택, topic을 넘는 제한 없는 접근, AI 제안의 자동 승인은 약속하지 않습니다.

## 로컬 연결 설정하기

**AI 설정**의 **채팅 연결**에서 연결 이름을 입력하고 필요한 grant만 선택한 뒤, 필요하면 허용할 topic ID를 입력해 연결을 만듭니다. **허용된 접근 범위**을 펼치면 정확한 grant와 `llm-wiki-desktop --mcp --connection <id>` 형식의 로컬 명령을 확인할 수 있습니다. 지원하는 로컬 데스크톱 host의 stdio MCP 설정에 이 명령을 넣으세요. 실행 파일이 PATH에 없으면 설치한 앱의 실행 파일 전체 경로를 지정하세요. stdio bridge는 실행 중인 GUI로 요청을 전달하므로 앱을 열어 두어야 합니다. 앱이 없으면 요청을 차단합니다. 같은 화면에서 topic 멤버십을 관리하고, **연결 해제**로 연결을 즉시 비활성화할 수 있습니다.

[Task 중심 Workbench](conflict-gated-workflow.ko.md)와 현재 계약의 역사 맥락인 [명세 012](../../specs/012-task-centered-workbench/spec.md)를 참고하세요.
