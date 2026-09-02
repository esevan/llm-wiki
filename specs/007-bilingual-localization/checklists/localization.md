# Localization Requirements Quality Checklist

**Purpose**: Review Korean-English localization requirements as a formal pre-implementation and PR gate
**Created**: 2026-08-21

## Requirement Completeness

- [x] CHK001 Are all four content classes—static, system-generated durable, live dynamic, and Knowledge—defined with distinct language rules? [Completeness, Spec §FR-023]
- [x] CHK002 Are human-authored durable content and generated durable content explicitly distinguished? [Completeness, Spec §Assumptions]
- [x] CHK003 Are first-run locale selection, explicit persistence, unsupported locale fallback, and restart behavior all specified? [Coverage, Spec §FR-001–FR-005]
- [x] CHK004 Are visible, validation, empty, status, tooltip, and accessible static strings included in localization scope? [Completeness, Spec §FR-003]
- [x] CHK005 Are legacy database rows and legacy Vault files both covered by no-migration requirements? [Completeness, Spec §FR-010–FR-012]
- [x] CHK006 Are both application-managed and externally edited canonical Knowledge scenarios documented? [Coverage, Spec §FR-015–FR-019]

## Requirement Clarity

- [x] CHK007 Is “immediate” language switching quantified and separated from AI-backed operations? [Clarity, Spec §SC-001]
- [x] CHK008 Is the set of supported locales exact and free of ambiguous regional variants? [Clarity, Spec §FR-001, §FR-005]
- [x] CHK009 Is “original fallback” defined as unchanged stored content with no provider call? [Clarity, Spec §FR-010–FR-011]
- [x] CHK010 Is Knowledge translation-cache validity tied to exact canonical content rather than an ambiguous time window? [Clarity, Spec §FR-018–FR-019]
- [x] CHK011 Is request-start locale binding defined for in-flight live output? [Clarity, Spec §FR-013–FR-014]

## Requirement Consistency

- [x] CHK012 Do bilingual stored-content requirements remain consistent with the AI-free raw Capture and manual-entry assumptions? [Consistency, Spec §FR-007–FR-009, §Assumptions]
- [x] CHK013 Do English canonical requirements consistently prevent Korean cached content from becoming authoritative or entering the Vault? [Consistency, Spec §FR-015–FR-019]
- [x] CHK014 Do manual supplementation requirements preserve logical identity, lineage, and independent user ownership of each version? [Consistency, Spec §FR-012, §Assumptions]
- [x] CHK015 Are static fallback and legacy-content fallback rules distinct and non-conflicting? [Consistency, Spec §FR-006, §FR-011]

## Acceptance Criteria Quality

- [x] CHK016 Can provider-call absence during static switching, stored reads, and legacy fallback be objectively measured? [Measurability, Spec §SC-002–SC-004]
- [x] CHK017 Can canonical Knowledge language and cache invalidation be verified without relying on implementation internals? [Measurability, Spec §SC-005–SC-006]
- [x] CHK018 Is full primary-journey static-string coverage measurable in both languages? [Measurability, Spec §SC-007]
- [x] CHK019 Is preservation under provider and translation failure specified as a complete acceptance outcome? [Measurability, Spec §SC-008]

## Scenario and Edge-Case Coverage

- [x] CHK020 Are partial bilingual generation, missing translations, unsupported locale, and failed Korean Knowledge translation addressed? [Coverage, Spec §Edge Cases]
- [x] CHK021 Are concurrent canonical edits and late translation results addressed so stale cache cannot become current? [Recovery, Spec §Edge Cases]
- [x] CHK022 Are user-authored code, identifiers, paths, citations, and quoted evidence protected from unintended translation? [Coverage, Spec §FR-021]
- [x] CHK023 Are accessibility semantics, keyboard focus, open dialogs, navigation context, and unsaved input covered during switching? [Coverage, Spec §FR-002, §Assumptions]
- [x] CHK024 Are cross-platform, privacy, adapter, performance, and human-approval dependencies defined and aligned with the constitution? [Dependency, Plan §Constitution Check]
- [x] CHK025 Are AI-generated Image Summaries distinguished from user-authored Work Log evidence, with one-call bilingual storage, legacy fallback, and provider-free switching specified? [Consistency, Spec §FR-024–FR-026]
