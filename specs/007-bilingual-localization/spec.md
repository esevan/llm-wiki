# Feature Specification: Korean-English Localization

**Feature Branch**: `feat/bilingual-localization`

**Created**: 2026-08-21

**Status**: Draft

**Input**: User description: "Provide a natural Korean/English experience across LLM Wiki using Static → i18n, Stored → KO+EN, Dynamic → User Locale, and Knowledge → English Canonical principles."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Switch the application language (Priority: P1)

As a Korean- or English-speaking user, I can choose my preferred language and immediately use the application with navigation, controls, and guidance written naturally in that language.

**Why this priority**: A consistent language setting is the foundation for demonstrating and deploying every other localized behavior.

**Independent Test**: Start in either supported language, change the language setting, and verify that all visible static interface text changes without reloading or losing current work.

**Acceptance Scenarios**:

1. **Given** the application is open in English, **When** the user selects Korean, **Then** all static text on the current surface changes to Korean immediately and the current context remains intact.
2. **Given** a user has selected a language, **When** the application is reopened, **Then** the same supported language is used.
3. **Given** no language has been selected, **When** the application opens, **Then** it uses the supported language matching the user's environment when possible and otherwise uses English.

---

### User Story 2 - Reuse bilingual versions of newly saved content (Priority: P1)

As a user creating system-generated durable content, I receive both Korean and English versions at creation time so later language switches show stored content without another AI request.

**Why this priority**: Pre-generating durable translations makes demonstrations predictable and avoids translation drift during ordinary language switching.

**Independent Test**: Create a new durable item in either supported language, inspect it in both languages, and verify both versions were saved during creation and reused thereafter.

**Acceptance Scenarios**:

1. **Given** the user approves newly generated durable content, **When** creation completes, **Then** Korean and English versions are stored as one logical item and the version matching the current language is displayed.
2. **Given** both versions are stored, **When** the user switches languages repeatedly, **Then** the matching stored version is shown without an additional AI request.
3. **Given** bilingual generation is incomplete, **When** the item is saved, **Then** the successfully generated original content remains available and the missing-version state is disclosed without discarding user work.
4. **Given** a user requests an AI Image Summary for Work Log evidence, **When** generation succeeds, **Then** Korean and English summaries are created in the same AI request and stored with the image evidence.
5. **Given** a Work Log image has stored Korean and English summaries, **When** the user switches languages, **Then** the matching stored summary is shown without another AI request while the user-authored Work Log remains unchanged.

---

### User Story 3 - Preserve legacy content without migration (Priority: P1)

As a user with an existing Vault or saved records, I can continue using them unchanged, even when they do not have a version in my selected language.

**Why this priority**: Backward compatibility protects private data and keeps localization from becoming a migration prerequisite.

**Independent Test**: Open pre-existing monolingual records under both language settings and verify the stored original is shown unchanged with no automatic translation request or data rewrite.

**Acceptance Scenarios**:

1. **Given** existing content has only one language version, **When** the other language is selected, **Then** the original stored content is displayed without dynamic translation.
2. **Given** existing content has user-supplied Korean and English versions, **When** the language changes, **Then** the matching stored version is displayed.
3. **Given** localization is introduced, **When** existing Vault files and records are inspected, **Then** none were rewritten solely to add language versions.

---

### User Story 4 - Generate live content in the active language (Priority: P2)

As a user in an interactive AI flow, I receive transient responses only in my currently selected language.

**Why this priority**: Live conversation should feel natural while avoiding unnecessary duplicate generation that is not reused as durable content.

**Independent Test**: Run the same live flow in Korean and English and verify each response uses only the active language and performs no second-language generation.

**Acceptance Scenarios**:

1. **Given** Korean is active, **When** live dynamic content is requested, **Then** one Korean response is generated.
2. **Given** English is active, **When** live dynamic content is requested, **Then** one English response is generated.
3. **Given** the language changes after a live response, **When** the prior response remains visible, **Then** it is not retroactively translated or regenerated.

---

### User Story 5 - Read portable Knowledge in Korean (Priority: P2)

As a user publishing or reading Knowledge, I retain an English canonical Markdown source and can request a Korean reading version without changing the canonical file.

**Why this priority**: English canonical Knowledge remains portable and reusable while Korean readers still receive a natural experience.

**Independent Test**: Publish Knowledge from Korean input, verify the canonical Markdown is English, request Korean twice, then change the canonical source and verify the prior Korean translation is no longer reused.

**Acceptance Scenarios**:

