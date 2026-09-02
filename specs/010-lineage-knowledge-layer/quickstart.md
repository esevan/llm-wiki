# Quickstart: Validate Lineage Knowledge

## Prerequisites

- Python 3.12+
- `uv`
- Playwright Chromium installed for browser checks
- A temporary vault and database; do not use personal Knowledge files for validation

## Scenario 1: Automatic four-stage lineage

1. Create a Capture with distinctive original feedback.
2. Promote it to a Problem with a refined statement and desired outcome.
3. Approve the Problem and create a Solution.
4. Record a conflict evaluation, approve the Solution, and add Work Log/checklist evidence.
5. Run completion review and confirm completion as the human.
6. Open the completed Solution and choose Lineage.

Expected:

- Capture → Problem → Solution → Complete appears without a second lineage approval.
- Each stage contains a source-backed snapshot.
- The final Markdown contains Detail, Lineage, Decision Changes, Conflicts & Addresses, and Completion Evidence.
- The Raw Data link and source evidence remain available.

## Scenario 2: Provenance and no-evidence behavior

1. Include one direct user statement, one explicit human workflow decision, and one plausible but unstated rationale.
2. Generate lineage and inspect provenance badges and evidence drill-down.

Expected:

- Source text is Observed and human approval/decision is Decided.
- Plausible rationale is absent with `Not explicitly recorded` or labeled `AI inferred` with High/Medium/Low confidence.
- Every assertion reaches an evidence excerpt; unknown evidence IDs are rejected.

## Scenario 3: Conflict address semantics

1. Record a conflicted review.
2. Clear it once without structured address evidence and inspect lineage.
3. Record a supported human address with basis, disposition, and evidence, then regenerate.

Expected:

- The first lineage shows Unclear or Unaddressed, never Addressed/Resolved.
- The supported version shows Addressed, disposition, address basis, and source link.
- Conflict appears on the affected transition, not as a fifth stage.

## Scenario 4: Problem navigation and evidence drill-down

1. Select the Problem card in the graph.
2. Return to the completed Solution and select a decision evidence link.

Expected:

- The correct Problem opens in the existing exploration workspace as read-only completed context.
- Evidence opens inside the completed workspace with its preserved excerpt.
- If the live record is unavailable, the snapshot remains readable and explains unavailable navigation.

## Scenario 5: Correct an AI interpretation

1. Open an inferred claim and save a correction.
2. Reload the completed Solution and final Markdown.
3. Open audit history for that claim.

Expected:

- The correction is current in UI and Markdown.
- The prior AI interpretation, timestamp, confidence, and evidence remain in audit history.
- Capture, decision, conflict, and completion evidence are unchanged.

## Scenario 6: Failure and retry

1. Complete a Solution while the model provider is unavailable.
2. Open Lineage and the final Markdown.
3. Restore the provider and request lineage regeneration.

Expected:

- Human completion succeeds.
- Deterministic four-stage lineage exists with `ready_without_inference` and an explicit retry state.
- Regeneration appends a snapshot version and does not overwrite the earlier audit record.

## Scenario 7: Final document uses Lineage as its source boundary

1. Complete a Solution whose Raw Data contains both material evidence and unrelated verbose history.
2. Capture the final-report model request and generate the completed-work Markdown.
3. Compare the request, stored projection metadata, Lineage claims, and rendered report.

Expected:

- The model request contains the selected Lineage projection and only evidence excerpts referenced by it.
- The request does not contain an arbitrary truncated prefix of Raw Data.
- The final document records the Lineage snapshot version used.
- Every factual report claim is supported by that snapshot or its evidence.
- The generated report is not present among the source evidence of the snapshot that generated it.

## Scenario 8: Final evidence citations are readable

1. Complete a Solution with a Capture, Work Log, validation checklist, and human completion decision.
2. Inspect the completion-report model request and generated Markdown.
3. Compare them with the private Lineage inference request.

Expected:

- The completion-report request and Markdown use labels such as `Original capture`, `Work log 1`, `Validation criterion 1`, and `Completion decision`.
- No snapshot, claim, evidence, revision, source-record, or workflow UUID appears in the completion-report request or reader-facing citation.
- The private inference request retains opaque evidence IDs, and inference validation still rejects citations outside its supplied bundle.

## Verification commands

Run these separately from the feature worktree:

```bash
uv run pytest -q
```

```bash
node -e "const fs=require('fs'); const s=fs.readFileSync('llm_wiki/static/index.html','utf8').match(/<script>([\\s\\S]*)<\\/script>/)[1]; new Function(s); console.log('browser script parses')"
```

```bash
git diff --check
```

Also record benchmark results for deterministic assembly and cached lineage reads against the budgets in [plan.md](plan.md).
