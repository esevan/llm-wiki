# UI/UX 개선 구현 계획

> **실행 상태 — 구현·패키지 E2E·문서 캡처 완료.** 초안 보호, 창 크기별 배치, 선택 언어 문구,
> 포커스/IME 조합 보호, Queue 복구를 구현했습니다. 실제 한국어 IME·VoiceOver·OS 동작 줄이기는
> 수동 검증을 남겼습니다.

[English](ui-ux-improvements.md) | **한국어**

> 상태: **후속 구현·자동 릴리스 검증 완료.** 구현 기준은 `fix/ui-ux-improvements`의
> `de01ff47398871902a765d43b5a4060161316f92`입니다.

## 근거와 범위

**최종 패키지 결과:** 서명한 `9b2b81082a43de0637bedd33a8cc670709ff2b21` bundle은 packaged scenario 32/32를 통과했습니다. 생성한 inventory는 scanned source control 188개와 rendered/exercised/asserted control 각각 175개를 기록했고 6개 gap array는 모두 비었습니다. 실제 한국어 IME, VoiceOver, OS reduced motion, Windows, zoom, native quit/crash draft 내구성은 미검증 또는 범위 밖입니다.

2026-09-12 오전 11:44:31에 `LLM Wiki Local Signing`으로 만든 signed macOS release bundle의 CDHash는 `9b2b81082a43de0637bedd33a8cc670709ff2b21`입니다. 이는 위 기준과 branch로 식별하는 dirty worktree build이며 최종 commit이 아닙니다. unit/runtime, typecheck, scoped ESLint, native test, build 검사는 통과했고, 전체 packaged E2E 32/32와 컨트롤 175개 검증은 통과했습니다. 실제 한국어 IME·VoiceOver 수동 검토는 미검증입니다.

계획 대상은 Task detail/Workbench/Refinement/AI setup/Queue의 점진적 개선이다. 밝은 desktop 지원 범위에서만 검토하며, 전면 정보 구조 재설계, 새 UI 라이브러리, dark theme, 모바일, 검증되지 않은 autosave, Task draft용 새 durable schema는 이 작업의 기본 범위가 아니다.

## 구현 시작 조건과 파일 경계

- [x] **T0 · 구현 worktree 준비 (P1, small).** Primary checkout에서 `git status --short`와 시작 HEAD를 기록하고, 현재 사용자 변경을 보존한다. `scripts/create_task_worktree.sh ui-ux-improvements fix/ui-ux-improvements HEAD`로 `<repo>/.worktrees/ui-ux-improvements`를 만든다. primary checkout에서 실행하고, helper의 기본값 main 대신 확인한 HEAD를 세 번째 인자로 지정한다. 다른 기준점이 필요하면 그 commit/ref를 먼저 확정해 전달한다. 기존 사용자 변경은 임의로 복사·정리·되돌리지 않는다. 공유 `node_modules`/Cargo `target`/검증된 embedding assets와 복사된 `dist/`를 build cache로만 사용하며, linked dependency 상태에서 `npm ci`를 실행하지 않는다.
- [x] **T1 · 현재 계약 재확인 (P1, small).** worktree에서 `frontend/src/services/taskClient.ts`, `frontend/src/types/taskWorkbench.ts`, Task API/IPC와 revision-conflict 응답을 확인한다. `taskRevision`을 받는 `revise`, `workLog`, checklist/decision/transition/complete 등의 expected-revision 계약을 유지한다. 계획 중 새 API나 persistence schema를 가정하지 않는다.
- [ ] **T2 · 네이티브 창 종료 지원 확인 (선택적 조사).** React Task panel의 close와 Task switch는 T2 결과를 기다리지 않고 구현한다. native app close/quit은 현재 Tauri window close interception, async save, cancel/keep-open이 실제로 가능한지 먼저 조사하고 작은 spike 또는 native test로 입증될 때만 포함한다. 불가능하거나 안전하게 보장할 수 없으면 앱 종료 guard는 후속 issue로 남기며, crash/restart에서 draft가 보존된다고 주장하지 않는다.

