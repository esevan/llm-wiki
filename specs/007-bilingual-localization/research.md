# Research: Korean-English Localization

## Decisions

| Decision | Rationale | Alternatives considered |
|---|---|---|
| Persist a dedicated application locale (`ko` or `en`) in SQLite and use browser language only on first run | One product locale must consistently drive UI, API reads, and AI requests across restarts | `localStorage` alone splits state from the database; `report_language` has different legacy semantics |
| Send the active locale on every browser request | A request-start locale makes concurrent tabs and in-flight AI behavior deterministic | Reading only mutable server state can change a request after it starts |
| Keep static translations in paired JSON resources with identical keys | Resources are reviewable and testable without a frontend framework | Hard-coded ternaries are difficult to audit; an i18n dependency is unnecessary for two locales |
| Preserve base workflow columns and add a generic localized-field sidecar | Existing SQL and legacy data keep working; versions can be added per field | Per-table locale columns create schema churn; entity JSON weakens partial editing |
| Require bilingual generation for AI-produced Problem and Solution drafts, not raw human-authored input | This matches generated stored content while preserving AI-free Capture, manual transition, Work Log, and goal flows | Hard-gating every human save on AI breaks local-first behavior and can alter authored evidence |
| Produce both durable variants in one structured provider response and store them only after human approval | One call reduces drift and the preview/apply gate preserves human authority | Two calls can disagree or partially fail; background translation hides state |
| Store live chat/history in the one locale active at request start | Live content is transient and must not be duplicated | Backfilling chat adds calls and rewrites private history |
| Mark only newly app-managed Knowledge as English canonical | Existing Vault files remain legacy and are never normalized silently | Treating every Vault file as canonical English violates the no-migration boundary |
| Store Korean Knowledge readings at `Translations/ko/<canonical-path>` with source identity in frontmatter | The reading remains inspectable, linked, and portable while exact-hash validation prevents stale reuse; excluding the tree from retrieval avoids canonical duplication | SQLite is less inspectable and can retain opaque orphan rows; sibling files beside canonical notes blur authority |
| Keep canonical Knowledge search/indexing English in this feature | It preserves one portable index and bounds scope to viewing translation | Cross-language retrieval needs query translation or another index and is separate scope |
| Add a Knowledge read contract and browser reading surface | Search currently exposes only snippets and paths | Translating snippets alone does not satisfy Korean Knowledge viewing |
| Retire `report_language` as a canonical-output choice while accepting it for compatibility | Managed Knowledge must always be English; app locale controls reading language | Two language settings permit contradictory outputs |

## Content Classification Registry

| Class | Examples | Locale rule |
|---|---|---|
| Static | Navigation, buttons, labels, guidance, errors, transition descriptors | Paired resource keys; active locale; English missing-key fallback |
| System-generated durable | Approved AI Problem and Solution fields | Generate/store KO+EN together; select stored variant; no translation on read |
| Human-authored durable | Capture, manual forms, Work Log, comments, checklist edits, goals, evidence, citations, code, paths | Preserve authored text; optional manual localized sidecar; source fallback |
| Live dynamic | Chat, organization, reviews, image summary, enrichment | Generate once in request-start locale; never back-translate on switch |
| Managed Knowledge | Newly published projection/completion Markdown | English canonical; requested Korean translation in derived cache |
| Legacy Vault/saved data | Rows/files without localization or managed-canonical metadata | No migration or dynamic translation; original fallback |

## Risks and Mitigations

- The single HTML file has late renderer overrides. Resource parity and browser coverage must exercise both initial and overridden paths.
- Server transition descriptors and API errors can leak English. Localize descriptors and map stable error codes while preserving machine values.
- Bilingual draft payloads affect `ai_runs` restoration. Store validated localized payloads and return both reviewed variants.
- Canonical Knowledge can change during translation. Re-read and hash before cache commit; discard mismatched results.
- Use system Korean font fallbacks because the current remote Nunito import does not cover Hangul reliably.
- Locale switching and stored reads must be instrumented to prove they do not call the provider.

## Decision: Treat AI Image Summary as bilingual durable generated content

**Decision**: Keep Work Log bodies, comments, and checklist items in their authored language, but request Korean and English Image Summary variants together in the existing explicit summary provider call. Persist both variants against the progress-entry identity and use stored locale selection on reads.

**Rationale**: Image Summary already incurs an AI request and is reused as durable completion evidence. Returning both languages in that single request adds output work but no additional provider round trip, keeps later language switching provider-free, and does not require a background job queue.

**Alternatives considered**:

- Translate summaries on locale switch: rejected because it moves AI into a hot read path and makes switching slow and nondeterministic.
- Bind second-language translation to durable Queue work so it survives reader close, navigation,
  and web-process loss while retaining explicit cancellation through the Queue.
- Translate all Work Log content: rejected because user-authored evidence must remain exact and most entries do not otherwise require AI.
