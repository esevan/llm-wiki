# 애플리케이션 설정 저장소

[English](application-settings.md) | **한국어**

LLM Wiki는 비밀정보가 아닌 애플리케이션 설정을 사용자 소유 JSON 파일 하나에 저장합니다.

```text
~/.llm-workbench/settings.json
```

Windows에서 `~`는 현재 사용자 프로필이므로 실제 위치는
`%USERPROFILE%\.llm-workbench\settings.json`입니다. 선택한 Vault 경로, 최초 실행 상태, 명시적으로
선택한 언어, provider endpoint와 model routing, report 언어, background worker 수가 이 파일에
들어갑니다. API key는 이 파일에 기록하지 않으며 계속 macOS Keychain 또는 Windows Credential
Manager가 관리합니다.

설정 저장은 process-wide lock과 임시 파일 교체를 사용합니다. Unix에서는 디렉터리 권한을 `0700`,
파일 권한을 `0600`으로 제한합니다. Workflow 기록, 색인, Job, 알림, 생성된 애플리케이션 상태는
플랫폼 앱 데이터 위치의 SQLite DB에 유지됩니다.

SQLite에 설정을 저장하던 버전에서 업그레이드하면 `settings.json`이 없을 때만 기존 Vault, locale,
provider 값을 최초 시작 과정에서 가져옵니다. Rollback 안전성을 위해 기존 DB row는 남겨 두지만 이전
후에는 읽거나 갱신하지 않습니다. 새 DB에는 예전 설정 table을 생성하지 않습니다.

격리된 개발과 자동화 test에서는 `LLM_WORKBENCH_HOME`으로 설정 디렉터리를 바꿀 수 있습니다.
명시적인 process-level override가 없다면 production package는 항상 현재 사용자의 home을 사용합니다.
