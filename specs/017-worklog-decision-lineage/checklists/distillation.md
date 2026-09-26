# Distillation Requirements Quality Checklist

**Purpose**: Review evidence fidelity, decision lineage, incremental update, and UI requirement quality before implementation
**Created**: 2026-09-26
**Audience**: PR reviewer
**Depth**: Standard release gate

## Requirement Completeness

- [x] CHK001 Are primary, corroborating, contradictory, missing-report, and deletion-tombstone source roles defined? [Completeness, Spec §FR-003–FR-005, FR-021]
- [x] CHK002 Are the meaningful facts that must survive Distillation enumerated, including reasons, failed approaches, conditions, values, paths, and unresolved items? [Completeness, Spec §FR-006]
- [x] CHK003 Are original-record preservation requirements defined for reports, evidence, manual content, decisions, completion, and legacy snapshots? [Completeness, Spec §FR-001, FR-030–FR-031]
- [x] CHK004 Are rules defined for Task completion, reopening, and repeated completion snapshots? [Completeness, Spec §FR-018]
- [x] CHK005 Are last-good, pending, stale, retryable-failure, repair-required, and unavailable states addressed? [Completeness, Spec §FR-023–FR-026]

## Requirement Clarity

- [x] CHK006 Is “Distillation” distinguished from shortening, raw transcription, and new advice? [Clarity, Spec §FR-002]
- [x] CHK007 Are attempted, completed, observed, and verified states distinguished with an explicit evidence threshold? [Clarity, Spec §FR-007, FR-017]
- [x] CHK008 Are add, enrich, merge, omit, and supersede outcomes defined so a new decision cannot overwrite an old one? [Clarity, Spec §FR-013–FR-014]
- [x] CHK009 Is current topic state resolved from explicit evidence rather than chronology? [Clarity, Spec §FR-015–FR-016]
- [x] CHK010 Is the boundary between feature-015 shared ownership and feature-017 semantic/projection ownership explicit? [Clarity, Spec §FR-032–FR-033]

## Requirement Consistency

- [x] CHK011 Are the readable Work Log and active Task journey required to use the same structured result without making their presentation identical? [Consistency, Spec §FR-011–FR-012, FR-027–FR-028]
- [x] CHK012 Are AI suggestion, user decision, performed work, evidence, and verification terms used consistently across requirements and acceptance scenarios? [Consistency, Spec §FR-017]
- [x] CHK013 Are incremental updates, explicit full repair, and rules/schema incompatibility requirements mutually consistent? [Consistency, Spec §FR-021–FR-025]
- [x] CHK014 Is active Task journey authority consistent with the explicit exclusion of legacy Solution lineage? [Consistency, Spec §FR-011, FR-031]

## Acceptance Criteria Quality

- [x] CHK015 Can claim citation fidelity and invented-fact absence be objectively measured? [Measurability, Spec §SC-001]
- [x] CHK016 Can epistemic-state preservation and routine-control filtering be measured with fixtures? [Measurability, Spec §SC-002–SC-003]
- [x] CHK017 Can targeted invalidation and stable identity be measured through unchanged unrelated nodes? [Measurability, Spec §SC-004]
- [x] CHK018 Can replay, stale completion, and last-good preservation be objectively measured? [Measurability, Spec §SC-005–SC-006]
- [x] CHK019 Are no-model-on-click and local projection performance outcomes quantified? [Measurability, Spec §SC-007, SC-010]

## Scenario and Edge-Case Coverage

- [x] CHK020 Are successful, failed, cancelled, interrupted, uncertain, missing-report, contradictory, and evidence-poor Runs covered? [Coverage, Spec §US1 and Edge Cases]
- [x] CHK021 Are repeated wording, multilingual sources, returning to an old option for a new reason, and unknown reasons covered? [Coverage, Spec §US2 and Edge Cases]
- [x] CHK022 Are concurrent edits, deletion, duplicate delivery, batching, retry, stale completion, and repair covered? [Recovery Coverage, Spec §US3 and Edge Cases]
- [x] CHK023 Are manual-only Tasks and histories containing legacy lineage addressed without making those sources authoritative? [Coverage, Spec §Edge Cases, FR-031]

## Non-Functional Requirements

- [x] CHK024 Are keyboard, focus, non-color status, narrow-window, long-value, and bilingual requirements specified? [Accessibility, Spec §FR-029]
- [x] CHK025 Are performance and model-invocation boundaries specified for reads and node detail? [Performance, Spec §FR-019, FR-035]
- [x] CHK026 Are privacy and human-authority boundaries explicit for Task state, decisions, Problem resolution, and Knowledge publication? [Governance, Spec §FR-030]

## Dependencies and Assumptions

- [x] CHK027 Is foundation integration a blocking prerequisite with contract and migration reconciliation rather than an assumed implementation detail? [Dependency, Spec §FR-032–FR-033]
- [x] CHK028 Are `capture_final_report`, completed evidence, and active Task journey source assumptions documented? [Assumption, Spec §Assumptions]

## Ambiguities and Conflicts

- [x] CHK029 Does the spec avoid treating the last chronological message as a final decision? [Conflict Prevention, Spec §FR-015]
- [x] CHK030 Does the spec prevent normal process exit, model assertion, or AI suggestion from being silently upgraded to verified/user-decided status? [Conflict Prevention, Spec §FR-007, FR-017]
