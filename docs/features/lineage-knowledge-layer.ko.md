# Task 계보와 Knowledge

[English](lineage-knowledge-layer.md) | **한국어**

Task 상세는 Capture 출처, 선택적으로 연결한 정확한 Problem 리비전, Work Log 근거·댓글·체크리스트·결정·완료 근거·연결을 함께 보존합니다. 이 결정적 기록은 Task 완료 후 비공개 Knowledge 초안을 만드는 원본이며 자동 발행되지 않습니다.

![작업 근거와 출처를 함께 보존하는 Task 상세](images/task-detail.png)

초안을 명시적으로 만들고 검토·수정한 뒤 별도로 발행합니다. 초안 리비전과 콘텐츠 해시가 발행 원본을 식별합니다. 발행한 Vault 파일이 외부에서 바뀌면 해시 검사로 조용한 덮어쓰기를 막습니다. 재생성은 기록한 Task에서 초안을 갱신하며 Task의 비공개 기록을 지우지 않습니다.

이전 Solution 계보 탭, 추론 주장 교정 화면, 자동 완료 보고서는 보존한 레거시 동작·명세 기록입니다. 현재 Task 인터페이스가 제공한다고 약속하는 기능이 아닙니다.

[완료와 Knowledge](completion-writeback-archive.ko.md)와 [Task 중심 Workbench](conflict-gated-workflow.ko.md)를 참고하세요.
