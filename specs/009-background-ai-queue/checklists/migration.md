# Requirements Checklist: Async Migration and Fail-safe

**Purpose**: Review requirement quality for the test-first, multi-process migration before implementation and PR acceptance
**Created**: 2026-09-01
**Feature**: [spec.md](../spec.md)

## Requirement Completeness

- [x] CHK001 Are all current provider and embedding call sites assigned to Fast or durable execution, including maintenance and fallback paths? [Completeness, Spec §Task and Completion Interface Matrix]
- [x] CHK002 Are the intentionally excluded operational provider checks distinguished from AI content work? [Completeness, Spec §Task and Completion Interface Matrix]
- [x] CHK003 Are synchronous compatibility, handler extraction, asynchronous submission, and compatibility removal each defined as distinct migration stages? [Completeness, Spec §Assumptions]
- [x] CHK004 Are source, target, model, locale, result-interface, notification, and idempotency identities defined for every durable task family? [Completeness, Spec §Key Entities]
- [x] CHK005 Are completion and cleanup requirements documented separately for SQLite-only, Vault, notification, and embedding publications? [Completeness, Spec §FR-016/FR-020/FR-026–FR-028]

## Requirement Clarity

- [x] CHK006 Is “one shared Fast throttler” unambiguous across multiple web processes and distinct from one throttler per process? [Clarity, Spec §FR-002a]
- [x] CHK007 Is the boundary between ephemeral Fast cancellation information and forbidden durable Fast status explicit? [Clarity, Spec §Fast Request]
- [x] CHK008 Are lease expiry, ownership loss, retryable recovery, and stale output distinguished clearly enough to avoid conflicting terminal states? [Clarity, Spec §FR-021/FR-026]
- [x] CHK009 Is “immediately enqueue” bounded to occur after successful source persistence without delaying the originating save? [Clarity, Spec §FR-017]
- [x] CHK010 Is the point at which Knowledge working rows may be removed defined by durable Vault reconciliation rather than provider completion alone? [Clarity, Spec §FR-016]

## Requirement Consistency

- [x] CHK011 Do Draft/Refine cancellation requirements remain consistent with their durable Queue visibility and no detached result history? [Consistency, Spec §FR-008–FR-009]
- [x] CHK012 Does automatic Image Summary attachment remain consistent with the user-approval boundary for workflow state and authored fields? [Consistency, Spec §FR-010/FR-023]
- [x] CHK013 Do multi-process worker configurability requirements preserve the fixed single Fast request rule? [Consistency, Spec §FR-002a/FR-029]
- [x] CHK014 Are translation requirements consistent about preserving authored Capture/Work Log source while exposing derived readings? [Consistency, Spec §FR-017–FR-018]
- [x] CHK015 Are Queue completion history and Knowledge Job deletion requirements consistent about where final results remain discoverable? [Consistency, Spec §Assumptions]

## Acceptance Criteria Quality

- [x] CHK016 Can API compatibility be objectively assessed before and after synchronous Job recording without treating approved `202` conversion as a regression? [Measurability, Spec §SC-017]
- [x] CHK017 Are cross-process Fast concurrency, durable duplicate publication, worker-loss recovery, and checkpoint reuse each assigned measurable success outcomes? [Measurability, Spec §SC-013–SC-016]
- [x] CHK018 Are enqueue responsiveness and Queue update latency defined with percentile, threshold, and representative-run context? [Measurability, Spec §SC-002–SC-003]
- [x] CHK019 Can scroll, focus, tab, and navigation preservation be observed consistently for every non-navigating result type? [Measurability, Spec §SC-004]

## Scenario Coverage

- [x] CHK020 Are primary, alternate, exception, recovery, cancellation, retry, stale-source, and duplicate-delivery scenarios represented? [Coverage, Spec §Edge Cases]
- [x] CHK021 Are process-loss scenarios defined at claim, checkpoint, staged-result, publication, and notification boundaries? [Coverage, Gap]
- [x] CHK022 Are browser disconnect, web-process loss, Fast-worker loss, durable-worker loss, and full application restart distinguished? [Coverage, Gap]
- [x] CHK023 Are zero-work, cache-hit, all-checkpoints-valid, partial-checkpoint, and source-changed paths covered for translation and embeddings? [Coverage, Spec §User Story 3/FR-028]
- [x] CHK024 Are notification read, dismiss, target deletion, replay, and duplicate-publication scenarios defined? [Coverage, Spec §User Story 4/Edge Cases]

## Non-Functional Requirements

- [x] CHK025 Are database contention, bounded retry, busy handling, and long-transaction prohibitions captured as reliability/performance constraints? [Coverage, Plan §Technical Context]
- [x] CHK026 Are same-host, cross-platform spawn, loopback-only internal transport, and private-input boundaries explicit? [Coverage, Plan §Technical Context; Contracts §Worker Roles]
- [x] CHK027 Are Queue and notification accessibility, localization, reduced-motion, and terminology requirements defined across all states? [Coverage, Spec §FR-024–FR-025]
- [x] CHK028 Are observability requirements sufficient to diagnose worker ownership, attempt, lease expiry, safe errors, progress, and publication without exposing secrets? [Coverage, Data Model §AI Job/Job Attempt]

## Dependencies & Assumptions

- [x] CHK029 Are the reasons for adopting HTTPX and asynchronous SQLite access documented against dependency-minimization principles? [Dependency, Plan §Complexity Tracking]
- [x] CHK030 Is the single-host SQLite WAL assumption explicit and consistent with the supported deployment model? [Assumption, Plan §Technical Context]
- [x] CHK031 Is the future addition of supported locales covered without requiring false translation work for the canonical locale? [Assumption, Spec §Assumptions]
- [x] CHK032 Are external AI provider cancellation limitations and late-result discard behavior documented? [Dependency, Spec §FR-022]

## Review Gate

- [x] CHK033 Are all critical product-behavior requirements traceable from the task matrix to functional requirements and success criteria? [Traceability]
- [x] CHK034 Are intentional legacy API contract changes listed explicitly before compatibility tests are retired? [Traceability, Spec §FR-030/SC-017]
- [x] CHK035 Are no unresolved terms or conflicting uses of “Queue,” “Fast Request,” “Job,” “completed,” or “applied” left in the artifacts? [Ambiguity]
