# Task 중심 MCP 보완 검증

**한국어** | [English](mcp-task-workflow-verification.md)

날짜: 2026-09-12. 구현·macOS 릴리스 검증·로컬 설치를 완료했다.

구현 전에 GPT-6 Astra/high가 검토한 [DB Migration Plan](../plans/mcp-task-db-migration-plan.ko.md)과
[보완 계획](../plans/mcp-task-workflow-remediation.ko.md)을 따른다. 이후 FK 테이블 재구축 방식의
정정은 SQLite 공식 일반 절차와 실제 데이터가 있는 마이그레이션 테스트에 근거한다.
추가 Astra 승인을 받은 것으로 표현하지 않는다.

확인한 항목:

- `main`의 `3bab28a`까지 통합한 전체 `npm test`: Vitest 34개 파일 184개 테스트,
  desktop helper 6개, provider fake 2개,
  signing 4개, runtime 파싱·native-only·application boundary 검사 통과.
- TypeScript typecheck와 production build 통과. 번들 폰트 subset 148개 확인.
- 최초 릴리스 검증에서 변경한 프런트엔드 파일의 ESLint는 통과했으며,
  `completedButtonsRuntime.test.ts`, `remainingButtonsRuntime.test.ts`의 기존
  `no-explicit-any` 오류 23개를 기록했다. 후속 작업에서 테스트 harness에 runtime 함수,
  Window 확장과 DOM 타입을 지정해 해결했다. 규칙 억제 없이 전체 `npm run lint` 통과.
  typecheck와 프런트엔드 전체 184개 테스트도 통과했다. 제품 코드는 변경하지 않았다.
- 데이터가 있는 v8 → v9 fixture에서 과거 row/event, parent/child 참조, 인덱스·트리거·간접
  뷰를 보존했다. 실패 rollback, 검증된 backup, 명시적 retry, 지원하지 않는 새 스키마 거부 통과.
- canonical Task 문맥, 자식 내용만 바뀐 stale source 거부, 승인·변경 atomic rollback,
  이미 승인된 Knowledge만 정확한 과거 본문으로 복구하는 Rust 집중 검사 통과.
- 최종 전체 `cargo test --no-fail-fast`: 129개 통과, 실패 0개. release application acceptance
  9개 포함. `cargo fmt --check`와 모든 target의 strict clippy 통과.
- 체크포인트·검토 ID·hash 안내를 갱신한 최종 workflow Skill contract 5개 통과.
- Tauri 서명 릴리스와 strict 서명 검증 통과. 서명은 `LLM Wiki Local Signing`,
  CDHash는 `6a9f4749dd0654fb4c8d173df205b816bc49ca38`,
  서명 시각은 `2026-09-12 13:27:49 America/New_York`이며 designated requirement는 유지했다.
- 첫 전체 패키지 실행은 31/33으로 optional Capture 직렬화 결함과 공개 시나리오 대기 결함을
  발견했다. 둘을 수정한 집중 재검사 2/2 통과. 최종 전체 실행 33/33 통과.
  스캔된 소스 컨트롤 188개 중 175개를 렌더링·조작·결과 확인했고 모든 증거 누락은 0이다.
- 검증한 릴리스로 `/Applications/LLM Wiki.app`을 교체하고 정상 실행하여 기존 Workbench를
  확인했다. 이전 앱은 `/Applications/LLM Wiki.app.previous-1789234193535-63649`에 보존했다.
  설치 중 application data·Keychain·TCC를 초기화하지 않았다.
- `llm-wiki@personal`을 `0.1.0+codex.20260912165743`으로 재설치했다.
  cache의 Skill·bridge 내용이 로컬 소스와 일치하며 기존 bridge 설정을 유지했다.
  설치된 GUI/stdio bridge의 읽기 전용 검사에서 32개 도구, `continue_task`, 안정적 operation ID,
  canonical Task 완료·Knowledge 스키마와 폐기된 Solution 승인·완료 action의 스키마 제외를 확인했다.

`.worktrees/mcp-task-workflow/.tmp/`에 보존한 artifact:

- `desktop-e2e-artifacts-bqyej6`: 진단용 첫 전체 실패 기록.
- `desktop-e2e-artifacts-85u8WL`: 통과한 집중 재검사.
- `desktop-e2e-artifacts-rmozfL`: 최종 전체 통과 `results.json`·`interactive-coverage.json`.
- `final-npm-test.log`, `final-cargo-test.log`, `final-clippy.log`, `final-lint.log`,
  `final-tauri-build.log`, `final-app-install.log`, `final-plugin-install.log`.
- `lint-followup-lint.log`, `lint-followup-typecheck.log`, `lint-followup-test.log`,
  `lint-followup-focused.log`:
  남아 있던 테스트 harness lint 오류를 제거한 후 통과한 검증.

실제 provider 품질·지연, 설치된 Windows/Linux, OS reduced-motion, 재설치한 plugin을 새
Codex task에서 로드하는 검증은 별도 외부 증거 gate로 남는다.
