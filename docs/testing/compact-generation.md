# Compact generation verification

Scope: new WorkPreviewFinalizer and KnowledgeDraft instructions, version 4.
Review interactions are maintained separately. The reported disabled Workbench
publication state remains unconfirmed; these changes do not claim to fix it.

## Reproducible size comparison

Run from the repository root:

```sh
node scripts/measure_generation_prompts.mjs fc02d9f
```

| Instructions | Before UTF-8 bytes | After UTF-8 bytes | Change |
| --- | ---: | ---: | ---: |
| KnowledgeDraft | 4,079 | 3,410 | -669 (-16.4%) |
| WorkPreviewFinalizer | 2,923 | 2,737 | -186 (-6.4%) |
| Combined | 7,002 | 6,147 | -855 (-12.2%) |

JSON-escaped instruction strings shrink by 897 and 191 bytes respectively.
Context is neither shortened nor summarized. The strict response schema remains
unchanged and supplies the structural contract formerly duplicated in prose.
Preview planning/final writing and Knowledge generation keep their existing call
counts; no retry or extra generation is added. This is a deterministic instruction
size measurement, not a measurement of provider tokens, generated output length,
latency or total request reduction percentage. No live-provider speedup is claimed.

## Retained content and safety

Preview fields keep their existing names/types. New writing avoids repeating the
work description in background, scope and initial approach; material requirements,
source bindings and assumptions remain. Existing Task fields must not be cleared
for brevity. Knowledge prose leads with the result and reuse situation, conditions
and limits; additional explanation remains when material. Structured applicability,
exact quotes, final outcomes, ideas and warnings remain in the existing contract.
No parser, migration, quality check or publication approval guard is relaxed.

## Search path and fixture coverage

`knowledge_archive::prepare_from_plan` writes the five applicability fields to
`llm_wiki.applicability`. `retrieval::parse_document_at_revision` creates separate
applicability units for summary, representative questions, helps-with, conditions
and exclusions. Vault indexing persists those units. Work preview retrieval uses
`vault::search`, retaining exact document versions/sections as references.

Native fixture tests index terms found only in each metadata field and search them
through the work-tracking reference API. A publication fixture additionally checks
that the exact approved bytes are written, the published pointer waits for index
receipts, and a representative question absent from body prose is retrieved from
the final published document. Legacy body-only notes remain searchable. Semantic
ranking quality is not benchmarked; lexical reference retrieval is verified.

Run the focused checks:

```sh
cargo test --manifest-path src-tauri/Cargo.toml workflow_foundation::
cargo test --manifest-path src-tauri/Cargo.toml native::knowledge_distillation::
cargo test --manifest-path src-tauri/Cargo.toml native::reference_aware_workbench::
cargo test --manifest-path src-tauri/Cargo.toml --test work_tracking_search
cargo test --manifest-path src-tauri/Cargo.toml archive_final_knowledge_uses_reviewed_bytes_and_defers_f_pointer_until_receipt
npm test
git diff --check
```

No review UI files changed. Packaged desktop E2E is skipped because the change is
prompt-only and local provider, strict-schema, semantic-validation, publication
and indexing fixtures cover the affected contracts. No production data is used.
