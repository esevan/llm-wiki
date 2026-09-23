# Design Readiness Checklist: Task-Centered Workbench

**Purpose**: Confirm planning artifacts are coherent and executable after analysis remediation
**Created**: 2026-09-05

## Governance and Supersession

- [x] Constitution 3.0.0 authorizes independent Task work and preserving migration
- [x] Product Spirit, adapter, performance, human authority, and publication gates pass
- [x] Every existing feature is classified in impact.md
- [x] Every affected historical spec and contract identifies the precise schema-8 replacement

## Architecture and Contracts

- [x] One Task aggregate serves native routes and MCP
- [x] Frontend DTO and request-bridge paths are explicit
- [x] Legacy baseline schema remains immutable; v8 uses a new schema include
- [x] Migration backup, validation, rollback, retry, and restore are specified
- [x] Capture-draft review works before Task persistence
- [x] Review currentness and false-clear rules bind exact identity
- [x] Completion, Problem resolution, and Knowledge publication are separate
- [x] Workbench order and canonical category behavior are explicit

## Implementation Readiness

- [x] Historical schema-8 baseline: all original 40 functional requirements mapped semantically to
  implementation and verification tasks
- [x] Historical schema-8 baseline: all original 9 measurable outcomes had verification work
- [x] All 8 user stories have an independent test
- [x] Historical schema-8 baseline: all original 77 tasks used valid sequential checklist format
  and concrete paths
- [x] 25 parallel opportunities are marked without overlapping dependent work
- [x] Final release build, packaged E2E, UI review, and fixed-corpus provider QA reuse one artifact
- [x] Missing provider configuration is reported as blocked rather than replaced by fallback output

## Analyze Remediation

- [x] T017 no longer edits the immutable legacy schema baseline
- [x] Request-bridge routing work is included in T001
- [x] All legacy Workbench board owners are included in T026
- [x] Localization work points to the actual JSON resource paths

## Notes

- [x] Recorded chronology and optional AI interpretation are separate layers; interpretation cannot
  mutate events or `followed_by` edges.
- [x] Details reads the current journey while Review reads the exact draft snapshot, falling back to
  current journey only when no draft snapshot exists.
- [x] Locale-bound caches and immutable draft snapshots avoid silently retranslating historical
  evidence when interface language changes.
- [x] Async persistence rechecks Task, completion, source, and journey currentness.
- [ ] Real-provider localized label/rationale UI review is complete for both English and Korean.

- Final post-remediation analysis: 0 critical, 0 high, 0 unresolved ambiguities, 0 uncovered buildable
  requirements. Historical specs remain evidence; schema-8 normative rules are linked explicitly.
