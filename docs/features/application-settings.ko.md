# 애플리케이션 설정 저장소

[English](application-settings.md) | **한국어**

LLM Wiki는 API key를 포함한 애플리케이션 설정을 사용자 소유 JSON 파일 하나에 저장합니다.

```text
~/.llm-workbench/settings.json
```

Windows에서 `~`는 현재 사용자 프로필이므로 실제 위치는
`%USERPROFILE%\.llm-workbench\settings.json`입니다. 선택한 Vault 경로, 최초 실행 상태, 명시적으로
선택한 언어, provider endpoint와 model routing, report 언어, background worker 수, API key가 이
파일에 들어갑니다. API key는 UI나 MCP로 반환하지 않으며 Vault나 애플리케이션 DB에 저장하지 않습니다.
다른 provider 설정을 바꿀 때 API-key 필드를 비워 두면 저장된 키를 유지합니다.

AI 설정의 공통 **AI 요청 Timeout (초)** 은 대화 스트리밍과 연결 테스트를 포함한 모든 AI 제공자 HTTP 요청에 적용됩니다. 1–3600초 사이의 정수를 입력할 수 있고, 필드가 없는 기존 설정에는 180초 기본값이 적용됩니다. JSON의 provider 필드는 `requestTimeoutSeconds`이며 설정 API에서는 `request_timeout_seconds`입니다. 저장하면 재시작 없이 새 요청부터 적용되고, 이미 실행 중인 요청은 기존 제한을 유지합니다.

현재 설정 문서 version은 2입니다. `introCompleted`는 완전히 새로운 설치에서만 `false`로 기록하고,
소개를 건너뛰거나 완료하면 `true`가 됩니다. 이 필드가 없으면 소개 기능 이전의 기존 설치로 판단하므로
upgrade 뒤 최초 설치 animation이 갑자기 나타나지 않습니다.

프로세스 공통 `SettingsStore` Singleton이 모든 설정 읽기와 저장을 관리합니다. 설정 파일은 처음 한 번만 읽고 시스템에는 변경할 수 없는 스냅샷을 제공합니다. 저장은 잠금으로 직렬화하며 파일을 원자적으로 교체한 뒤 성공한 경우에만 공유 스냅샷을 갱신합니다. 저장 실패 시 기존 메모리 값을 유지하고, 격리된 앱 home은 각각 별도의 스냅샷을 사용합니다. JSON 파일을 직접 편집했다면 앱을 재시작해야 적용됩니다. Unix에서는 디렉터리 권한을 `0700`,
파일 권한을 `0600`으로 제한합니다. Workflow 기록, 색인, Job, 알림, 생성된 애플리케이션 상태는
플랫폼 앱 데이터 위치의 SQLite DB에 유지됩니다.

SQLite에 설정을 저장하던 버전에서 업그레이드하면 `settings.json`이 없을 때만 기존 Vault, locale,
provider 값을 최초 시작 과정에서 가져옵니다. Rollback 안전성을 위해 기존 DB row는 남겨 두지만 이전
후에는 읽거나 갱신하지 않습니다. 새 DB에는 예전 설정 table을 생성하지 않습니다.

AI setup에는 현재 사용 중인 Vault 경로도 표시됩니다. `Vault 위치 변경`은 네이티브 폴더 선택기를
열고 선택한 폴더를 새 Vault로 사용하도록 앱을 다시 시작합니다. `Embedding 재생성`은 현재 Vault의
Markdown 문서를 다시 읽어 번들 모델로 semantic embedding을 재생성하며, Vault 파일이나 앱 기록을
삭제하지 않습니다.

격리된 개발과 자동화 test에서는 `LLM_WORKBENCH_HOME`으로 설정 디렉터리를 바꿀 수 있습니다.
명시적인 process-level override가 없다면 production package는 항상 현재 사용자의 home을 사용합니다.

## 구조화 출력 호환성

구조화 AI 작업은 strict `response_format` JSON Schema를 지원하는 OpenAI 호환 Chat Completions 엔드포인트와 모델이 필요합니다. 모든 구조화 요청에 실제 스키마를 전달하며, 모델 목록 연결 테스트만으로 이 기능을 확인할 수는 없습니다. provider가 요청을 거부하면 명확한 오류를 표시하고 스키마를 뺀 재시도는 하지 않습니다. 새 모델 호출이나 스키마 없는 fallback은 추가하지 않습니다. 다른 업체의 고유 API 형식은 이 앱에서 지원하는 어댑터가 아닙니다. 자유 형식 스트리밍 대화와 Codex Task 실행은 기존 텍스트 계약을 유지합니다.

프로토콜은 [공식 Structured Outputs 가이드](https://developers.openai.com/api/docs/guides/structured-outputs)를 따릅니다. 사용자 지정 호환 엔드포인트도 같은 계약을 지켜야 하며, 로컬 테스트가 모든 provider와 모델의 호환성을 보장하지는 않습니다.
