# 기여 안내

도메인 모델, 사용자가 보는 동작, 저장된 결과, 이를 실행한 검증이 이어지는 근거를 남기세요. 관련 경로만 먼저 확인하고 변경 경계를 분명히 유지합니다.

## 실제 workflow를 먼저 모델링하기

- 직접 Task를 만들지 선택적인 Problem 맥락을 연결할지 결정하기 전에 실제 도메인 관계, 독립 lifecycle, 원본 근거를 모델링하세요. 제품에 없는 1:1 연결을 강제하지 마세요.
- 이미 기록한 Capture, Task 맥락, 제안한 접근을 다시 탐색하지 말고 입력으로 사용하세요. 기계적인 승인을 품질 근거로 삼지 마세요.
- UI action에서 API 또는 IPC 계약, service 경계, persisted state까지 추적하세요. 변경 책임을 먼저 정하고 무관한 구조 개편은 분리하세요.

## 경계를 넘는 동작 검증하기

- E2E 수는 coverage가 아닙니다. interactive control 목록을 만들고 rendered, exercised, asserted 여부를 의미 있게 확인하세요. unavailable 또는 disabled 경로는 제외 이유를 기록하세요.
- 다중 턴 workflow는 입력 순서, provider history, 실제 저장 결과를 확인하세요. deterministic fake provider는 애플리케이션 동작과 재현성을 증명하지만 실제 AI 품질이나 속도는 증명하지 않습니다.
- 비동기 작업은 성공, 중복 클릭, 지연, 실패, 재시도, 닫기와 재열기, 범위에 포함된 process 재시작을 다루세요. UI를 닫았다고 서버나 job이 취소된 것은 아닙니다.

## 점진적으로 작업하고 근거 남기기

- 반복 실패 시 무작정 재시도하지 마세요. 실패를 보존하고 증거와 원인 가설로 범위를 좁힌 뒤 가장 작은 변경을 하세요.
- 모델은 위임, context, 실행, 재실행을 포함한 총비용으로 선택하세요. 맞는 가장 저렴한 모델부터 시작하고 실제 병목에서만 상향하세요.
- 애플리케이션 코드·테스트 변경은 전용 worktree에서 작업하세요. 관련 검증을 점진적으로 실행하고, 구현과 문서가 최종 정리되면 릴리스 빌드와 패키징된 앱의 전체 E2E를 한 번 실행하세요. 새 변경이나 실패 근거가 없으면 통과한 검증을 반복하지 마세요. 문서 전용 변경은 문서 검증으로 마무리합니다.
- [데스크톱 E2E 실행 안내](docs/testing/desktop-e2e-runbook.ko.md)의 `--list`, `--scenario`, `--full`, `--artifact-dir`, `--keep-state` 명령으로 결과를 재현하고 artifact를 남기세요.
- 완료는 주장한 동작이 실제 artifact, 코드, 문서와 일치하는 상태입니다. fake provider 검증과 실제 provider 품질 주장을 구분해 보고하세요.

인계 전 `git diff --check`를 실행하고 변경된 링크와 영어·한국어 문서의 대응을 확인하세요. 자세한 규칙은 [docs/DOCUMENTATION_GUIDE.md](docs/DOCUMENTATION_GUIDE.md)를 참고하세요.