1. **Given** Knowledge is created or refined from content in either language, **When** it is approved for publication, **Then** the canonical Markdown is stored in English.
2. **Given** Korean is active and English canonical Knowledge exists, **When** the user opens it, **Then** a Korean reading version is generated on request while the canonical Markdown remains unchanged.
3. **Given** a reusable Korean reading version exists, **When** the same unchanged English canonical Knowledge is requested again in Korean, **Then** the prior translation may be reused.
4. **Given** the English canonical Knowledge changes, **When** Korean is requested again, **Then** no translation associated with the previous canonical version is reused.

### Edge Cases

- Unsupported or malformed language preferences fall back to English without blocking use.
- Missing static translations fall back to the English resource and remain detectable during validation.
- A durable item with only one successfully generated version remains readable in that version and is not dynamically translated during language switching.
- User-authored text, file names, code, identifiers, citations, and quoted source passages are preserved unless their content type explicitly requires generation or translation.
- Language changes during an in-flight AI request do not alter the language contract of that request; later requests use the newly selected language.
- Korean Knowledge translation failure leaves the English canonical content readable and does not modify or remove it.
- Cached Korean Knowledge is reused only when it corresponds exactly to the current English canonical content.
- Concurrent canonical Knowledge changes cannot cause an older Korean translation result to become the valid cache entry for newer content.
- Existing monolingual Image Summaries are not migrated; when the selected-language version is absent, the stored original summary remains visible without an AI request.
- Failed or malformed bilingual Image Summary generation leaves the prior stored summary and image evidence unchanged.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The system MUST support exactly Korean and English as interface languages.
- **FR-002**: Users MUST be able to change the active language from every primary application surface without losing their current navigation or unsaved input.
- **FR-003**: The system MUST apply a language change to all static menus, buttons, labels, status text, validation messages, empty states, and guidance immediately.
- **FR-004**: The system MUST remember an explicit user language choice across application restarts.
- **FR-005**: When no explicit choice exists, the system MUST select Korean for a Korean environment and English for all other environments.
- **FR-006**: Static text MUST have centrally managed Korean and English versions, with English as the fallback for a missing resource.
- **FR-007**: The system MUST treat newly approved, system-generated durable workflow content as one logical item with Korean and English stored versions generated as part of creation.
- **FR-008**: Changing the active language MUST select a stored version of durable content and MUST NOT trigger translation or regeneration.
- **FR-009**: If bilingual generation of new durable content is partially unsuccessful, the system MUST preserve the successful content, expose that a language version is missing, and allow a user-controlled retry or manual addition.
- **FR-010**: The system MUST NOT automatically migrate, rewrite, or translate existing Vault files or saved records for localization.
- **FR-011**: For existing durable content without the selected-language version, the system MUST display the original stored content unchanged.
- **FR-012**: Users MUST be able to add or revise a missing language version of existing durable content without changing its logical identity or lineage.
- **FR-013**: Live dynamic content MUST be requested and generated only in the language active when the request begins.
- **FR-014**: A language switch MUST NOT translate or regenerate an already produced live response.
- **FR-015**: Knowledge creation and refinement MUST normalize approved canonical content to English Markdown regardless of the working language.
- **FR-016**: The English canonical Knowledge Markdown MUST remain the authoritative portable record and MUST NOT be replaced by a translated reading version.
- **FR-017**: Korean Knowledge MUST be translated only when requested for Korean viewing, and English Knowledge viewing MUST use the canonical source directly.
- **FR-018**: The system MAY reuse a Korean Knowledge translation only when it is tied to the exact current English canonical content.
- **FR-019**: Any change to English canonical Knowledge MUST invalidate prior Korean translation reuse before the changed content is presented in Korean.
- **FR-020**: AI or translation failure MUST preserve the user's original input, existing stored versions, canonical Knowledge, and human control over retry or manual correction.
- **FR-021**: Localization MUST preserve user-authored code, identifiers, citations, quoted evidence, and workflow lineage unless a requirement explicitly identifies the content as generated or translated.
- **FR-022**: Every primary workflow and surface MUST be included in localization validation; screen-by-screen implementation order does not reduce final scope.
- **FR-023**: The system MUST distinguish static, durable stored, live dynamic, and Knowledge content consistently according to the rules in this specification.
- **FR-024**: A newly requested AI Image Summary for Work Log evidence MUST produce Korean and English summaries in one AI request and store both versions with the existing progress-entry identity.
- **FR-025**: Reading or switching the language of a stored Image Summary MUST use the matching stored version without an AI request and MUST fall back to the unchanged original summary when a selected-language version is absent.
- **FR-026**: User-authored Work Log bodies, comments, and checklist items MUST remain in their authored language and MUST NOT be included in Image Summary bilingual generation.

### Key Entities *(include if feature involves data)*

