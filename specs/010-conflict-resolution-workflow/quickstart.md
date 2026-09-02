# Quickstart: Validate Conflict Resolution Workflow

## Scenario 1: Structured multi-conflict review

1. Start Conflict Review from a proposed Solution and open its completed Queue result.
2. Confirm each conflict has its own card and textual High/Medium/Low severity.
3. Confirm the top comparison states the current claim and existing target/claim.
4. Expand evidence and verify citation and excerpt.
5. Confirm the footer reports the exact total and every card begins unresolved.

Expected: cards are independently scannable and raw Markdown is not the primary presentation.

## Scenario 2: Rationale and Continue validation

1. Select Apply recommendation on one conflict.
2. Select Accept conflict on another and leave its rationale blank.
3. Confirm the accepted card remains invalid and Continue is disabled.
4. Add rationale and confirm the counts change.
5. Resolve all remaining cards and continue.

Expected: one action exists per card; accept requires rationale; persistence occurs only when every conflict is valid.

## Scenario 3: Gate behavior

- Accept every conflict with rationale. Expected: explicit accepted history is stored and the existing clear gate permits the normal workflow.
- Apply at least one recommendation. Expected: history is stored, the Solution remains conflicted, and revision plus a fresh review is required.

## Scenario 4: Persistence and compatibility

Restart the app and reopen a saved review, then open a legacy findings-only fixture.

Expected: saved actions, rationales, timestamps, targets, and evidence remain; the legacy result remains readable without fabricated persistence.

## Scenario 5: Conflict-free review

Run a review with adequate coverage and no conflicts.

Expected: the concise existing clear/conflicted human decision path remains with no empty cards.

## Verification commands

```sh
uv run pytest -q
node -e "const fs=require('fs'); const s=fs.readFileSync('llm_wiki/static/index.html','utf8').match(/<script>([\\s\\S]*)<\\/script>/)[1]; new Function(s); console.log('browser script parses')"
git diff --check
```
