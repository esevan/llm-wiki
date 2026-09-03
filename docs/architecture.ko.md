# 네이티브 애플리케이션 아키텍처

[English](architecture.md) | **한국어**

LLM Wiki는 React와 Tauri로 구성한 네이티브 전용 애플리케이션입니다. React는 하나의 typed
`ApplicationClient`에 의존하며 Tauri adapter는 프레젠테이션 호환 작업을 제한된 도메인 command로
변환합니다. Rust가 애플리케이션 동작, SQLite persistence, Vault 파일 접근, background job, provider
연동, localization, offline semantic retrieval을 소유합니다.

```text
React 기능 -> ApplicationClient -> Tauri 도메인 command -> Rust 애플리케이션 모듈
                                                    -> SQLite / Markdown Vault
                                                    -> 번들 ONNX 임베딩
                                                    -> 설정한 AI provider
```

FastAPI server, 내부 TCP listener, Python process, sidecar, HTTP application adapter, web 전달 모드는
없습니다. Content-security policy의 `http://ipc.localhost`는 Tauri의 가상 WebView IPC origin이며
수신 대기 socket이 아닙니다. HTTP는 사용자가 외부 AI provider를 설정했을 때만 사용합니다.

## Frontend 경계

| 경계 | 위치 | 책임 |
| --- | --- | --- |
| App shell | `frontend/src/app/` | 창 탐색과 최상위 조립 |
| 기능 | `frontend/src/features/` | 도메인 화면과 사용자 상호작용 |
| 공용 UI | `frontend/src/components/` | 실제로 재사용하는 primitive |
| Application client | `frontend/src/services/` | Typed Tauri 애플리케이션 경계 |
| Theme | `frontend/src/theme/` | Semantic token, 전역 규칙, component style |
| 정적 controller | `frontend/public/runtime/` | 경계가 분명한 호환 controller 11개 |
| Localization | `frontend/public/i18n/` | 번들된 영어·한국어 resource |
| Build output | `dist/` | Tauri가 소비하는 재현 가능한 무시 대상 산출물 |

호환 controller에는 raw `fetch`나 직접 IPC가 없습니다. 중앙 application client만 호출하며 Tauri
`invoke`와 `Channel`은 `tauriApplicationClient.ts`만 import합니다.

## 네이티브 경계

| 경계 | 위치 | 책임 |
| --- | --- | --- |
| Tauri 진입점 | `src-tauri/src/lib.rs` | 도메인 allowlist 검증과 command 위임 |
| Workflow | `src-tauri/src/native/workflow.rs` | Capture, Problem, Solution, 전이, 근거, Compass |
| Job | `src-tauri/src/native/jobs.rs` | 지속 lifecycle, retry, timeout, cancellation |
| 결과 handler | `src-tauri/src/native/job_results.rs` | 작업별 결과 검증과 저장 |
| Completion/Lineage | `completion.rs`, `lineage.rs` | 완료 기록과 감사 가능한 lineage |
| Vault | `vault.rs`, `patches.rs`, `projection.rs` | 안전한 Markdown index, 검색, atomic write |
| Semantic engine | `semantic.rs` | 번들 다국어 ONNX 추론 |
| 설정 | `settings.rs`, `localization.rs` | Locale, provider routing, secret-safe 설정 |
| Provider | `src-tauri/src/provider.rs` | 유일한 외부 AI protocol adapter |

Tauri command는 도메인 선택, 작업 경계 검증, `NativeApplication` 호출, 결과 변환만 담당합니다.
Persistent write는 transaction 또는 임시 파일 교체를 사용하고 source hash가 오래된 게시를 차단하며,
Windows overwrite는 `ReplaceFileW`를 사용합니다.

## 시작과 리소스

창은 Vault indexing보다 먼저 표시됩니다. Blocking worker가 초기 scan을 실행하고 semantic 작업이
필요할 때만 checksum 고정 다국어 MiniLM model을 로드합니다. Release 앱은 runtime에 model이나 font를
내려받지 않습니다. Build 준비 단계가 model 파일 5개와 Nunito, DM Mono, Noto Sans KR WOFF2 subset
148개를 검증합니다.

## 검증 경계

- Vitest가 React 조립과 Tauri application adapter를 보호합니다.
- `verify_runtime.mjs`가 호환 controller 11개를 parse하고 HTTP fallback을 거부합니다.
- Rust 단위·command test가 실제 SQLite, filesystem, workflow, cancellation, ONNX 동작을 검증합니다.
- 패키징 desktop E2E는 React → Tauri → Rust → SQLite/Vault와 전체 재실행을 검증합니다.
- macOS는 `.app`을 만들며 Windows agent는 [Windows 패키징 안내](windows-packaging.ko.md)에 따라 MSI와
  NSIS installer를 만듭니다.

폐기된 FastAPI 구현과 characterization test는 Git commit `caef236`에서 확인할 수 있습니다.
[폐기 기록](migrations/python-browser-retirement.md)을 참고하세요.
