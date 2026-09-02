# Lineage Knowledge Layer

[English](lineage-knowledge-layer.md) | **한국어**

> 완료된 Solution은 출발점, 결정, 충돌, 완료 근거를 추적 가능한 Knowledge로 남깁니다.

Solution을 완료하면 LLM Wiki가 현재 **Capture → Problem → Solution → Complete**
스냅샷을 자동으로 만듭니다. 완료 보고서는 이 스냅샷을 먼저 만든 뒤, Lineage 투영과 거기서
참조한 근거만 입력으로 사용해 생성합니다. 최종 Markdown에는 Detail, Lineage, Decision Changes,
Conflicts & Addresses, Completion Evidence가 함께 들어갑니다.

DB UUID는 비공개 Lineage 감사 모델 안에만 유지합니다. 완료 보고서 모델과 최종 Markdown에는
`Original capture`, `Problem record`, `Work log 1`, `Validation criterion 2`, `Completion decision`처럼
사람이 이해할 수 있는 결정적 라벨만 전달합니다. 비공개 추론 검증 요청은 추론 claim이 정확한 근거
묶음 밖을 인용하면 거부할 수 있도록 opaque evidence ID를 계속 사용하지만, 이 ID를 독자용 인용으로
재사용하지 않습니다.

Lineage와 완료 문서는 하나의 lifecycle을 공유합니다. 완료 Knowledge를 재생성하면 현재 원 기록에서
deterministic Lineage를 다시 만들고, 선택적으로 AI 해석을 갱신한 다음 보고서와 문서를 다시 만듭니다.
이 재생성을 문서 버전으로 노출하지 않으며, 포괄적인 Version Control은 별도 향후 기능으로 둡니다.

Lineage 탭은 네 단계를 좌우 페이지 스크롤이 필요 없는 세로 흐름으로 보여줍니다. Capture, Problem,
Solution 카드는 각각 읽기 전용 원 기록을 열며, 각 단계의 기록 시각은 사용자의 시스템 locale로
표시됩니다. 근거는 `[1]`, `[2]` 같은 Lineage 전체 고유 **Reference** 번호로 인용합니다. 같은 원 기록을
여러 곳에서 인용하면 같은 번호를 재사용하며, 번호를 선택하면 해당 카드나 전환 바로 아래에서 원문
발췌를 확인할 수 있습니다.

전환은 명시적인 **Decision basis**와 deterministic한 **Recorded change**를 구분합니다. 판단 이유가
기록되지 않았으면 의도를 만들어내거나 빈 이유를 보여주지 않고 단계 사이에서 관찰되는 변화를
설명합니다.

모든 서술에는 출처 성격이 표시됩니다.

- **Observed**는 보존된 원 기록에서 직접 가져온 내용입니다.
- **Decided**는 기록된 사람의 결정이나 실제 워크플로 결정입니다.
- **AI inferred**는 High, Medium, Low 신뢰도를 함께 표시하는 AI 해석입니다.
- **Corrected**는 기존 AI 해석에 대한 현재 사용자 교정입니다.

기록되지 않은 이유는 `Not explicitly recorded`로 남습니다. AI 추론만으로 Conflict를 Addressed로
바꿀 수 없습니다. Addressed에는 명시적 결정 또는 구현 근거가 필요하며, 원 요구가 Preserved,
Modified, Superseded, Rejected 중 어떻게 다뤄졌는지도 기록합니다.

사용자는 Lineage 탭에서 inferred claim을 교정할 수 있습니다. 교정은 현재 Knowledge가 되지만 이전
AI 해석과 변경 불가능한 원 근거는 revision history에 보존됩니다. 같은 claim을 재생성하면 교정 내용도
이어집니다.

AI Setup에는 **Lineage interpretation**이 독립 작업으로 표시됩니다. Advanced model이 설정되어 있으면
기본적으로 이를 사용하며 **Completion report** 작업과 별도로 라우팅됩니다. AI를 사용할 수 없어도
deterministic Lineage는 유지됩니다.

관련 Spec Kit: [010 — Lineage Knowledge Layer](../../specs/010-lineage-knowledge-layer/spec.md)
