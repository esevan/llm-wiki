# Data Model: Task Work Sessions

## Task Work Session

- Stable `id`; required owner `task_id`; required title up to 200 characters.
- Provider catalog ID (`codex` in MVP), supported model ID (default `gpt-5.6-sol`).
- Approval mode (`ask` or `auto`) is future intent only.
- Workspace path is opaque text up to 4096 characters and is never traversed.
- Created/updated timestamps; Task/update index.

One Task owns zero or more sessions. Reads never create one.

## Session Record

- Stable `id`; required `session_id`; ordered by timestamp then ID.
- Author: `user`, `assistant`, `system`.
- Kind: `note`, `ai_output`, `execution_result`.
- Text plus optional attachment; text may be empty only with attachment.

The MVP writes only `user` + `note`; other values define the future extension boundary.

## Session Attachment

- Original name up to 255 characters, media type up to 200 characters, encoded app-managed content representing at most 10 MB input.
- No independent lifecycle; returned only through its Task-scoped session.

## Pending Composer Draft

- Task/session ownership key, text, optional attachment, stable append operation ID.
- Survives failures and in-screen navigation. Application-restart persistence of unsaved drafts is outside scope.

## Transitions and validation

- No session → explicit create → empty session.
- Session → explicit settings save → updated session.
- Pending composer → append confirmed → saved record and new composer.
- Append uncertain/failed → same composer and operation ID.
- Any session transition → Task state unchanged.
- Compound Task/session ownership, allowed catalog values, field lengths, nonempty record, and attachment limit are enforced.
