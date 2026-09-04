# Windows 패키징 및 설치

[English](windows-packaging.md) | **한국어**

이 문서는 Windows build agent가 네이티브 LLM Wiki 설치 파일을 만들고 선택적으로 설치하는 절차입니다.
저장소에는 Python runtime이나 브라우저 backend가 없습니다. React UI, Rust 애플리케이션, 번들 폰트,
고정 리비전 다국어 ONNX 모델을 Tauri 설치 파일에 포함합니다.

## 지원 빌드 환경

- x64 Windows 10 1803 이상 또는 Windows 11
- Git for Windows
- Node.js 22 LTS와 npm
- MSVC toolchain을 사용하는 Rustup
- **Desktop development with C++**, MSVC v143, Windows 10 또는 11 SDK를 설치한 Visual Studio
  2022 Build Tools
- Microsoft Edge WebView2 Runtime(최신 Windows에는 일반적으로 포함됨)
- 의존성과 체크섬 고정 모델을 복원하기 위한 빌드 시점 인터넷 연결

패키징 전에 `winver`, `node --version`, `npm --version`, `rustup --version`, `where.exe cl`을
확인합니다. `cl`이 없으면 Visual Studio workload를 설치하고 PowerShell을 새로 여세요.

## Agent용 단일 명령 흐름

깨끗한 `main` checkout에서 64비트 PowerShell로 실행합니다.

```powershell
git clone <repository-url> llm-wiki
Set-Location llm-wiki
git checkout main
powershell -ExecutionPolicy Bypass -File .\scripts\package_windows.ps1
```

스크립트는 Python source가 없음을 확인하고 stable MSVC Rust toolchain 설치, 잠금된 Node 의존성
복원, 임베딩 모델 검증, lint/typecheck/React/runtime/Rust 테스트, UI build, Tauri package 생성을
차례대로 수행합니다.

설치 파일 위치:

```text
src-tauri\target\release\bundle\msi\*.msi
src-tauri\target\release\bundle\nsis\*.exe
```

성공한 package를 바로 설치하려면 로컬 애플리케이션 설치 권한이 있는 PowerShell에서 실행합니다.

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\package_windows.ps1 -Install
```

스크립트는 MSI를 우선 사용하고, 없으면 NSIS installer를 실행합니다. 설치가 끝날 때까지 기다리고
실패 종료 코드를 거부하며, MSI 코드 3010은 재시작이 필요한 설치 성공으로 안내합니다.

## 설치 후 검증

1. 시작 메뉴에서 **LLM Wiki**를 실행합니다.
2. 최초 실행 네이티브 Vault 선택기에서 기존 Markdown 폴더를 선택합니다.
3. Terminal, Python process, localhost service 없이 Workbench가 열리는지 확인합니다.
4. Capture를 만들고 앱을 완전히 종료한 뒤 다시 실행하여 Capture가 유지되고 Vault 선택기가 다시
   나타나지 않는지 확인합니다.
5. 선택한 Vault에 Markdown 파일을 넣고 **Vault 검색**에서 의미 기반을
   켠 뒤 background indexing 이후 노트가 반환되는지 확인합니다.
6. AI 설정에서 API key가 설정 여부로만 표시되는지 확인합니다. Key는 Windows Credential Manager에
   저장되며 React로 값이 반환되지 않습니다.
7. `%USERPROFILE%\.llm-workbench\settings.json`에 Vault와 비밀정보가 아닌 UI/provider 설정이 있고,
   앱 데이터 SQLite DB에는 Workflow와 색인 데이터만 있는지 확인합니다.

Installer는 upgrade 중 home 설정 파일을 덮어쓰거나 제거하지 않습니다. SQLite에 설정을 저장하던 기존
설치는 home 파일이 없을 때 한 번만 설정을 가져옵니다. Build와 E2E 자동화는
`LLM_WORKBENCH_HOME`을 격리된 임시 디렉터리로 지정하므로 agent 실행이 대화형 Windows 사용자의
설정을 변경하지 않습니다.

Tauri content-security policy의 `http://ipc.localhost`는 WebView2가 사용하는 가상 IPC origin이며
수신 대기 TCP socket이 아닙니다. Network 연결은 사용자가 외부 AI provider를 설정했을 때만 발생합니다.

## 서명과 배포

서명하지 않은 MSI/NSIS는 통제된 내부 테스트에는 사용할 수 있지만 Windows SmartScreen 경고가 나타날
수 있습니다. 공개 배포에는 조직 소유 code-signing certificate와 CI secret 설정이 필요합니다. PFX나
password를 저장소에 넣지 마세요. Build agent가 release credential에 암묵적으로 접근하지 않도록 로컬
패키징 스크립트는 의도적으로 서명을 수행하지 않습니다.

실패하면 전체 PowerShell log와 `rustc -vV`, `cargo -vV`, `node --version`, `where.exe cl` 출력을
보관하세요.
