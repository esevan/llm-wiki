# Localization Interface Contract

## Locale preference

### Read

`GET /api/settings/locale`

Returns `locale`, whether it is explicit, and supported locales. If no explicit value exists, the browser supplies its normalized candidate and the server resolves Korean only for `ko-*`, otherwise English.

### Save

`PUT /api/settings/locale`

Accepts exactly `{ "locale": "ko" }` or `{ "locale": "en" }`. Invalid values return a stable error code and do not change the prior setting.

Every browser API and streaming request sends `X-LLM-Wiki-Locale: ko|en`. The backend validates and binds it at request start.

## Static resources

`GET /api/i18n/{locale}` returns the packaged flat resource object for `ko` or `en`. Both resources have identical, non-empty keys. English is the missing-key fallback. Loading resources never invokes AI.

## Localized workflow reads

Board, current-detail, refinement-context, completed-Solution, progress, dashboard, and transition responses preserve machine fields and return user-facing text for the requested locale. Localized objects expose:

- `content_locale`: selected stored locale or original fallback;
- `available_locales`: stored versions present;
- `fallback_used`: whether the requested locale was absent;
- `localized_versions`: KO and EN structured variants when immediate client switching requires both.

A missing requested variant returns the base original and never invokes the provider.

## Bilingual draft and apply

Problem/Solution draft and refinement responses contain validated `ko` and `en` field objects plus source and review locale. One provider request creates both. Apply/create requests carry both reviewed variants; the server validates and stores them in the same transaction as the base workflow change.

Failure returns a stable error and preserves source items and prior drafts. Manual/offline paths remain available and may store only the authored locale with missing-version metadata.

## Live AI requests

Chat, next-stage chat, completed-Solution chat, organization, conflict review, and enrichment use one response-language instruction derived from the request-start locale. They perform no second-language generation and keep private run history in its generated language.

## Bilingual Image Summary

`POST /api/progress/{entry_id}/summarize-image` performs one provider request whose validated result contains complete `ko` and `en` summary values. The response returns the request-locale `summary`, `localized_versions`, and no missing locales. Both versions are stored against the existing progress-entry identity in the same transaction; the compatibility summary field stores the request-locale value.

`GET /api/features/{feature_id}/progress` returns the requested stored Image Summary locale with the same availability and fallback metadata used by other localized durable content. It never invokes the provider. Work Log bodies, comments, checklist items, images, identifiers, and timestamps remain shared and untranslated.

Existing progress entries without localized summaries return their compatibility summary unchanged with original-fallback metadata. A provider error or invalid bilingual result does not overwrite an existing summary.

## Knowledge read

`GET /api/knowledge?path={vault-relative-path}&locale={ko|en}`

The response includes `path`, `markdown`, `canonical_locale`, `served_locale`, `translated`, `cache_status`, `source_hash`, and an optional stable `warning_code`.

1. English requests for managed Knowledge return canonical Markdown directly with no provider call.
2. Korean requests for managed English-canonical Knowledge use a matching `Translations/ko` derived file or one translation request.
3. Korean requests for legacy/unmarked Vault files return the original unchanged with no provider call.
4. Provider failure returns English canonical content with a warning and never overwrites canonical or valid cache data.
5. Path traversal and paths outside the Vault are rejected through MarkdownVaultAdapter validation.
6. Translation preserves frontmatter, headings, code fences, link targets, identifiers, and quoted evidence; only readable prose changes.
7. The server re-reads canonical content after translation and caches only if its hash is unchanged.

The browser's first-render request uses `translate=false`. On a cache miss it returns canonical Markdown with `cache_status: pending` without a provider call. `GET /api/knowledge/translate?path=...&request_id=...` then emits newline-delimited JSON events for complete translated paragraphs (`paragraph`), final cache commit (`complete`), or recoverable failure (`error`). A complete result is atomically written to `Translations/ko/<canonical-path>` with canonical-link and source-identity frontmatter. This tree is excluded from canonical retrieval. `POST /api/knowledge/translation-cancel?request_id=...` sets the server cancellation token; a superseded or closed reader invokes it as well as aborting its stream.

## Backward compatibility

The existing provider-config `report_language` field remains accepted during transition but no longer controls managed canonical Knowledge. New publication is always English; application locale controls requested reading language. Existing files produced under the old setting remain legacy and are not rewritten.
