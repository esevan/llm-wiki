# Data Model: Korean-English Localization

## Application Locale Setting

| Field | Meaning | Validation |
|---|---|---|
| locale | Active application language | Exactly `ko` or `en` |
| explicit | Whether the user selected the value | Boolean; false permits first-run browser-language resolution |
| updated_at | Last explicit change | Local database timestamp |

The setting is a singleton. An explicit choice survives restarts. Without one, `ko-*` browser environments resolve to `ko`; all others resolve to `en`.

## Localized Content Version

| Field | Meaning | Validation |
|---|---|---|
| entity_type | Stable workflow entity namespace | Registered localized entity type |
| entity_id | Existing logical item identity | Must refer to the owning workflow item |
| field_name | Registered user-facing field | IDs, states, paths, hashes, code, and binary data are forbidden |
| locale | Stored version language | `ko` or `en` |
| value | Stored localized value | Same field limits as its base workflow field |
| origin | How the version was supplied | `ai` or `user` |
| source_hash | Optional hash of source version | Empty for independently authored values |
| created_at / updated_at | Audit timestamps | Database timestamps |

Primary key: `(entity_type, entity_id, field_name, locale)`.

State rules:

1. Legacy rows have no localized-content rows and always return base-column originals.
2. A reviewed bilingual draft writes both locales for every registered field in one transaction with the base workflow write.
3. A manually supplied version may add or replace one locale without changing identity, lineage, or the other locale.
4. Reads overlay only requested-locale fields; absent fields use the unchanged base value and report fallback metadata.
5. Language switching never creates or changes a localized-content row.

## Bilingual Durable Draft

| Field | Meaning | Validation |
|---|---|---|
| entity_type | Result being proposed | `problems` or `features` |
| source_locale | Locale active when generation began | `ko` or `en` |
| ko | Korean structured field object | Exact entity schema and field limits |
| en | English structured field object | Exact entity schema and field limits |
| reviewed_locale | Variant shown for human editing | `ko` or `en` |

Both objects validate before the draft is offered as successfully bilingual. Applying it preserves the existing explicit human action.

## Localized Image Summary

The existing Work Log progress entry remains the identity and continues to own its image, user-authored body, and compatibility summary value. Localized summary versions use the shared localized-content version model with `solution_progress_entries` as the entity type and `image_summary` as its only registered field.

State rules:

1. Existing progress entries with only a compatibility summary and no localized versions remain legacy and return that original summary in either locale.
2. A successful explicit summary request validates complete Korean and English summary strings before writing either locale.
3. Both localized versions and the compatibility summary selected for the request locale commit together.
4. A malformed response or provider failure leaves the image, prior summary, and any prior localized versions unchanged.
5. Progress reads overlay only the requested summary locale and never translate Work Log bodies, comments, or checklist items.

## Managed Canonical Knowledge

| Field | Meaning | Validation |
|---|---|---|
| path | Vault-relative Markdown path | Contained and normalized by MarkdownVaultAdapter |
| canonical_locale | Authoritative language | `en` for newly managed Knowledge |
| managed marker | Identifies translation-eligible files | Written only by approved publication flows |
| source_hash | Hash of complete canonical content | Recomputed on every read and remembered write |

Existing files without the managed English-canonical marker are legacy content.

## Knowledge Translation Derived File

| Field | Meaning | Validation |
|---|---|---|
| derived path | `Translations/ko/<canonical-path>` | Normalized by MarkdownVaultAdapter and excluded from canonical indexing |
| canonical | Obsidian link to the authoritative note | `[[<canonical-path-without-.md>]]` |
| source_path | Canonical Vault-relative path | Same normalized identity as canonical record |
| locale | Derived reading language | `ko` only |
| source_hash | Exact canonical revision translated | Must equal current full-content hash before serving |
| translated Markdown | Derived Korean reading body | Atomically written after complete translation |
| model | Provider model used | Non-secret string |
| generated_at | Cache audit timestamp | UTC timestamp in frontmatter |

The derived path is unique per canonical path. A source-hash mismatch is a cache miss. App writes and deletes remove the derived file eagerly; watcher cleanup removes files whose canonical source disappeared or changed externally. Matching legacy SQLite cache rows are promoted to this file representation on first read.

## Invariants

- Base legacy fields and files are never backfilled on startup, read, search, or locale switch.
- Machine state and lineage are shared across language versions.
- English canonical Knowledge is the only copy indexed and written to the Vault.
- Translation failure cannot replace canonical content or validate stale cache data.
- A canonical change during translation prevents the older result from being committed.
- Image Summary locale selection never invokes AI; only the explicit summarize action may create or replace its bilingual versions.
