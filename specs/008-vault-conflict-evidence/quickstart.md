# Quickstart

Start the service with semantic dependencies and provider models, then review a Solution against a Vault containing a contradictory decision. Confirm the modal shows scope, coverage, counts, phase, timings, and an exact cited passage. Repeat with missing coverage and no candidates; both must be insufficient, not clear. Cancel a running review and confirm the server reports cancelled.

```bash
uv run pytest -q
node -e "const fs=require('fs'); const s=fs.readFileSync('llm_wiki/static/index.html','utf8').match(/<script>([\\s\\S]*)<\\/script>/)[1]; new Function(s); console.log('browser script parses')"
git diff --check
```
