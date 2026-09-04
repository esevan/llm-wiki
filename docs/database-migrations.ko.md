# SQLite schema migration

[English](database-migrations.md)

네이티브 애플리케이션은 `src-tauri/src/native/migrations.rs` 한 곳에서 SQLite schema migration을
관리합니다. SQLite의 `PRAGMA user_version`에 마지막으로 commit된 version을 기록합니다. 앱 시작
시 domain command나 기존 설정 import가 DB를 읽기 전에 이후 migration을 version 순서대로 모두
실행합니다.

## 보장 사항

- Version `0`은 이전 Python 애플리케이션 또는 과거 네이티브 build가 만든 version 미지정 DB입니다.
- 각 migration은 별도의 `IMMEDIATE` transaction에서 실행됩니다. Schema/data 변경과
  `user_version` 변경이 함께 commit되며 오류가 발생하면 둘 다 rollback됩니다.
- Migration version은 빠짐없이 오름차순이어야 합니다. Compile된 계획이 잘못되면 DB를 바꾸기
  전에 실패합니다.
- 현재 version에서 앱을 다시 시작하면 migration은 아무 작업도 하지 않습니다.
- 앱이 지원하는 version보다 새로운 DB는 수정하지 않고 열기를 거부합니다.
- Version 2 호환 migration은 기존 field 단위 localization 데이터를 보존하고 Python 앱에서
  조건부로 추가했던 것으로 확인된 모든 column을 추가합니다.

## Migration 추가 방법

1. `native/schema.sql`의 version 1 baseline과 이미 release한 migration은 수정하지 않습니다.
2. `native/migrations.rs`에 목적이 하나인 migration function을 추가합니다.
3. Version, 이름, function을 `MIGRATIONS` 끝에 추가하고 `CURRENT_SCHEMA_VERSION`을 올립니다.
4. 파괴적 변경이나 data rewrite는 명시적이고 결정적으로 작성합니다. DB migration 안에서 network,
   Vault, provider, UI 작업을 실행하지 않습니다.
5. 바로 전 version upgrade, data 보존, 실패 rollback, 재실행 idempotence, 새로운 version 거부 test를
   필요한 범위에 맞게 추가합니다.
6. Release 전에 `cargo test --manifest-path src-tauri/Cargo.toml`과 package desktop 재실행 E2E를
   실행합니다.

애플리케이션 설정은 `~/.llm-workbench` 아래의 별도 version JSON 파일입니다. 기존 SQLite 설정을
한 번 가져오는 작업은 schema migration이 성공한 뒤에만 DB를 읽습니다.
