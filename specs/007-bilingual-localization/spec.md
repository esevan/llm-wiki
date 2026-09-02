# Feature Specification: Korean-English Localization

**Feature Branch**: `007-bilingual-localization`
**Created**: 2026-08-21
**Last Reconciled**: 2026-09-02
**Status**: Current behavior reconciled — confirmed translation contract changes pending

## User Scenarios & Testing *(mandatory)*

### User Story 1 — Switch the application language (Priority: P1)

A user selects Korean or English and sees packaged interface text and supported dynamic status text
change without reloading the application. The explicit choice is reused after restart.

**Acceptance Scenarios**:

1. **Given** English is active, **When** the user selects Korean, **Then** the current primary view is
   translated and open text inputs retain their values.
2. **Given** an explicit saved choice, **When** the application restarts, **Then** the saved language
   is used.
3. **Given** no explicit choice, **When** the application opens, **Then** Korean browser environments
   use Korean and other environments use English.

### User Story 2 — Reuse stored language versions (Priority: P1)

AI-generated Problem and Solution proposals can store Korean and English fields under one workflow
identity. Later language switching selects stored fields without another AI request.

**Acceptance Scenarios**:

1. **Given** a successful bilingual Draft or Refine result, **When** the user applies it, **Then**
   both stored versions remain attached to the same Problem or Solution.
2. **Given** only one stored version, **When** another language is selected, **Then** the original
   base content remains readable rather than triggering an automatic migration.
3. **Given** an existing Problem or Solution, **When** the user manually supplies one locale's
   fields, **Then** that locale is added without changing the record identity or other locale.

### User Story 3 — Preserve authored Capture and Work evidence (Priority: P1)

Saving or changing a Capture, Work Log body, comment, or checklist item returns immediately and
schedules a derived Korean/English reading. The authored source remains unchanged.

### User Story 4 — Generate live output in the request language (Priority: P2)

Chat and other live single-language AI interactions use the locale active when the request begins.
Changing language later does not regenerate the existing response.

### User Story 5 — Read English-canonical Knowledge in Korean (Priority: P2)

Managed completed-work Knowledge remains authoritative English Markdown. A Korean user immediately
sees either an exact-revision Korean reading or the English canonical document while paragraph-based
background translation proceeds.

**Acceptance Scenarios**:

1. **Given** managed English-canonical Knowledge with no current Korean reading, **When** it is opened
   in Korean, **Then** canonical Markdown is shown immediately and a translation job is returned.
2. **Given** a matching Korean derived reading, **When** the same canonical revision is reopened,
   **Then** it is reused without an AI request.
3. **Given** canonical content changes, **When** Korean is requested, **Then** the prior derived
   reading is rejected and removed before it can be presented as current.

### User Story 6 — Read bilingual Image Summaries (Priority: P2)

An explicitly requested Work Log Image Summary produces Korean and English summaries in one request
and stores them on the same evidence entry. Language switching selects a stored summary without
resummarizing the image.

### Edge Cases

- Unsupported locale preferences are rejected without replacing the prior saved choice.
- Missing stored localized fields fall back field-by-field to the unchanged base content.
- Existing unmanaged Vault Markdown is never translated merely because it is opened in Korean.
- A Knowledge translation failure leaves untranslated English paragraphs readable and retryable.
- Closing or changing the Knowledge reader requests cancellation of its active translation job.
- Late translation output is rejected when its source revision no longer matches.
- Code, identifiers, paths, URLs, citations, and quoted evidence are instructed to remain unchanged
  in derived translations.
- Existing monolingual Image Summaries fall back to their stored original.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The application MUST support Korean and English interface locales.
- **FR-002**: An explicit locale choice MUST persist across restarts; without one, browser locale
  MUST select Korean for Korean environments and English otherwise.
- **FR-003**: Locale switching MUST update packaged interface resources without invoking an AI
  provider.
- **FR-004**: Locale switching MUST preserve current navigation and unsaved text input in supported
  primary journeys.
- **FR-005**: Packaged Korean and English resources MUST have the same non-empty keys and values.
- **FR-006**: AI-generated durable Problem and Solution proposals MUST support aligned Korean and
  English stored fields under one identity.
