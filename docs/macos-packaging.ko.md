# macOS 패키징과 설치

[English](macos-packaging.md) | **한국어**

LLM Wiki는 bundle identifier(`com.llm-wiki.desktop`)와 설치 경로
(`/Applications/LLM Wiki.app`)를 유지합니다. macOS의 Keychain 접근과 개인정보 권한은 앱의 code-signing
designated requirement에 연결될 수 있습니다. ad-hoc signature로 다시 빌드하면 이 requirement가 빌드마다
달라지므로 release 패키징은 ad-hoc signing을 거부합니다.

## 서명 identity를 한 번 설정하기

배포용에는 조직의 **Developer ID Application** identity를 사용하세요. Gatekeeper와 notarization 흐름을
지원하므로 장기적으로 권장합니다.

이 Mac에서만 사용할 앱이라면 안정적인 로컬 self-signed identity로 이후 교체에서도 같은 designated
requirement를 유지할 수 있습니다. **Keychain Access**에서 **Certificate Assistant → Create a
Certificate**를 선택하고 `LLM Wiki Local Signing`처럼 계속 사용할 이름을 지정합니다. **Self Signed
Root**와 **Code Signing**을 선택하고 생성된 private key는 login keychain에 보관합니다. 한 번만
만들고 build마다 새 identity를 만들지 마세요. 기본 keychain 정책으로 code signing에 충분하므로
certificate를 **Always Trust**로 바꾸지 마세요.

identity의 SHA-1 fingerprint를 확인하고 바뀔 수 있는 표시 이름에 의존하지 않도록 fingerprint를 한 번 등록합니다.

```text
security find-identity -v -p codesigning
node scripts/register_macos_signing_identity.mjs 40_CHARACTER_FINGERPRINT
npm run tauri:build
```

등록은 public fingerprint만 `~/.llm-workbench/macos-signing.json`에 사용자 전용 권한으로 저장하며,
certificate나 private key를 export하지 않습니다. 패키지 명령은 그 fingerprint가 login keychain의
유효한 identity인지 확인하고 Tauri에 전달합니다. 완성된 bundle은 `codesign --verify --deep --strict`로
확인하며, designated requirement 누락, 잘못된 bundle identifier, `Signature=adhoc`를 거부합니다.
필요하면 명시적인 `LLM_WIKI_CODESIGN_IDENTITY` 환경 값으로 등록된 fingerprint를 일시적으로
override할 수 있습니다. `npm run tauri:build -- --no-bundle`은 unsigned compile-only CI 확인에 계속
사용할 수 있습니다.

## 검증된 교체본 설치

서명된 build가 성공한 뒤에는 guarded installer로만 설치합니다.

```text
node scripts/install_macos_app.mjs --replace
```

이 도구는 `/Applications/LLM Wiki.app`을 변경하기 전에 후보 앱을 검증하고, 설치된 앱의 designated
requirement와 비교한 뒤 일치할 때만 복사합니다. `~/.llm-workbench`, 앱 database, Keychain 항목,
TCC 개인정보 권한을 삭제하거나 재설정하지 않습니다. 이후 교체가 복구본을 덮어쓰지 않도록 이전 app
bundle은 고유한 `/Applications/LLM Wiki.app.previous-<timestamp>-<process-id>` 경로에 보관합니다.

기존 ad-hoc build를 처음 교체하면 designated requirement가 반드시 바뀝니다. 표시된 signature를
확인한 뒤 이 한 번의 migration만 명시적으로 진행하세요.

```text
node scripts/install_macos_app.mjs --replace --accept-designated-requirement-change
```

이전 app bundle은 복구용으로 남습니다. 이전 ad-hoc 앱이 만든 provider key는 old build의 cdhash를
Keychain access rule에 사용했을 수 있으므로 migration 후 한 번 다시 입력해야 할 수 있습니다. 이후에는
같은 signing identity를 유지해야 새 Keychain과 TCC identity가 교체 후에도 유지됩니다. 이 절차만으로
특정 폴더 권한의 유지가 증명되지는 않습니다. 한 번 권한을 부여하고 두 번째 signed build를 설치한 뒤
같은 폴더를 열어 확인하세요.

## Release 확인

각 release candidate에서 signed build 뒤에 일반 packaged scenario를 실행합니다.

```text
npm run tauri:build
npm run test:desktop
codesign -dvvv "/Applications/LLM Wiki.app"
codesign -d -r- "/Applications/LLM Wiki.app"
```

마지막 명령은 교체 전후에 같은 designated requirement를 표시해야 합니다. Developer ID certificate 또는
로컬 self-signed identity는 설치된 제품의 수명 동안 보관하세요. 잃어버리거나 교체하면 새 identity
migration이 시작됩니다.