예상 변경 경계는 `frontend/src/features/workbench/TaskDetail.tsx`, `WorkbenchView.tsx`, `RefinementPanel.tsx`, `task-workbench.css`, `taskWorkbenchText.ts`, `frontend/src/features/settings/SettingsView.tsx`, Queue runtime/overlay의 현재 소유 파일, 그리고 대응 React/runtime/desktop scenario 및 interaction coverage manifest다. native close guard가 검증되어 채택될 때만 `src-tauri`와 Rust test가 추가된다. 완료 후 실제로 바뀐 사용자 동작에 맞춰 `docs/features/conflict-gated-workflow(.ko).md`, `background-ai-queue(.ko).md`, `visual-guide(.ko).md` 및 coverage 문서를 검토·갱신한다. 현재 coverage 문서의 혼재된 artifact identity는 실제 최종 implementation evidence가 생길 때만 바로잡는다.

## 우선순위와 issue 매핑

## 구현 상태 (2026-09-12)

| 구현 범위 | 상태 |
| --- | --- |
| I1.1–I1.4 Task 정의 draft, 독립 mutation 병합, expected-revision 저장, close/switch guard | 구현 및 focused test 완료 |
| I2.1–I2.2 shortcut layout과 title-input 폭 | 구현 완료, focused native 기하·full E2E 통과 |
| I3.1–I3.3 선택 locale 문구와 assertion | 구현 및 focused test 완료 |
| I4.1–I4.3 Refinement 간격, 비모달 focus/Escape, IME 조합 안전성 | 구현 완료, 실제 IME·VoiceOver 수동 검토 대기 |
| I5.1–I5.2 Queue의 AI 설정 열기와 실패 작업 복귀/retry | 구현 및 focused test 완료 |
| I4.4의 공통 왼쪽 읽기축 heading/count, I6의 12px 이상 status/revision metadata | 구현 완료, 폭넓은 detail regrouping은 범위 밖 |
| T2 native quit/crash guard | 보류 |

아래 checkbox 목록은 원래 계획의 기록입니다. 위 상태 표와 [검증 근거](../testing/evidence/ui-ux-improvements.json)가 최종 구현 상태를 나타냅니다.

| 단계 | 실행 순서 | 완료 판단 |
| --- | --- | --- |
| 1. 재현된 결함과 낮은 비용의 수정 | I1 → I2 → I3 → I4.1 | 편집 소실·카드 겹침 회귀 검증, 제목 폭·선택 언어·발화자 간격 확인 |
| 2. 공통 상호작용과 복구 | I4.2–I4.3 → I5 | 비모달 포커스·안전한 닫기·설정 복귀/재시도, 새 컨트롤의 의미 있는 assertion |
| 3. 읽기 공간과 정보 계층 | I4.4 → I6 | 장문 fixture에서 먼저 검토 후 국소 배치/크기 조정. 필수 결함 수정의 완료를 막지 않음 |

각 단계는 국소 검증으로 진행하고, 아래의 릴리스 빌드와 전체 packaged E2E는 통합된 최종 변경 후 한 번 수행한다. 선택적인 native 종료 조사 T2는 단계 1의 필수 수정 선행 조건이 아니다.

| 구현 묶음 | 감사 issue | 우선순위 | 크기 가정 | 선행 |
| --- | --- | --- | --- | --- |
| I1 Task draft 보호 | UX-001 | P1 | 중간 | T0, T1; native 종료만 T2 |
| I2 좁은 창과 title 폭 | UX-012, UX-002 | P2 | small–medium | T0 |
| I3 locale/copy | UX-003, UX-006 및 copy inventory | P2 | small | T0 |
| I4 Refinement와 detail 읽기 축 | UX-004, UX-005, UX-008 | P2 | small–medium | T0 |
| I5 Queue 복구 | UX-007 | P2 | small–medium | T0, 현재 runtime ownership 확인 |
| I6 핵심 메타데이터 | UX-011 | P3 | 작음 | I4.4, 대표 화면 검토 |

크기는 구현 전제이며 날짜·사용 빈도 추정이 아니다. I1을 먼저 끝내고, 나머지는 공통 텍스트/레이아웃 변경을 같은 reviewable change 안에서 묶되 서로의 acceptance를 대체하지 않는다.

## I1 — Task 수정 draft와 persisted snapshot 분리

