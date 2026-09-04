# LLM Wiki

[English](README.md) | **한국어**

> **You talk. The work organizes itself. — 말하면, 일이 스스로 정리됩니다.**

LLM Wiki는 대화를 구조화된 일로 바꾸고, 언제든 이어갈 수 있도록 맥락을 보존하며, 완료된
결과만 휴대 가능한 Knowledge로 만드는 AI 중심·로컬 우선 Workbench입니다.

![가벼운 Capture 입력과 현재 진행 중인 Solution을 강조하는 최신 LLM Wiki Workbench](docs/features/images/02-workbench.png)

## Product Spirit

LLM Wiki는 여섯 가지 타협할 수 없는 원칙에서 출발합니다.

1. **You talk. The work organizes itself.** 대화와 Refinement가 구조화를 담당합니다.
2. **Reduce cognitive load.** Capture는 가볍게 유지하고, 지금 진행 중인 일을 가장 선명하게 보여줍니다.
3. **Resume where you left off.** Work Log 스크린샷, Refinement 맥락, Knowledge 기반 충돌 검토로 바로 이어갑니다.
4. **Organize around problems, not tasks.** 지속되는 흐름은 **Capture → Problem → Solution**입니다. 실행 정보는 Solution Work Log와 검증 체크리스트에 남습니다.
5. **Private process, portable knowledge.** 초안과 작업 맥락은 로컬에 두고, 사람이 승인한 완료 결과만 Markdown Knowledge가 됩니다.
6. **Understand the work, never score the worker.** 근거·위험·방향은 이해하되 사람의 생산성을 점수화하지 않습니다.

각 원칙이 실제 제품에 어떻게 반영됐는지는 [제품에 녹아든 Product Spirit](docs/product-spirit.ko.md)에서 확인할 수 있습니다.

## 제품이 하는 일

| 필요 | LLM Wiki의 방식 |
| --- | --- |
| 생각을 빠르게 꺼내기 | Capture는 처음부터 구조를 요구하지 않습니다. |
| 진짜 Problem 이해하기 | AI Refinement가 맥락을 보존하고 검토 가능한 Problem을 제안합니다. |
| 해결 방향 선택하기 | 승인된 Problem에서 Solution을 만들고 Knowledge 기반 충돌 검토가 clear일 때 시작합니다. |
| 진행 중인 일 이어가기 | In Progress Solution을 강조하고 Work Log에 텍스트·스크린샷·댓글·검증 기준을 남깁니다. |
| 근거로 완료하기 | AI가 기록된 근거를 검토하되 완료 결정은 사람이 내립니다. |
| 결과 재사용하기 | 완료된 결과만 Obsidian 호환 Playbook과 검색 가능한 Knowledge가 됩니다. |
| AI 작업 중에도 빠르게 반응하기 | 숨겨진 Fast Queue는 상호작용 요청을 제한하고 지속 작업은 백그라운드 Queue에서 조회·복구합니다. |

Task 단계는 없습니다. 실행을 Solution에 붙여 두어 일이 존재하는 이유인 Problem과 분리되지 않게 합니다.

## 빠른 시작

