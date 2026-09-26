# Contract: Search Metadata in Markdown

Metadata uses ordinary YAML front matter under one versioned namespace. Unknown keys are preserved
by the source file and ignored by this reader. A malformed or unsupported `llm_wiki` block produces
a bounded warning and body-search fallback.

```yaml
---
title: Example title
llm_wiki:
  schema: 1
  document_id: knowledge-example
  source_revision: optional-declared-provenance
  information_type: knowledge # knowledge | idea
  status: current # current | historical | unverified | deferred | rejected
  applicability:
    summary: Use this when choosing a local evidence index.
    representative_questions:
      - How can semantic matches work without exact keywords?
    helps_with:
      - local evidence retrieval
    conditions:
      - the Vault is the source of truth
    exclusions:
      - broad attachment indexing
  decisions:
    - id: decision-local-index
      topic: search-storage
      status: adopted # adopted | superseded | withdrawn | unresolved
      confirmed_final: true
      successor_id: null
      reason: Keeps private process local.
      conditions:
        - desktop local-first operation
---
```

The authoritative `source_revision` in evidence is always the computed content SHA-256. The declared
value above is provenance only. `decisions` is topic-scoped and may contain several independent
decision identities. The singular `decision` key is accepted as a compatibility alias for one item.
Final-decision ranking requires `confirmed_final: true` or a mapped C completion snapshot; `adopted`
alone does not convert a suggestion or the containing document into a final decision.

Recognized English and Korean applicability headings can add heading-bounded units for representative
questions, help, conditions, exclusions, and reuse guidance. The exact reuse forms include `When to
reuse`, `Reuse guidance`, `재사용하기 좋은 경우`, and `참조하기 좋은 경우`; a vague heading such as
`Reuse` is not reclassified. Heading recognition extracts written content; it does not infer absent
applicability or decision state.