- **Language Preference**: The user's active supported language, its origin (explicit choice or environment fallback), and the value retained for later sessions.
- **Localized Static Resource**: A stable text key with Korean and English values and an English fallback when one value is missing.
- **Localized Durable Content**: One workflow item identity and lineage with independently editable Korean and English versions plus their availability state.
- **Live Dynamic Content**: A transient generated result bound to the active language at request start and not duplicated for the other language.
- **Canonical Knowledge**: Human-approved portable Markdown whose authoritative content is English.
- **Knowledge Translation Cache Entry**: A Korean reading version associated with the exact identity and content revision of English canonical Knowledge; it is reusable only while that revision remains current.
- **Localized Image Summary**: One AI-generated description of Work Log image evidence with Korean and English stored versions attached to the same progress-entry identity; the image and user-authored Work Log text remain shared and unchanged.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A user can switch between Korean and English on any primary surface, see all static text update within one second, and retain current work and navigation.
- **SC-002**: In acceptance coverage, 100% of newly approved, system-generated durable content types store both Korean and English versions during a successful creation flow and require zero additional AI requests during language switching.
- **SC-003**: In a representative legacy-data fixture, 100% of existing Vault files and records remain byte-for-byte unchanged solely due to enabling or switching languages.
- **SC-004**: In acceptance coverage, 100% of live dynamic requests produce only one language version matching the language active at request start.
- **SC-005**: In acceptance coverage, 100% of newly published or refined Knowledge has English canonical Markdown, and Korean viewing never changes that canonical source.
- **SC-006**: After any canonical Knowledge edit, 100% of subsequent Korean views avoid reuse of translations produced from the prior canonical revision.
- **SC-007**: Every primary user journey can be completed in both Korean and English without encountering untranslated static interface text.
- **SC-008**: AI and translation failure scenarios preserve all pre-existing user content and provide a recoverable path in 100% of acceptance cases.
- **SC-009**: In acceptance coverage, 100% of successful new Image Summary requests make Korean and English stored summaries in one provider request and require zero additional provider requests during later language switching.

## Assumptions

- This is a single-user local application, so language preference is device-local rather than synchronized across accounts.
- "System-generated durable content" includes AI-produced Problem and Solution structured fields approved by a user and an explicitly requested AI Image Summary. Raw capture text, manually entered fields, Work Log bodies, comments, checklist items, quoted evidence, code, identifiers, and file names remain in their authored form and may be manually supplemented with another language version.
- Existing content is identified by whether it predates localization metadata or lacks localized versions; it is never upgraded merely by reading it.
- Stored bilingual versions may be edited independently by the user; the system does not silently synchronize manual edits by regenerating the other language.
- Korean Knowledge translation caching is enabled when safe reuse is available, but correctness depends on exact canonical-content identity rather than elapsed time.
- Accessibility semantics, keyboard operation, and focus are preserved when adding the language control and replacing text.
- The existing AI provider and privacy boundaries are reused; localization does not authorize sending additional private content to any new external service.

## Product Spirit Alignment

- **Reduce Cognitive Load**: A persistent global choice removes repeated language decisions, and automatic bilingual creation avoids manual translation work for new durable content.
- **Resume Where You Left Off**: Switching languages retains navigation, drafts, lineage, and existing source content.
- **Private Process, Portable Knowledge**: English canonical Markdown remains portable and human-approved; Korean translations are reading aids and never replace canonical Knowledge.
- **Human Authority over AI**: Partial generation and translation failures preserve originals and leave retry or manual correction under user control.
- The feature does not create a workflow stage, expose private process, rank a worker, or require users to classify content manually.

## Progressive Korean Knowledge Reading Addendum (2026-08-24)

- A Knowledge click MUST display either the exact-hash Korean cache or English canonical Markdown within one second; a cache miss MUST NOT put AI work on this first-render path.
- On a managed English-canonical cache miss, Korean translation MUST progress by completed Markdown paragraph. A high-contrast sticky status MUST keep completed/total progress visible while the document scrolls. Completed paragraphs replace whole English paragraphs with a clearly perceptible, approximately 900 ms left-to-right wave: the English layer recedes in reading order while the Korean layer is revealed in the same direction. Token and typing replacement are forbidden.
- Navigating to another document or closing its reader MUST cancel the prior server translation job, not merely ignore its eventual browser result.
- Translation failure MUST leave every untranslated English paragraph readable and offer an explicit retry.
- A completed progressive translation MUST populate the existing exact-canonical-hash cache and MUST still be rejected if canonical content changes during translation.
- Completed Korean readings MUST be stored as derived Markdown under `Translations/ko/<canonical-path>`. Their frontmatter MUST link to the canonical document and record `source_path`, exact `source_hash`, locale, model, and generation time. Derived translations MUST be excluded from canonical search/indexing, removed when the app changes or deletes their canonical source, and cleaned up when their source disappears or changes externally.
