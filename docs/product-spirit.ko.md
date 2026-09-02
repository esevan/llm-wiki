# LLM Wiki에 녹아든 Product Spirit

[English](product-spirit.md) | **한국어**

Product Spirit은 모든 제품·개발 판단의 첫 번째 기준입니다. [Constitution](../.specify/memory/constitution.md)은
이 원칙들을 필수 리뷰 게이트로 구체화합니다.

## 1. You talk. The work organizes itself.

Capture는 자연스러운 생각을 그대로 받습니다. AI 대화와 Refinement가 구조를 찾아 의도를 보존하고
편집 가능한 제안을 만듭니다. 사용자는 처음부터 정리하는 대신 정리된 결과를 검토합니다.

## 2. Reduce cognitive load.

Capture는 의도적으로 작고 가볍습니다. Workbench에는 Capture·Problem·Solution만 보이며, In Progress
Solution은 별도 영역에서 강조됩니다. 상세·검증·완료 제어는 현재 결정에 필요할 때만 나타납니다.

![Capture는 가볍게 유지하고 현재 Solution을 우선 표시하는 Workbench](features/images/02-workbench.png)

## 3. Resume where you left off.

Solution Work Log는 텍스트·스크린샷·댓글·검증 체크를 지원합니다. Refinement Preview는 이전 결정,
근거, 제약, trade-off를 계속 보여줍니다. 충돌 검토는 현재 Solution을 검색 가능한 Knowledge와
비교해 사용자가 맥락을 다시 조립하지 않게 합니다.
전역 한국어·영어 설정은 현재 화면, 저장하지 않은 입력, workflow 계보를 버리지 않고 언어를
바꿉니다. 새로 생성하고 승인한 AI Problem·Solution은 두 언어 저장본을 유지하고, 기존 콘텐츠는
원문 그대로 계속 읽을 수 있습니다. 명시적으로 요청한 AI 이미지 요약도 기존 한 번의 요청에서 두
언어를 함께 저장하며, 사용자가 작성한 Work Log 근거는 원문을 유지합니다.

![최신 시각 상태와 검증 맥락을 보존하는 Solution Work Log](features/images/06-work-log.png)

![현재 대화 옆에 이전 맥락을 유지하는 Refinement Preview](features/images/05-refinement-preview.png)

## 4. Organize around problems, not tasks.

지속되는 흐름은 **Capture → Problem → Solution**입니다. Solution이 Work Log와 검증 체크리스트를
소유합니다. 실행 정보가 별도 계층으로 분리되지 않으므로 모든 행동은 왜 중요한지를 설명하는
Problem에 연결됩니다.

## 5. Private process, portable knowledge.

대화·초안·Refinement·진행 기록은 개인 로컬 과정으로 남습니다. 사람이 승인한 완료만 Obsidian 호환
Playbook과 원시 근거 묶음을 만듭니다. 이 Markdown은 LLM Wiki 없이도 유용하며 이후 충돌 검토의
Knowledge로 검색할 수 있습니다.
앱이 관리하는 Knowledge는 영문 Markdown을 휴대 가능한 canonical 원본으로 사용합니다. 한국어
열람본은 요청할 때 파생하며, 정확히 같은 현재 원본에 대해서만 재사용합니다. 한국어 열람본이
canonical 파일을 대체하거나 다시 쓰는 일은 없습니다.

![개인 근거 검토와 사람의 Knowledge 발행 결정을 분리하는 완료 화면](features/images/03-completion-archive.png)

## 6. Understand the work, never score the worker.

Compass는 목표·근거·마일스톤 이벤트·방향을 설명합니다. 이를 직원 점수, 생산성 순위, 개인 평가로
바꾸면 안 됩니다. 향후 조직 기능도 언어·데이터·권한·시각화에서 이 경계를 지켜야 합니다.