Node.js 22 LTS, stable Rust toolchain과 플랫폼별
[Tauri 사전 요구사항](https://v2.tauri.app/start/prerequisites/)이 필요합니다.

```text
npm ci
LLM_WIKI_VAULT=/path/to/your-vault npm run tauri -- dev
```

**AI 설정**에서 OpenAI 호환 endpoint와 model을 설정합니다. AI는 필수 제품 기능이며, 로컬 검색과
수동 제어는 provider 장애 중에도 개인 작업 과정과 사용자의 결정권을 보존하기 위한 fallback입니다.

비밀정보가 아닌 설정은 `~/.llm-workbench/settings.json`에 저장합니다. API key는 Vault, 설정 파일,
앱 DB가 아니라 macOS Keychain 또는 Windows Credential Manager에 저장됩니다.

React frontend는 네이티브 Tauri 데스크톱 애플리케이션으로 패키징됩니다. 데스크톱 process는 Rust
도메인 명령을 통해 SQLite DB와 선택한 Vault를 직접 엽니다.

```text
npm run tauri:build
```

데스크톱 빌드는 고정 리비전과 SHA-256으로 검증한 다국어 MiniLM ONNX 모델을 무시된 빌드
리소스 캐시에 내려받아 앱에 포함합니다. Release 앱의 의미 인덱싱과 검색은 로컬에서 실행되며 앱
시작 시 임베딩 모델을 내려받지 않습니다. 캐시를 검증하거나 복구하려면
`npm run prepare:embedding`을 다시 실행합니다.

Nunito, DM Mono, variable Noto Sans KR도 모든 production build에서 앱 asset으로 복사됩니다.
따라서 패키징 UI는 web font 요청 없이 한국어와 영어를 표시합니다. macOS build는 `.app`을 만들고,
Windows build는 MSI와 NSIS installer를 만듭니다. Windows agent가 그대로 실행할 수 있는 절차는
[Windows 패키징 및 설치](docs/windows-packaging.ko.md)에 정리되어 있습니다.

Release `.app`에는 Python Runtime, Sidecar, 내부 Web Server가 없습니다.
새 설치는 Workbench를 조작하기 전에 네이티브 폴더 선택기를 열고 사용자가 고른 Markdown Vault를
기억합니다. 기존 설치는 `Documents/LLM Wiki Vault`를 승계하며 `LLM_WIKI_VAULT`는 개발용
override로 유지됩니다. React는 Workflow, Vault, 설정, Job, System 명령을 각각 호출하며 Chat
Chunk와 취소는 네이티브 Tauri Channel을 사용합니다. Socket은 사용자가 명시적으로 설정한 외부 AI
Provider 호출에만 열립니다. 자세한 내용은
[최초 실행 Vault 설정](docs/features/first-run-vault-setup.ko.md)을 참고하세요.
[애플리케이션 설정 저장소](docs/features/application-settings.ko.md)에서 파일 구조, 기존 SQLite
설정 이전, 플랫폼별 경로를 확인할 수 있습니다.
기존 Python/FastAPI 브라우저 전달은 네이티브 명령 parity 검증 후 폐기했습니다. 마지막 구현은
`caef236` Git history에서만 확인할 수 있습니다.

Worker 역할, 복구, 작업별 결과, 알림 동작은
[백그라운드 AI Queue](docs/features/background-ai-queue.ko.md)를 참고하세요.

### 한국어와 영어

전역 언어 제어에서 한국어와 영어를 전환할 수 있습니다. 새로 고치거나 현재 화면과 입력을 잃지
않으며, 명시적으로 선택한 언어는 다음 실행에도 유지됩니다. 저장된 선택이 없으면 한국어 환경은
한국어를, 그 밖의 환경은 영어를 사용합니다.

새로 생성하고 검토한 AI Problem·Solution은 한국어·영어 저장본을 함께 유지합니다. 기존 기록과
Vault 파일은 변경하지 않으며 대상 언어 버전이 없으면 저장된 원문을 표시합니다. 실시간 AI
콘텐츠는 요청 시작 시점의 언어 하나로 생성됩니다. 앱이 관리하는 Knowledge는 영문 canonical
Markdown으로 유지하고, 한국어 열람본은 휴대 가능한 원본을 바꾸지 않은 채 요청 시 생성합니다.
자세한 내용은 [한국어·영어 전환](docs/features/bilingual-localization.ko.md)을 참고하세요.

## 기능 안내

- [Product Spirit과 제품 결정](docs/product-spirit.ko.md)
- [실행 화면으로 보는 기능 둘러보기](docs/features/visual-guide.ko.md)
- [빠른 Vault 검색](docs/features/fast-vault-search.ko.md)
- [Problem 중심 Workbench](docs/features/conflict-gated-workflow.ko.md)
- [완료·Knowledge·아카이브](docs/features/completion-writeback-archive.ko.md)
- [Compass](docs/features/direction-dashboard.ko.md)
- [맥락을 보존하는 Refinement Preview](docs/features/refinement-preview-status.ko.md)
- [한국어·영어 전환](docs/features/bilingual-localization.ko.md)

## 개발

[빠른 작업 Worktree](docs/worktree-workflow.ko.md)에 설명한 shared-cache helper로 격리된 task branch를
만듭니다. 새 Worktree는 Node dependency, compile된 Rust dependency, 검증된 embedding asset을
처음부터 다시 준비하지 않고 재사용합니다.

모든 Spec·Plan·구현·리뷰는 [프로젝트 Constitution](.specify/memory/constitution.md)의 Product
Spirit Review Gate를 통과해야 합니다.

애플리케이션 계층, 의존성 방향, 네이티브 작업 경계는
[아키텍처 안내](docs/architecture.ko.md)에 정리되어 있습니다.

```text
npm test
npm run typecheck
npm run lint
npm run build
cargo test --manifest-path src-tauri/Cargo.toml
cargo check --manifest-path src-tauri/Cargo.toml
npm run tauri:build
npm run test:desktop
```

Spec Kit 산출물은 [specs/](specs/)에 있습니다.
