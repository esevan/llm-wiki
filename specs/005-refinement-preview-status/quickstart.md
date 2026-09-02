# Quickstart: Validate Refinement Preview Status

## Prerequisites

- Python 3.12 and project dependencies installed with `uv sync --all-extras`.
- Playwright Chromium installed for real-browser checks.
- Work from the dedicated feature worktree and branch.

## Automated validation

```sh
uv run pytest -q
node -e "const fs=require('fs'); const s=fs.readFileSync('llm_wiki/static/index.html','utf8').match(/<script>([\\s\\S]*)<\\/script>/)[1]; new Function(s); console.log('browser script parses')"
git diff --check
```

## Required UI state matrix

1. **Problem with context, generating**
   - Start refinement and hold proposal generation pending.
   - Expect the Preview to open with `Refine 중...` and the Problem’s bounded context summary.

2. **Solution with context, generating**
   - Repeat with a Solution.
   - Expect only that Solution’s context.

3. **Problem and Solution failure**
   - Force proposal generation to fail.
   - Expect no Preview, the refinement dialog to remain usable, and one corner warning.
   - Hover and keyboard-focus the warning; expect `Refinement preview를 띄울 수 없습니다. 다시 시도해 주세요.`

4. **Retry and stale-response isolation**
   - Start a retry or change items before an earlier request completes.
   - Expect the old warning to clear and late responses not to alter the current item’s UI.

5. **No previous context**
   - Start Problem or Solution refinement with title only and no history.
   - Expect existing button-level progress with no empty loading Preview.

6. **Capture regression**
   - Refine a Capture.
   - Expect no call to the refinement-context endpoint and no new loading Preview or corner-warning behavior.

7. **Human approval regression**
   - Complete a successful Preview.
   - Expect the proposed fields and bounded prior-context summary, with no data applied until the existing explicit approval action is used.

## Performance and boundary checks

- Verify summary selection is bounded to three same-item records and 500 visible characters.
- Verify the local context-summary unit path completes within its focused 100 ms budget.
- Verify no provider call occurs for the context endpoint and no dependency or schema is added.
