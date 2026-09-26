# Contract: Portable Knowledge and idea Markdown

The complete UTF-8 bytes are produced during proposal preparation, shown in review, and written unchanged. YAML uses deterministic key order and escaping; Markdown line endings are normalized once before review. Publication never calls a model or reserializes the artifact.

## Knowledge artifact

```yaml
---
llm_wiki:
  schema: 1
  document_id: stable-knowledge-id
  source_revision: generation-snapshot-hash
  information_type: knowledge
  status: current
  aliases: []
  tags: []
  applicability:
    summary: Supported use situation.
    representative_questions: []
    helps_with: []
    conditions: []
    exclusions: []
  decisions: []
  provenance:
    task_id: stable-task-id
    task_revision: 7
    knowledge_revision: 4
    generation_snapshot_hash: sha256
    sources: []
    assumptions: []
  sections: []
---
Exact reviewed final body follows.
```

`sections` is optional and empty in this feature. If future mixed-content support populates it, each entry must carry stable section ID, explicit information type/status, and exact provenance. Current search trust never infers type solely from folder or heading.

## Idea artifact

```yaml
---
llm_wiki:
  schema: 1
  document_id: stable-idea-id
  source_revision: idea-provenance-hash
  information_type: idea
  status: deferred
  aliases: []
  tags: []
  applicability: {}
  idea:
    disposition: out_of_scope
    reconsideration_conditions: []
    related_task_id: stable-task-id
    related_knowledge_document_id: stable-knowledge-id
    sources: []
---
Exact reviewed idea body follows.
```

Each explicitly selected idea is a separate artifact under the archive's `idea/` canonical area. Exact disposition is `unverified`, `deferred`, `rejected`, or `out_of_scope`. The compatible top-level status follows feature 020/018 mapping and never hides `out_of_scope`.

## Source links

An optional reviewed section lists only actual-use and counterevidence sources:

```markdown
## Sources

- [[Knowledge/Source#Exact section|Readable source]] — Rationale for how this evidence was used.
- [[Knowledge/Counterpoint|Counterpoint]] — Counterevidence that limits the conclusion.
```

Wikilinks use validated vault-relative targets and safe section/label escaping. The rendered path is portable display. SQLite stores exact document ID, authoritative revision, section, role, rationale, and feature 019 interaction identity. A rename updates managed rendered links only through a reviewed patch; the stored historical identity remains unchanged.

Viewed, mentioned, candidate, adopted-without-use, and excluded references are never rendered as publication sources.

## Hash rules

- `source_revision` and provenance hashes describe derivation.
- `body_hash` audits the exact feature 020 body embedded in the artifact.
- SHA-256 of the complete actual file bytes is the authoritative current revision used by feature 018 and all external-edit guards.
- Paths, file hashes, and exact bytes are revalidated immediately before apply.

The Markdown remains readable and useful without LLM Wiki or Obsidian. It contains no application-only URI as its sole source label.
