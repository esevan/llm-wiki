# Korean and English localization

**English** | [한국어](bilingual-localization.ko.md)

> **Resume where you left off.** Language changes preserve the current surface, input, and workflow
> lineage instead of making the user start over.

LLM Wiki supports Korean and English throughout the application. The language control is available
from every primary surface. A change updates interface text immediately, keeps the current view and
unsaved input in place, and is remembered for later sessions. Before the user makes an explicit
choice, a Korean environment selects Korean; other and unsupported environments use English.
An explicit choice is stored in the local application database. On the next connection, LLM Wiki
restores that choice before loading locale-sensitive stored content, so the first rendered workbench
does not briefly use a different language. The startup preference read bypasses browser caches so
the database remains authoritative. Packaged language resources are refreshed on each app
load so newly deployed interface text does not appear as an untranslated resource key.

## How each kind of content behaves

| Content | Korean and English behavior |
| --- | --- |
| Menus, controls, guidance, and status text | Switch immediately from the packaged language resources. Missing Korean text falls back to English. |
| New AI-generated Problems and Solutions | The reviewed item stores Korean and English versions together. Later switching reads those stored versions without another AI request. |
| New AI Image Summaries | One explicit AI request creates and stores Korean and English descriptions of the same Work Log image. Later switching reads the matching stored summary without another AI request. |
| Existing records and Vault files | Remain unchanged. If the selected-language version is missing, LLM Wiki shows the stored original and does not translate it automatically. |
| Live AI conversation and reviews | Use only the language that was active when the request started. Existing responses are not regenerated after a switch. |
| App-managed Knowledge | Keeps English Markdown as the canonical portable record. Korean reading is produced on request and may be reused only while that exact English source is current. |

Raw Capture text, manual entries, Work Log bodies, comments, checklist items, file names, code,
identifiers, citations, and quoted source material remain as authored. Only the explicitly requested
AI description of a Work Log image is bilingual. A user can add or revise a missing stored language
version without changing the item's identity or lineage.

## Knowledge and failure safety

Korean Knowledge is a reading aid, not a replacement for the English canonical Markdown. Changing
the canonical source invalidates reuse of its earlier Korean translation. Legacy or unmarked Vault
files are displayed unchanged in either application language.

Completed Korean readings are atomically stored at `Translations/ko/<canonical-path>`. Their
frontmatter links back to the canonical note and records the exact source path and hash, locale,
model, and generation time. LLM Wiki excludes this complete directory from canonical search and
indexing. App-managed canonical changes remove the derived file immediately; watcher cleanup removes
it after an external canonical change or deletion. Matching legacy SQLite cache entries are promoted
to this file form when first reused.

Opening Knowledge first renders the exact-hash Korean cache when one exists, or the English
canonical Markdown without waiting for AI. On a cache miss, a high-contrast status stays pinned
above the document and reports completed and total paragraphs. Each complete Korean paragraph
replaces its whole English paragraph with a clearly visible, roughly 900 ms left-to-right wave:
English recedes in reading order while Korean is revealed behind it. Text is never replaced token
by token or shown as a typing animation. People who prefer reduced motion receive an immediate
whole-paragraph replacement. Opening another
document or closing the reader aborts the browser stream and cancels its server translation job.
Failures leave the English canonical paragraphs readable and provide an explicit retry action.

If bilingual generation or Korean Knowledge translation fails, the successful original and the
canonical Markdown remain available. LLM Wiki identifies the missing version or falls back to the
canonical content; retry and manual correction remain user-controlled.

Existing monolingual Image Summaries are not migrated. They continue to appear in their stored
language, and a failed or malformed new bilingual summary never overwrites the prior summary.

Related Spec Kit record: [007 — Korean-English Localization](../../specs/007-bilingual-localization/spec.md)
