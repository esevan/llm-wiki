# Task-centered architecture backlog

**Recorded:** 2026-09-08

This is follow-up architecture work, deliberately deferred from the Task-centered Workbench
behavior task. It must be planned and implemented independently. The current task remains focused
on functional fixes and scenario verification; deferring this refactor does not waive those checks.

1. Consolidate duplicated Task mutation paths, including service and refinement SQL.
2. Replace handwritten DTO and operation registries that can drift from their contracts.
3. Define a retirement boundary for legacy DOM guards and runtime surfaces.
4. Split the oversized Task assistance module by responsibility.
5. Isolate migration-recovery responsibilities from ordinary application workflows.

The LLM Wiki plugin backlog capture was attempted for this list, but connector elicitation was
required and no remote record was saved. This local document is the authoritative pending handoff
until connector elicitation support is available.
