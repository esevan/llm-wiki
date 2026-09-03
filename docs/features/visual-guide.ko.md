# 기능 둘러보기

[English](visual-guide.md) | **한국어**

아래 네 장의 주요 화면은 React·Tauri 전환이 `main`에 반영된 뒤 패키징한 네이티브 앱에서 다시
캡처했습니다. 문서용으로 격리한 Vault와 데이터베이스를 사용했으며 개인 Vault 콘텐츠나 자격 증명
값은 포함하지 않습니다. 각 화면은 하나의 제품 기능을 보여주고, 뒤의 세부 화면은 더 깊은 workflow
상태를 설명합니다.

![Tauri command로 Capture를 저장한 패키징 네이티브 Workbench](../images/native-app/workbench.jpg)

## 1. Capture, Problem, Solution

현재 Workbench 화면은 번들된 Noto Sans KR 폰트로 한국어 UI를 표시합니다. 샘플 Capture 저장은 typed
application client와 Tauri workflow command를 통과하며 loopback 웹 서버를 호출하지 않습니다.

![Capture, Problem, Solution, 진행 중 작업, 충돌 상태를 함께 보여주는 Workbench](images/02-workbench.png)

![Capture, Problem, Solution, 미해결 충돌 열을 보여주는 Workbench 보드](images/02-workbench-board.png)

Workbench는 주요 workflow를 한곳에 유지합니다. 카드는 현재 상태를 보존하며 승인이나 workflow
상태를 바꾸는 동작은 계속 명시적인 사용자 결정으로 남습니다.

관련 문서: [Workbench](conflict-gated-workflow.ko.md),
[언어 전환](bilingual-localization.ko.md),
[충돌 검토](vault-conflict-evidence.ko.md)

## 2. 맥락을 잃지 않고 탐색하기

![대화 옆에서 현재 Solution Detail을 보여주는 Explore 작업공간](images/05-refinement-preview.png)

Explore는 저장된 Detail, lineage 맥락, 대화를 하나의 작업공간에 엽니다. AI는 refinement를 준비할 수
있지만 사용자가 적용을 선택하기 전에는 제안을 반영하지 않습니다.

관련 문서: [Refinement Preview](refinement-preview-status.ko.md)

## 3. 지속 가능한 작업 기록과 함께 실행하기

![체크리스트, Work Log, 검토 댓글을 함께 보여주는 Solution Work 탭](images/06-work-log.png)

Work 탭은 검증 기준, 체크 상태, 진행 메모, 검토 댓글을 함께 보존합니다. 이 근거는 완료와 lineage
생성까지 이어집니다.

관련 문서: [완료와 Knowledge](completion-writeback-archive.ko.md),
[Lineage Knowledge Layer](lineage-knowledge-layer.ko.md)

## 4. 기존 Knowledge 다시 찾기

![번들된 다국어 모델을 사용하는 네이티브 의미 기반 Vault 검색](../images/native-app/vault-search.jpg)

Vault 검색은 로컬 Markdown 맥락을 반환합니다. 이 화면에서 선택한 의미 검색 결과는 `.app`에
포함된 다국어 임베딩 모델이 만들었으며 임베딩 서비스나 시작 시 다운로드가 필요하지 않습니다.
로컬 추론에 실패해도 lexical 검색은 계속 사용할 수 있습니다.

관련 문서: [빠른 Vault 검색](fast-vault-search.ko.md)

## 5. 방향을 계속 보이게 하기

![Tauri command로 방향 목표를 저장한 네이티브 Compass](../images/native-app/compass.jpg)

Compass는 개인 활동을 성과 점수로 바꾸지 않고 현재 방향을 기록합니다.

관련 문서: [Compass](direction-dashboard.ko.md)

## 6. 비밀 값을 노출하지 않고 AI 설정하기

![Endpoint, model routing, Worker 수, 마스킹된 자격 증명 상태를 보여주는 네이티브 AI 설정](../images/native-app/ai-settings.jpg)

AI 설정은 endpoint·model routing과 자격 증명을 분리합니다. 저장된 키는 native secret storage에
보관하며 그 값을 UI로 돌려주지 않습니다.

관련 문서: [백그라운드 AI Queue](background-ai-queue.ko.md)

## 7. 백그라운드 작업 확인하기

![읽을 수 있는 대상, 상태, 결과 목적지, 취소 동작을 보여주는 백그라운드 Queue](images/08-background-queue.png)

지속 작업은 목적과 대상이 이해 가능한 형태로 남습니다. Queue는 내부 Job JSON을 사용자 경험으로
노출하지 않고 취소, 재시도, 진행률, 결과 목적지를 제공합니다.

![충돌 검토가 Queue에 등록되었음을 알리는 확인 화면](images/09-background-job-queued.png)

지속 검토를 시작하면 Workbench를 막지 않고 작업이 백그라운드에서 이어짐을 알린 뒤 Queue로
안내합니다.

관련 문서: [백그라운드 AI Queue](background-ai-queue.ko.md),
[Vault 충돌 근거](vault-conflict-evidence.ko.md),
[충돌 해결 워크플로](conflict-resolution-workflow.ko.md)
