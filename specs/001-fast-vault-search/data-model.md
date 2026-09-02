# Data Model: Fast Vault Search

`documents` stores path, folder, title, body, frontmatter, headings, tags, aliases, source hash,
and modification time. `documents_fts` is the FTS5 index; `document_links` captures link graph
edges; `document_embeddings` stores float32 vectors keyed by path and source hash.
