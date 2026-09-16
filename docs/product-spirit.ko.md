# LLM Wiki의 제품 원칙

[English](product-spirit.md) | **한국어**

제품 원칙은 모든 제품·개발 판단의 첫 번째 기준입니다. [Constitution](../.specify/memory/constitution.md)은
이 원칙들을 필수 리뷰 게이트로 구체화합니다.


명시적인 Subtask 적용은 자식 완료마다 부모 Knowledge를 누적 갱신·자동 발행하는 흐름에 동의하는 동작입니다. 모든 자식이 완료되어야 부모도 닫힙니다. 정제는 Task ID를 보존하고, 분리한 작업에는 부모·자식 경계를 표시합니다. 독립 Task의 Knowledge는 별도 검토 후 발행합니다.

Capture 정제는 ID와 원문을 보존하며 같은 업무를 Task로 전환합니다. 원본은 별도의 받은 항목이 아닌 이력으로 남습니다. 기존 Task는 현재 ID와 Capture 연결을 유지합니다. 추가 업무는 명시적인 하위 작업 분리로만 만들며, 일반 정제는 현재 업무를 갱신합니다.

## 1. 말하면, 일이 스스로 정리됩니다.

Capture는 자연스러운 생각을 그대로 받습니다. AI 대화와 Refinement가 구조를 찾아 의도를 보존하고
편집 가능한 제안을 만듭니다. 사용자는 처음부터 정리하는 대신 정리된 결과를 검토합니다.

## 2. 인지 부담을 줄입니다.

Capture는 의도적으로 작고 가볍습니다. Workbench는 현재 Task를 보여주고 진행 중인 작업을 별도
강조합니다. 상세·검증·완료 제어는 현재 결정에 필요할 때만 나타납니다.

![Capture는 가볍게 유지하고 현재 Task를 바로 이어서 할 수 있게 하는 Workbench](features/images/workbench-tasks.png)

## 3. 멈춘 곳에서 다시 이어갑니다.

Task Work Log는 텍스트·스크린샷·댓글·검증 체크를 지원합니다. Refinement는 이전 결정,
근거, 제약, 선택의 장단점을 계속 보여줍니다. 충돌 검토는 현재 Task를 검색 가능한 Knowledge와
비교해 사용자가 맥락을 다시 조립하지 않게 합니다.
전역 한국어·영어 설정은 현재 화면과 작업 흐름 계보를 버리지 않고 시스템 언어를 바꿉니다. 생성
텍스트는 요청을 시작할 때의 언어로 만들며, 사용자가 작성한 Work Log 근거와 기존 콘텐츠는 원문을
유지합니다. 앱이 관리하는 Knowledge의 한국어 열람은 별도 명시 동작이며 영문 canonical Markdown을
바꾸지 않습니다.

![최신 Work Log와 검증 맥락을 보존하는 Task 상세](features/images/task-detail.png)

![현재 대화 옆에 이전 맥락을 유지하는 Refinement](features/images/refinement.png)

## 4. 해결하려는 문제를 중심으로 일을 정리합니다.

각 작업이 어떤 문제를 해결하는지, 왜 필요한지 맥락을 보존합니다.
Capture에서 이미 찾은 해결 방향은 보존합니다. 간단한 Task는 즉시 시작할 수 있고, 선택적인
Refinement는 검토 가능한 Problem과 여러 버전의 Task를 만들 수 있습니다. Task는 `task`(Task),
`in_progress`(진행 중), `completed`(완료) 상태를 독립적으로 가집니다. Work Log에는 텍스트·이미지·파일·댓글·체크리스트·
결정을 남기며, Task를 완료해도 Problem이 자동으로 해결되지는 않습니다.

## 5. 과정은 비공개로 유지하고, Knowledge는 자유롭게 옮깁니다.

대화·초안·Refinement·진행 기록·완료 결정은 개인 로컬 과정으로 남습니다. 독립 Task의 완료와 발행은 서로 다른
사용자 결정입니다. 사용자가 Knowledge 발행을 명시적으로 승인해야 Obsidian 호환 Playbook과 원시
근거 묶음을 만듭니다. 이 Markdown은 LLM Wiki 없이도 유용하며 이후 충돌 검토의
Knowledge로 검색할 수 있습니다.
앱이 관리하는 Knowledge는 영문 Markdown을 휴대 가능한 canonical 원본으로 사용합니다. 한국어
열람본은 요청할 때 파생하며, 정확히 같은 현재 원본에 대해서만 재사용합니다. 한국어 열람본이
canonical 파일을 대체하거나 다시 쓰는 일은 없습니다.

![개인 근거 검토와 Knowledge 발행 결정을 분리하는 완료 화면](features/images/completion-knowledge.png)

## 6. 작업을 이해하되, 사용자를 점수화하지 않습니다.

Compass는 목표·근거·마일스톤 이벤트·방향을 설명합니다. 이를 직원 점수, 생산성 순위, 개인 평가로
바꾸면 안 됩니다. 향후 조직 기능도 언어·데이터·권한·시각화에서 이 경계를 지켜야 합니다.