- [x] **I1.1 · 상태 모델 (P1).** `TaskDetail`의 단일 `task` state를 (a) 마지막 서버 `persisted` snapshot, (b) 사용자가 편집하는 `draft`, (c) draft가 처음 생성될 때 캡처한 `baseTaskRevision`과 비교용 `baseSnapshot`, (d) 필드별 dirty 집합으로 분리한다. title/detail/outcome/scope/nonGoals/validationCriteria만 draft 대상이다. 편집 전 snapshot의 revision을 base로 보관한다.
- [x] **I1.2 · 독립 mutation 병합 (P1).** Work Log, comment, checklist, decision, relationship, readiness, transition, completion 등은 최신 persisted snapshot을 갱신한다. 해당 mutation이 Task revision을 증가시켜도 pending draft의 base revision을 조용히 새 revision으로 바꾸지 않는다. 새 snapshot과 `baseSnapshot`을 비교해 draft의 dirty field에 서버 변경이 없음을 확인한 경우에만 안전하게 rebase한다. dirty 값은 보존하고 수정하지 않은 필드는 최신 서버 값으로 갱신하며, 비교를 통과한 snapshot과 revision을 새 저장 기준으로 함께 갱신한다. 단순히 mutation이 성공했다는 이유로 base를 이동하지 않는다. 겹치거나 비교가 불확실하면 draft는 그대로 유지하고 충돌/재검토 상태를 보여 사용자가 최신값을 보고 다시 저장하거나 버리게 한다.
- [x] **I1.3 · 명시적 저장과 optimistic conflict (P1).** Save changes는 최초 또는 위의 검증된 disjoint rebase로 확정된 `baseTaskRevision`을 `expectedTaskRevision`으로 `revise`에 보낸다. 성공하면 반환된 snapshot을 새 persisted/draft 기준으로 삼고 dirty를 지운다. stale/conflict 또는 네트워크 실패에서는 입력과 base를 유지하고 저장 실패 이유와 재시도/비교 가능한 다음 행동을 표시한다. mutation 결과를 재조회하는 기존 queueing은 보존하되, 재조회가 dirty draft를 덮지 않게 한다.
- [x] **I1.4 · close/switch guard (P1).** close 버튼과 Workbench의 다른 Task 선택은 dirty draft일 때 Save, Discard, Keep editing을 제공한다. Save 실패 또는 conflict면 panel/switch를 계속 막고 draft와 오류를 남긴다. Discard만 draft를 버리고, Keep editing은 현재 panel을 유지한다. 새 Task 선택은 guard 승인 뒤에만 전환한다. 저장 성공 뒤에는 의도한 close/switch를 계속한다.
- [ ] **I1.5 · native 종료의 제한 (P1).** T2가 실제 지원을 확인한 경우에만 같은 guard를 native close/quit event에 연결하고, Save failure에서 종료를 취소할 수 있음을 test한다. 지원이 확인되지 않으면 Task panel close와 Task switch만 완료 조건으로 문서화하며, 앱 quit/crash 보호, background flush, autosave, 새 DB schema는 이 구현에서 제외한다.

완료 조건: 저장하지 않은 Task 정의를 편집한 뒤 Work Log를 추가하거나 다른 독립 mutation/refresh를 수행해도 모든 draft 값이 남는다. 명시적 save는 exact base revision으로 충돌을 감지하며, 다른 변경을 덮어쓰지 않는다. close/switch의 선택과 save failure가 안전하다.

## I2 — 좁은 Workbench와 입력 폭

- [x] **I2.1 · shortcut geometry (P2).** `WorkbenchView.tsx`의 full Capture/Task title과 `task-workbench.css` shortcut grid/card를 content-driven height로 조정한다. action은 카드 안의 하단 영역에 남고, 다음 `All work` heading과 겹치지 않아야 한다. 904px 문제를 고정 높이 탓으로 단정하지 말고 WebKit cascade와 intrinsic sizing을 재현해 최소 CSS를 고른다. title clamp를 선택한다면 detail로 전체 title에 키보드 접근 가능한 계약도 추가한다.
- [x] **I2.2 · Task title input (P2).** checkbox 전용 selector만 별도 폭 규칙에 남기고 Task detail의 text input을 다른 편집 필드와 같은 가용 폭으로 만든다. 긴 title은 input의 수평 스크롤 끝까지 키보드로 편집 가능해야 하며, 라벨·저장 버튼을 밀거나 overflow시키지 않아야 한다.

