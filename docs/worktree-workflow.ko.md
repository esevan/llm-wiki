# 빠른 작업 Worktree

[English](worktree-workflow.md)

저장소 작업은 전용 Git Worktree에서 격리하되, 변경되지 않는 dependency와 build artifact는 기본
checkout에서 재사용합니다. 작업마다 Node package를 다시 설치하고 Rust dependency 전체를 다시
compile하거나 번들 embedding model을 다시 download하지 않습니다.

macOS 또는 Linux에서는 이 저장소에 속한 checkout에서 다음 명령을 실행합니다.

```text
scripts/create_task_worktree.sh <task-name> <branch-name> [start-point]
```

Windows에서는 다음 명령을 사용합니다.

```powershell
.\scripts\New-TaskWorktree.ps1 -TaskName <task-name> -BranchName <branch-name> [-StartPoint main]
```

두 명령은 `.worktrees/<task-name>`을 만들고 다음 항목을 재사용합니다.

- 기본 checkout의 `node_modules`
- compile된 Rust dependency가 들어 있는 `src-tauri/target`
- checksum 검증을 통과한 embedding model asset
- Rust test를 바로 실행할 수 있도록 복사한 현재 `dist/` frontend build

스크립트는 embedding asset을 commit된 크기와 SHA-256 manifest로 전부 검증합니다. 기본 cache가
준비되지 않았거나 model 검증이 실패하면 Worktree를 만들기 전에 중단합니다. 기본 checkout에서
`npm ci`, `npm run build:desktop`, Cargo build 또는 test를 한 번 실행해 cache를 준비합니다. npm
download cache와 Cargo registry cache는 사용자 계정 단위 cache를 그대로 사용합니다.

Task branch는 선택한 start point와 같은 lockfile 및 model manifest로 시작합니다.
`package-lock.json`을 의도적으로 바꾸는 작업은 `npm ci` 전에 Worktree의 `node_modules` link를
제거하고 task-local dependency를 설치해야 합니다. Embedding manifest를 바꾸는 작업은
`npm run prepare:embedding` 전에 공유 model binary link를 제거해야 합니다. Cargo는 공유 target
directory 접근을 직렬화하고 일반 fingerprint 규칙으로 변경된 crate만 무효화합니다.

구현 중에는 범위를 좁힌 incremental check를 사용합니다. 최종 source와 문서가 준비된 뒤
`npm run tauri:build`와 `npm run test:desktop`을 각각 한 번 실행합니다.
