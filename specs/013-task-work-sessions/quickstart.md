# Quickstart: Validate Task Work Sessions

## Focused checks

1. `npm test`
2. `cargo test --manifest-path src-tauri/Cargo.toml`
3. Run the focused 100 ms p95 projection profiling check.
4. `git diff --check`

Expected: isolation, retry, ownership, state invariance, persistence, performance, and whitespace checks pass.

## Rendered review

Inspect English and Korean at 1280 px and 640 px with empty, long-content, saved-file, pasted-image, and save-failure states. Keyboard through tabs, picker, composer, attachment, settings, and actions; focus remains visible and no page overflows.

## Smallest packaged scenario

After the final release build, run only `task-work-session-restart`:

1. Task A starts with zero sessions until explicit creation.
2. Create two Task A sessions with distinct records/settings and an image.
3. Task B exposes none of Task A's sessions; save one Task B record.
4. Relaunch the packaged application.
5. Reopen all three sessions and verify restoration/isolation.
6. Verify both Task states are unchanged.

This E2E is justified only by packaged restart/native persistence. Do not run the full suite absent a concrete broader failure.

## Verification record (2026-09-21)

- `npm test`: passed, including 39 Vitest files and 268 tests plus the repository's helper, provider, signing, runtime, and boundary checks.
- `cargo test --manifest-path src-tauri/Cargo.toml`: passed, including 93 library tests and every integration suite. The native workflow dispatch regression covers all five Task work-session operations.
- `npm run build`: passed and verified 148 bundled font subsets.
- `npm run test:desktop -- --scenario task-work-session-restart --keep-state`: passed after the final release build in 4.172 seconds. The full desktop suite was not run.
- `git diff --check`: passed.
- Rendered review used the isolated normal `com.llm-wiki.session-review` bundle. English and Korean Sessions views, restored image/settings, and a long mixed Korean/English note were visually inspected. Keyboard Tab navigation reached the file input and Space saved the record; the record was restored after a real app restart. The native picker selected a 400×160 PNG, but UI automation then lost AppKit access while both app processes remained healthy, so picker completion was not observed. Narrow, explicit error-state, and Windows rendering remain unverified.