완료 조건: 900×640, 904×768, 1280×820에서 긴 KO/EN title의 shortcut action과 다음 heading이 겹치지 않는다. title input의 CSS/DOM geometry가 panel content width를 합리적으로 사용하며, long-value end editing을 확인한다. 모든 긴 text가 input box 안에 동시에 보인다는 불가능한 assertion은 쓰지 않는다.

## I3 — 선택 locale에 맞는 system copy

- [x] **I3.1 · source-of-truth inventory (P2).** 현재 locale source와 React/legacy runtime copy 경계를 조사해, 사용자 작성 콘텐츠를 제외한 AI setup privacy, prerequisite, readiness reason/outcome, completion CTA, Queue failure/retry/setup copy의 소유 key를 목록화한다.
- [x] **I3.2 · i18n 적용 (P2).** AI setup의 hardcoded privacy sentence를 shared locale key로 옮긴다. KO: `API 키는 이 기기의 로컬 설정 파일에만 저장됩니다. Vault나 앱 데이터베이스에는 저장되지 않습니다.` EN은 현재 의미를 유지한다. `Complete Task`는 실제 행동과 일치하도록 EN `Add completion evidence`, KO `완료 근거 추가`로 visible label과 accessible name을 같은 key에서 제공한다. prerequisite/reason outcome은 목적이 드러나는 locale 문자열로 정리한다.
- [x] **I3.3 · copy assertions (P2).** KO/EN 전환과 relaunch fixture에서 해당 system copy를 exact assertion한다. 번역되지 않은 사용자 콘텐츠나 모든 legacy copy의 일괄 교체를 완료로 주장하지 않는다.

## I4 — Refinement, 비모달 focus, detail 읽기 축

- [x] **I4.1 · role/body spacing (P2).** `RefinementPanel` message markup/CSS에서 role과 body를 block 또는 명시적 gap으로 나누고 장문·code/image fixture에도 heading hierarchy와 scroll을 유지한다.
- [x] **I4.2 · non-modal focus contract (P2).** Task detail/Refinement는 배경 비교와 병행 Work Log를 유지해야 하므로 modal/focus trap으로 바꾸지 않는다. open 시 panel heading 또는 compose control로 programmatic focus를 옮기고, `Escape`는 panel을 닫으며 trigger로 focus를 되돌린다. close 후에는 original trigger가 여전히 존재하면 그 trigger, 아니면 Workbench의 안정적인 heading으로 복귀한다. Tab은 비모달 순서를 따르며 trap하지 않는다.
- [ ] **I4.3 · save-failure safety와 IME (P2).** Refinement workspace flush/save가 실패할 때 close가 무음으로 입력을 잃지 않도록 기존 retry/keep-open 경로를 점검한다. Cmd/Ctrl+Enter는 React 이벤트의 `event.nativeEvent.isComposing`(실제 환경에서 필요성이 확인된 호환 조건 포함) 중에는 send하지 않는다. 실제 KO IME와 VoiceOver는 deterministic E2E가 대신할 수 없으므로 manual pass를 추가한다.
- [ ] **I4.4 · 국소 detail grouping (P2).** 전체 redraw 없이 `Task details`/현재 작업 기록/완료 근거/metadata의 섹션 order와 heading/count를 조정한다. Workbench heading·count·status는 해당 card group의 왼쪽 읽기 축에 둔다. evidence 뒤에 metadata를 배치하고, Refinement의 비교 공간은 먼저 간격을 고친 뒤 필요성이 확인될 때만 기존 panel을 넓힌다. 전면 detail redesign은 하지 않는다.

## I5 — Queue provider-missing 복구

- [x] **I5.1 · ownership과 navigation 확인 (P2).** Queue가 legacy runtime/Overlay에서 render되는 현재 계약과 AI setup navigation dispatch를 확인한다. `Open AI setup`은 새 설정 화면/라우터를 만들지 않고 existing sidebar/app navigation을 호출한다.
- [x] **I5.2 · 실패→설정→복귀→retry (P2).** API key 미설정 job에서 locale별 원인과 다음 행동을 표시한다: EN `No API key is configured. Save your connection details in AI setup, then retry.` / KO `API 키가 설정되지 않았습니다. AI 설정에서 연결 정보를 저장한 뒤 다시 시도하세요.` 설정을 열어도 원래 failed job과 Queue context를 잃지 않으며, 설정 성공 후 기존 Retry가 같은 job에 다시 적용된다. retry 중에는 그 job의 중복 retry만 막고 실패/성공 상태를 갱신한다.

