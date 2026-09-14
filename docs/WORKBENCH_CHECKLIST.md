# Workbench interface checklist

**Updated:** 2026-09-12
**Scope:** Current schema-8 Task Workbench. The earlier Capture → Problem → Solution checklist is historical migration evidence, not current product behavior.

## Current Task capabilities

- [x] Capture and direct Task entry create canonical records without a mandatory stage sequence.
- [x] A Task can start, retain Work Log evidence, comments, checklists, decisions, relationships, and completion evidence independently of an optional Problem link.
- [x] Refinement is optional and preserves its own workspace for retry and return.
- [x] Conflict review is asynchronous Task evidence; it neither approves nor blocks Task work.
- [x] Task completion, Problem resolution, Knowledge drafting, and Knowledge publication are separate user decisions.
- [x] AI setup, Queue recovery, Vault search, Compass, and scoped local MCP remain available from their primary surfaces.

## Evidence and pending work

The native Task-centered baseline was exercised with a signed packaged macOS run on 2026-09-12. Its exact package identity and final scenario/control totals belong in the generated desktop-E2E artifacts, not this checklist.

The UI/UX follow-up implements six-field Task-definition draft protection across independent mutations, guarded close/Task switching, responsive shortcut and title-input geometry, selected-locale system copy, modal focus/Escape/IME behavior, and Queue setup recovery. The final package passed 32/32 scenarios with 188 scanned source controls and 175 rendered/exercised/asserted controls, with all six gap arrays empty. Nine final screenshots were visually reviewed; real Korean IME, VoiceOver, and OS reduced-motion remain unverified. It does not claim autosave or app-quit/crash draft durability.

See [specification 012](../specs/012-task-centered-workbench/spec.md) and the [UI/UX plan](plans/ui-ux-improvements.md).
