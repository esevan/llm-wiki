# Prompt Registry Contract

## Public identities

Current identities include `problem_enrichment`, `workflow_draft_capture`, `workflow_draft_problem`, `workflow_refinement`, `image_summary`, `completion_review`, `completion_report`, `workbench_organization`, `knowledge_translation`, `derived_translation`, `lineage_inference`, `task_journey`, `refinement_response`, `refinement_preview`, `conflict_review`, and `knowledge_draft`. Foundation identities reserve `capture_distillation`, `work_log_distillation`, `run_report_distillation`, `task_journey_increment`, `publication_index`, `speculative_search`, and `user_generation`.

Every definition exposes:

```rust
pub struct PromptDefinition {
    pub id: PromptId,
    pub version: u32,
    pub operation_kind: OperationKind,
    pub required_input: &'static [&'static str],
    pub required_output: &'static [&'static str],
    pub instructions: &'static str,
}

pub fn prompt_definition(id: PromptId) -> &'static PromptDefinition;
pub fn build_prompt(id: PromptId, input: &serde_json::Value)
    -> Result<BuiltPrompt, ContractError>;
pub fn validate_prompt_output(id: PromptId, output: &serde_json::Value)
    -> Result<(), ContractError>;
```

`build_prompt` validates required inputs, serializes context as data after registry-owned instructions, and returns prompt ID/version plus provider content. `validate_prompt_output` requires an object and validates definition fields before any durable apply.

The central lookup is `workflow_foundation/prompts/mod.rs`. Existing product prompts live in `prompts/current.rs`; foundation and downstream operation prompts live in `prompts/foundation.rs`. Later features may own another definition module and add one routing arm to the central lookup, avoiding simultaneous edits to one large prompt table.

## Version rule

Changing semantic instructions, input fields, output fields, evidence rules, or human-control constraints requires a version increment. Formatting-only Rust refactors do not.

## Provider rule

Callers may assemble domain snapshots, images, and exact evidence before registry construction. They may not bypass the registry for structured workflow output.