## I6 — 핵심 메타데이터 가독성

- [x] **I6.1 · 역할별 크기 검토 (P3, 작음).** UX-011의 현재 상태·revision과 provenance를 구분하고, 현재 판단에 필요한 메타데이터는 12px 이상의 후보를 실제 KO/EN 화면에서 비교한다. 기존 body/mono 토큰을 활용하고 모든 텍스트를 같은 크기로 바꾸지 않는다. 카드 높이·줄바꿈·확대에서 정보 손실이 없는 안만 적용한다.

## 검증 계획

- [ ] React/unit/runtime: I1의 draft merge/disjoint rebase/conflict/save failure/close-switch guard, I3 text keys, I4 focus/Escape/IME, I5 navigation/retry 상태를 component 또는 runtime test로 검증한다.
- [ ] Desktop scenario와 interaction coverage: 새 enabled guard actions, setup CTA, close control의 manifest entry·scenario registry·source evidence를 함께 갱신한다. 최종 control total은 baseline 169보다 늘 수 있으며, 32/169를 고정 acceptance로 사용하지 않는다.
- [ ] Geometry/copy: 900×640, 904×768, 1280×820에서 KO/EN long-title screenshot 및 DOM geometry를 수집한다. `scrollWidth/clientWidth`, card bounds, action/heading non-overlap, input available width와 horizontal scroll end를 검증한다. locale exact copy assertions를 별도로 둔다.
- [ ] Accessibility/manual: keyboard open→focus, Tab non-trap, Escape→focus restore; real Korean IME composition; VoiceOver focus/name; long text/code/image scrolling을 macOS에서 수동 기록한다. Windows 및 dark/mobile은 지원 claim에 추가하지 않는다.
- [x] Incremental checks: 변경 영역에 맞춰 `npm test`, `npm run typecheck`, `npm run lint`를 실행한다. native가 실제 변경된 경우에만 `cargo test --manifest-path src-tauri/Cargo.toml`을 실행한다. 모든 코드·문서 변경 후에만 `npm run tauri:build`와 `npm run test:desktop -- --full`을 각각 한 번 실행하고 `git diff --check`, changed-link, 한·영 문서 대응, worktree status를 확인한다.

## 문서와 인계

기준 커밋 `de01ff47398871902a765d43b5a4060161316f92`에서 만든 `.worktrees/ui-ux-improvements`의
`fix/ui-ux-improvements` 브랜치에서 구현했습니다. Primary checkout의 사용자 변경은 보존했습니다.
`docs/DOCUMENTATION_GUIDE.md`를 검토하고 한글·영문 사용자 안내, 기여 안내, 내비게이션,
작업 흐름과 스펙의 현재 기준을 갱신했습니다. 실제 화면 9개로 예전 이미지 16개를 교체했습니다.
검토한 Markdown 187개와 이미지 참조 46개의 로컬 링크가 모두 유효합니다.
과거 Problem/Solution의 마이그레이션 계약은 역사적 근거로 유지했습니다.

자동 검증: Vitest 파일 34개·테스트 176개, Rust 테스트 106개, typecheck, 변경 코드 ESLint,
서명 빌드, 패키지 시나리오 32/32와 컨트롤 175개가 통과했습니다. 전체 lint에는 수정하지 않은
runtime 테스트 2개 파일의 기존 오류 23개가 남습니다. 실제 IME, VoiceOver, OS 동작 줄이기,
확대, Windows 설치 검증, 앱 종료·충돌 시 Task 초안 내구성은 미검증 또는 범위 밖입니다.
[현재 화면 안내](../features/visual-guide.ko.md),
[검증 기록](../../specs/012-task-centered-workbench/acceptance-verification.md),
[검증 근거](../testing/evidence/ui-ux-improvements.json)를 참고하세요.
