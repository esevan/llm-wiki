# Fast vault search

**English** | [한국어](fast-vault-search.ko.md)

> **Resume where you left off.** Search brings prior Knowledge back into the current decision.

![Search results show vault-relative paths and the matching note context](images/01-search-vault.png)

Search indexes Obsidian-compatible Markdown structure: paths, frontmatter, aliases, headings, nested
tags, wikilinks, references, embeds, and body text. Filesystem changes update the local SQLite FTS
index, and long result sets paginate without loading everything at once.

Semantic reranking is available when explicitly selected. Structural search remains a fast local
fallback during model or embedding failure, so existing Knowledge is never locked behind AI uptime.

Related Spec Kit: [001 — Fast Vault Search](../../specs/001-fast-vault-search/spec.md)
