# API Contract: Fast Vault Search

- `GET /api/health`: local index and semantic-row status.
- `POST /api/index`: incremental structural reindex.
- `GET /api/search?q=&limit=&offset=&semantic=`: paginated structural search; `semantic=true`
  triggers background embedding work if needed and reranks once candidate vectors are available.
- `GET /api/events`: SSE notification after vault filesystem changes are reindexed.