- **FR-007**: Applying a partially bilingual proposal MUST preserve the available version and expose
  any missing locale rather than discarding the proposal.
- **FR-008**: Reading stored workflow content MUST select the requested stored locale when present
  and otherwise fall back to unchanged base content.
- **FR-009**: Users MUST be able to supplement Problem and Solution localized fields without changing
  identity or lineage.
- **FR-010**: Existing records and unmanaged Vault files MUST NOT be rewritten or migrated solely by
  enabling or switching locale.
- **FR-011**: Live dynamic output MUST be generated only in the locale active when its request
  begins.
- **FR-012**: Managed Knowledge MUST use English canonical Markdown as its authoritative portable
  record.
- **FR-013**: Korean Knowledge reading MUST use only a derived version tied to the exact current
  canonical source revision.
- **FR-014**: A Korean Knowledge cache miss MUST display canonical content immediately and MUST
  schedule paragraph-based background translation only in response to an explicit Korean reading
  or translation request.
- **FR-015**: Completed Knowledge translation MUST be written as derived Markdown under the Korean
  translation area with source path, source revision, locale, model, and generation metadata.
- **FR-016**: Derived Korean Knowledge MUST be excluded from canonical search and removed when its
  canonical source disappears or changes.
- **FR-017**: Closing or superseding an active Knowledge reader MUST request server-side cancellation
  and MUST prevent its late result from replacing the current reader.
- **FR-018**: New or changed Capture, Work Log body, Work Log comment, and checklist source text MUST
  schedule a derived Korean/English reading after source persistence without delaying the save.
- **FR-019**: Derived Capture and Work readings MUST preserve the authored source and MUST NOT replace
  evidence or workflow state.
- **FR-020**: New Image Summary requests MUST create Korean and English summaries in one provider
  request and attach them to the requested image evidence.
- **FR-021**: Reading a stored Image Summary MUST use the selected stored version without another AI
  request and fall back to the original when unavailable.
- **FR-022**: Korean user-facing copy MUST use `사용자` rather than `인간` when referring to the
  product user.
- **FR-023**: Explicit Knowledge translation enqueue MUST use a state-changing request contract and
  MUST NOT use a read-shaped request.

### Key Entities

- **Locale preference**: The current supported locale and whether it was explicitly selected.
- **Localized durable fields**: Korean and English field values associated with one workflow record.
- **Derived content reading**: A generated locale view that does not replace authored source.
- **Canonical Knowledge**: Managed authoritative English Markdown.
- **Knowledge reading**: A Korean derived Markdown file bound to an exact canonical revision.
- **Localized Image Summary**: Korean and English descriptions attached to one image evidence entry.

## Success Criteria *(mandatory)*

- **SC-001**: Locale-switch browser tests retain current input and navigation and issue zero AI
  requests.
- **SC-002**: Stored bilingual workflow and Image Summary versions require zero AI requests during
  later language switches.
- **SC-003**: Legacy records and unmanaged Vault files remain byte-for-byte unchanged solely due to
  locale selection.
- **SC-004**: Every tested live request receives only the locale active at request start.
- **SC-005**: Stale Korean Knowledge is served in zero source-change tests.
- **SC-006**: Translation failure and cancellation tests preserve canonical and authored source in
  every case.
- **SC-007**: Packaged Korean and English resources pass key-parity and non-empty-value validation.

## Assumptions

- This is a single-user local application; locale preference is device-local.
- “Managed Knowledge” is explicitly marked English-canonical content produced by the application.
- Generated readings are navigation aids, not workflow approvals or replacements for authored
  evidence.
- Managed Knowledge translation is request-driven rather than proactively maintained for every
  document and locale.

## Confirmed Implementation Gaps

- **IG-007 — Translation after checklist edits**: Editing an existing checklist body currently does
  not enqueue replacement translation. A changed body must invalidate and replace its derived
  reading.
- **IG-018 — State-changing translation request**: Explicit Knowledge translation currently uses a
  read-shaped request. It must move to a state-changing request contract; any compatibility period
  must clearly mark the old contract deprecated.
