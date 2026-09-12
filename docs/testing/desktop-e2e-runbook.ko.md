# 데스크톱 E2E 실행 안내

결정적인 상호작용 증거가 필요할 때 패키징된 데스크톱 실행기를 사용합니다. 저장소 루트에서 실행합니다.

```sh
npm run test:desktop -- --help
npm run test:desktop -- --list
npm run test:desktop -- --full
npm run test:desktop -- --scenario task-refinement --scenario task-refinement-provider-recovery
```

`--full`은 릴리스 수락 명령입니다. 현재 checkout의 runner에 등록한 모든 시나리오를 실행하고 결과 커버리지가 완전하지 않으면 실패합니다. 집중 실행은 부분 실행으로 표시되며 이 게이트를 통과시키지 않습니다. 최종 시나리오·컨트롤 수는 생성한 `results.json`과 `interactive-coverage.json`에서 확인하세요.

기본 실행 파일은 `src-tauri/target/release`의 릴리스 파일입니다. 전체 실행 전에 릴리스 번들을 빌드하세요. 이미 서명된 빌드를 의도적으로 재사용하려면 `LLM_WIKI_E2E_EXECUTABLE=/absolute/path/to/executable`을 설정합니다. 결정적인 로컬 제공자는 자동으로 시작되며 외부 제공자 인증 정보는 필요하지 않습니다.

각 시나리오는 새 Vault, SQLite 데이터베이스, workbench 홈, IPC 엔드포인트와 로컬 가짜 제공자를 받습니다. 결과는 `.tmp/desktop-e2e-artifacts-*/results.json` 또는 `--artifact-dir PATH`에 기록되고 커버리지는 `interactive-coverage.json`에 기록됩니다. 실패한 사례는 로그, 격리 상태와 `failure.txt`를 보존합니다. 임시 상태를 확인하려면 `--keep-state`를 사용하세요.

새 시나리오는 `DESKTOP_E2E_SCENARIO_NAMES`에 이름을 등록하고 데스크톱 테스트 흐름을 구현한 뒤 렌더링·실행·검증된 컨트롤 ID를 보고해야 합니다. 일반적인 실패는 릴리스 실행 파일 누락, 오래된 아티팩트, 제공자 시작 시간 초과, 격리 상태/IPC 충돌입니다. 보존된 사례 디렉터리를 확인한 후 가장 작은 영향 시나리오를 다시 실행하세요.

실행기와 아티팩트 구조는 macOS, Linux, Windows에서 지원됩니다. 서명된 번들 경로와 코드 서명 확인은 macOS 전용이며, Windows와 Linux는 실행 파일 및 상호작용 증거를 제공합니다.

코드 변경 후 새 릴리스로 검증할 때는 다음 순서로 실행합니다. 빌드에는 플랫폼별 도구와 macOS 서명 설정이 필요합니다([패키징 안내](../macos-packaging.ko.md)).

```sh
npm run tauri:build
npm run test:desktop -- --full
```

진단 결과를 지정한 작업 폴더에 남기는 예시입니다. 실행마다 새 폴더 이름을 사용하세요.

```sh
npm run test:desktop -- --scenario task-refinement --artifact-dir .tmp/refinement-check-01 --keep-state
```

시나리오 등록은 [러너 레지스트리](../../scripts/desktop_e2e_helpers.mjs)와 [앱 시나리오 디스패처](../../frontend/src/test/desktopScenario.ts) 양쪽에 반영합니다. 커버리지 확장 기준은 [상호작용 검증 문서](interactive-coverage.md)를 참고하세요. 역사 수치를 릴리스 보고서에 복사하지 말고, 최종 registry와 생성한 coverage artifact를 사용하세요.

이 검증은 가짜 제공자를 사용하므로 실제 AI 응답의 품질·속도를 보장하지 않습니다. 이번 패키징 실행 증거는 macOS에서 확보했으며 Windows/Linux 실기기 검증은 별도입니다.
