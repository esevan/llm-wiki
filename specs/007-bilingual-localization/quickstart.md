# Quickstart: Validate Korean-English Localization

## Prerequisites

- Configure an AI provider for bilingual durable generation and Korean Knowledge translation.
- Prepare one legacy database row and one pre-existing Vault Markdown file before starting the localized build.

## End-to-end scenarios

1. Start without a saved locale in a Korean browser environment; verify Korean. Repeat with an unsupported environment and verify English fallback.
2. Switch languages on Workbench, Search, Compass, AI Setup, an open dialog, and Explore. Verify static text, placeholders, tooltips, accessible names, document language, and dates update without reload or lost input.
3. Generate and approve a new Problem and Solution. Switch repeatedly and verify stored KO/EN fields change immediately with zero provider calls.
4. Open the legacy row in both locales and verify its original text is unchanged with no translation request. Add one language version manually and verify identity and lineage are unchanged.
5. Start Korean and English live chats. Verify each request generates one response in its request-start locale. Switch in flight and verify the original language remains bound.
6. Publish managed Knowledge while Korean is active. Verify the Vault Markdown is English and marked canonical. English viewing reads it directly.
7. Request uncached Korean Knowledge. Verify English canonical Markdown renders within one second with zero provider calls on that first response, a high-contrast completed/total status remains visible at the top while scrolling, and only whole completed paragraphs transition through an approximately 900 ms left-to-right English-out/Korean-in wave. Verify reduced-motion mode replaces the paragraph immediately. Open another document or close the reader mid-translation and verify the durable job continues in the Queue without updating the superseding surface. Reopen the original document and verify completed translation is an immediate cache hit. Retry a simulated failure, then edit canonical content externally and verify the next request does not use the prior cache.
   - After completion, inspect `Translations/ko/<canonical-path>` and verify its frontmatter canonical link, source path/hash, locale, model, and generation time. Verify the file is absent from search results and is removed after the canonical source changes or disappears.
8. Open an unmarked legacy Vault file in Korean and verify unchanged content and no AI call.
9. Simulate provider failure during bilingual drafting and Korean Knowledge reading; verify source content and canonical Knowledge remain unchanged.
10. Add a Work Log image and request its Image Summary in either locale. Verify one provider request stores Korean and English summaries, locale switching reuses them with zero provider requests, Work Log text remains unchanged, and an existing monolingual summary falls back without migration.

## Automated verification

Run each command separately from the task worktree:

```sh
uv run pytest -q
```

```sh
node -e "const fs=require('fs'); const s=fs.readFileSync('llm_wiki/static/index.html','utf8').match(/<script>([\\s\\S]*)<\\/script>/)[1]; new Function(s); console.log('browser script parses')"
```

```sh
git diff --check
```

Acceptance also records zero provider calls for locale switches, stored reads, and the uncached Knowledge first-render response; canonical or cached Knowledge appears within one second; paragraph replacement uses the visible 900 ms directional wave; a representative static switch stays within 100 ms; capture persistence is unchanged; and the existing 1,000-note index benchmark regresses no more than 15%.
