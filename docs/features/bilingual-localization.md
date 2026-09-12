# Korean and English localization

**English** | [한국어](bilingual-localization.ko.md)

> **Resume where you left off.** Language changes preserve the current surface, input, and workflow
> lineage instead of making the user start over.

![The Workbench uses the selected system language while preserving authored content](images/workbench-tasks.en.png)

LLM Wiki supports Korean and English throughout the application. The language control is available
from every primary surface. A change updates interface text immediately, keeps the current view and
unsaved input in place, and is remembered for later sessions. Before the user makes an explicit
choice, a Korean environment selects Korean; other and unsupported environments use English.
An explicit choice is stored in local application settings and restored for later sessions.

## How each kind of content behaves

| Content | Korean and English behavior |
| --- | --- |
| Menus, controls, guidance, and status text | Switch immediately from the packaged language resources. Missing Korean text falls back to English. |
| Task and Problem records | Keep their authored or generated stored content. A language change does not rewrite a record or make a new AI request. |
| AI Image Summaries | Keep the stored summary. A language change does not rewrite authored evidence or automatically request another summary. |
| Existing records and Vault files | Remain unchanged. If the selected-language version is missing, LLM Wiki shows the stored original and does not translate it automatically. |
| Live AI conversation and reviews | Use only the language that was active when the request started. Existing responses are not regenerated after a switch. |
| Eligible managed Knowledge | A Korean reading is available only for an English Markdown note marked `llm_wiki_managed: true` and `canonical_locale: en`. It may be reused only while that exact English source is current. |

Raw Capture text, manual entries, Work Log bodies, comments, checklist items, file names, code,
identifiers, citations, and quoted source material remain as authored. The interface language does
not change an item's identity or lineage.

## Knowledge and failure safety

Korean Knowledge is a reading aid, not a replacement for the English canonical Markdown. Changing
an eligible canonical source invalidates reuse of its earlier Korean translation. Legacy or unmarked
Vault files, including Task-published Markdown that does not carry both managed-English markers,
are displayed in their original form.

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
whole-paragraph replacement. Opening another document or closing the reader detaches that reader
from progress updates but does not cancel durable translation. The job continues in the Queue and
its completed reading is reused when the document is reopened. Explicit Queue cancellation remains
available. Failures leave the English canonical paragraphs readable and provide an explicit retry
action.

If bilingual generation or Korean Knowledge translation fails, the successful original and the
canonical Markdown remain available. LLM Wiki identifies the missing version or falls back to the
canonical content; retry and manual correction remain user-controlled.

Existing monolingual Image Summaries are not migrated. They continue to appear in their stored
language, and a failed or malformed new bilingual summary never overwrites the prior summary.

Related Spec Kit record: [007 — Korean-English Localization](../../specs/007-bilingual-localization/spec.md)
