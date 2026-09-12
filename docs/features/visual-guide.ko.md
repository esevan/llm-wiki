# 현재 인터페이스 안내

[English](visual-guide.md) | **한국어**

이 안내의 이미지 9개는 2026-09-12에 서명한 macOS 패키지를 한국어 밝은 모드로 직접 캡처한 화면입니다. PNG 크기는 네이티브 제목 표시줄을 포함한 1198×768이며, WebKit 뷰포트 크기를 뜻하지 않습니다. 분리한 일회용 앱 데이터와 예시 Task·Work Log·완료 근거를 사용했습니다. Refinement와 충돌 검토에는 결정적인 로컬 테스트 제공자를 사용했으므로 외부 AI 품질을 입증하지 않습니다. Queue는 실제 API 키 누락 실패를, 완료 화면은 발행하지 않은 비공개 초안을 보여 줍니다. [영문 안내](visual-guide.md)에는 같은 패키지에서 캡처한 별도 영어 로캘 자산을 사용합니다.

캡처 출처: 기준 커밋 `de01ff47398871902a765d43b5a4060161316f92`, 브랜치 `fix/ui-ux-improvements`의 미커밋 소스 빌드입니다. 11:44:31 AM에 `LLM Wiki Local Signing`으로 서명했으며 CDHash는 `9b2b81082a43de0637bedd33a8cc670709ff2b21`입니다. 같은 패키지에서 데스크톱 시나리오 32개와 등록된 컨트롤 175개가 모두 통과했습니다. 근거와 제한은 [릴리스 검증](../../specs/012-task-centered-workbench/acceptance-verification.md)을 참고하세요.

## Workbench와 Task 상세

Workbench의 가벼운 입력에서 텍스트를 Capture로 저장하거나 Task를 바로 만듭니다. 저장한 작업과 재개할 수 있는 Refinement 바로가기는 그 아래에서 찾습니다. Problem은 Task에 연결할 수 있는 선택적인 맥락이며, 반드시 거쳐야 하는 단계가 아닙니다.

![Capture·Task 입력과 저장한 작업을 보여 주는 Workbench](images/workbench-tasks.png)

Task를 열면 정의를 수정하고 Work Log 근거·댓글·체크리스트·결정·연결·완료 근거·Knowledge 동작을 함께 확인합니다. Task 완료, 연결한 Problem 해결, Knowledge 발행은 각각 사용자가 별도로 결정합니다.

![작업 근거와 결정을 보여 주는 Task 상세](images/task-detail.png)

## Refinement와 완료

Refinement는 원래 맥락·대화·메모·검토할 제안을 함께 유지합니다. 저장한 작업 공간은 미완료 정제를 이어서 하게 하며, 제공자 또는 저장 오류는 완료로 처리하지 않고 재시도할 수 있도록 화면에 남깁니다.

![맥락과 대화를 함께 보여 주는 Refinement 작업 공간](images/refinement.png)

충돌 검토는 참고용이며 시도 이력을 유지합니다. 아래 예시는 근거 부족 상태입니다. 인용이 있는 발견 결과나 완료·발행의 허가를 뜻하지 않습니다.

![근거 부족·재시도·시도 이력을 보여 주는 Task 충돌 검토](images/task-review.png)

Task에 완료 근거를 남긴 뒤 비공개 Knowledge 초안을 만들고 검토합니다. 발행은 검토한 Markdown을 Vault에 쓰는 별도의 사용자 동작입니다.

![완료 근거와 별도 Knowledge 발행을 보여 주는 화면](images/completion-knowledge.png)

## Queue·검색·Compass·AI 설정

백그라운드 Queue는 지속 작업과 복구 동작을 식별합니다. 실패하거나 대기 중인 AI 요청을 완료된 Task 결정으로 바꾸지 않습니다.

![복구 경로가 있는 Queue 실패 화면](images/queue-recovery.png)

검색은 선택한 Vault를 읽고, Compass는 사람을 점수화하지 않고 방향을 기록합니다. AI 설정은 자격 증명 값을 가린 채 연결과 모델 라우팅 설정을 표시합니다.

![경로와 일치 맥락을 보여 주는 Vault 검색 결과](images/vault-search.png)

![방향과 근거를 기록하는 Compass](images/compass.png)

![연결과 라우팅 설정을 보여 주는 AI 설정](images/ai-settings.png)

동작과 검증 경계는 [Task 중심 Workbench](conflict-gated-workflow.ko.md), [완료와 Knowledge](completion-writeback-archive.ko.md), [백그라운드 AI Queue](background-ai-queue.ko.md)를 참고하세요.
