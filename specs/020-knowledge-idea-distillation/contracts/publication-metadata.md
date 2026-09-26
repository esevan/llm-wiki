# Contract: Feature 018/021 publication metadata handoff

Feature 020 supplies structured publication material; feature 021 chooses canonical paths and performs reviewed atomic file writes, organization, link maintenance, withdrawal, and indexing. Feature 018 reads the resulting metadata and computed content hash.

## Final Knowledge

```yaml
llm_wiki:
  schema: 1
  document_id: stable-knowledge-document-id
  source_revision: generation-snapshot-hash
  information_type: knowledge
  status: current
  applicability:
    summary: Supported use situation.
    representative_questions: []
    helps_with: []
    conditions: []
    exclusions: []
  decisions:
    - id: stable-topic-decision-id
      topic: approval-condition
      status: adopted
      confirmed_final: true
      successor_id: null
      reason: Source-supported reason or null.
      conditions: []
  provenance:
    task_id: stable-task-id
    task_revision: 7
    knowledge_revision: 4
    generation_snapshot_hash: sha256
    sources: []
    assumptions: []
```

`decisions[]` is canonical and contains independent stable topic units. `confirmed_final: true` is emitted only for that mapped feature 017 final/completion state. A singular `decision` alias is backward compatibility only. No document-wide status promotes sibling claims.

## Separate idea

```yaml
llm_wiki:
  schema: 1
  document_id: stable-idea-document-id
  source_revision: idea-content-provenance-hash
  information_type: idea
  status: deferred
  applicability:
    summary: Situation in which revisiting may help.
    representative_questions: []
    helps_with: []
    conditions: []
    exclusions: []
  idea:
    disposition: out_of_scope
    reconsideration_conditions: []
    related_task_id: stable-task-id
    related_knowledge_document_id: stable-knowledge-document-id
    sources: []
```

| Feature 020 disposition | Feature 018 `status` | Preserved/displayed exact field |
| --- | --- | --- |
| `unverified` | `unverified` | `idea.disposition: unverified` |
| `deferred` | `deferred` | `idea.disposition: deferred` |
| `out_of_scope` | `deferred` | `idea.disposition: out_of_scope` |
| `rejected` | `rejected` | `idea.disposition: rejected` |

The UI and feature 021 use exact `idea.disposition`; top-level `status` is a feature 018 ranking compatibility field.

## Hash authority and invariants

- The computed SHA-256 of actual file content is the authoritative current evidence revision. Declared `source_revision` and provenance hashes explain derivation only.
- The exact reviewed `body_markdown` is serialized without model regeneration.
- Ideas are separate files/units and never appended to final Knowledge.
- Publishing final Knowledge does not publish idea units unless the user explicitly selected each one.
- Feature 021 returns stable identity, path, and computed hash only after its atomic write/organization succeeds.
- Feature 020 records that result but does not choose `idea/`, canonical paths, MOCs, tags, aliases, or backlinks.
