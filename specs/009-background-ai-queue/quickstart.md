# Quickstart Validation: Background AI Queue

## Baseline and migration

1. Run the full API suite before moving an endpoint and preserve characterization results.
2. Enable synchronous Job recording; verify response, failure, locale, source protection, and side effects remain unchanged while exactly one terminal synchronous Job is recorded.
3. Run the same handler through compatibility and worker drivers and compare results.
4. Convert to `202 + job_id`; verify [jobs-api.md](contracts/jobs-api.md) while shared behavior assertions remain unchanged.

## Process and throttling

1. Start two web processes, one Fast worker, and two durable workers.
2. Submit overlapping Chat requests; provider concurrency must never exceed one and no durable Job/notification is created.
3. Saturate durable work and verify Chat still starts.
4. Terminate Fast work while queued/running; the Interaction fails locally without durable recovery.

## Durable fail-safe

1. Terminate durable workers after claim, checkpoint, staged result, publication, and notification boundaries.
2. Restart and verify one completion, retryable/failed, stale, or cancellation outcome with no duplicates.
3. Race cancellation/source edits against completion; late output must not publish.
4. Submit one idempotency key through different web processes; only one equivalent Job/publication results.

## Translation and embedding

1. Interrupt paragraph translation and document embeddings; unchanged checkpoints resume, changed units rerun.
2. Restart between Vault replace and Job cleanup; reconciliation must retain the file and finish cleanup.
3. Capture/Work Log persistence returns without waiting and enqueues derived translations without overwriting source.
4. Close or switch an active Knowledge reader and verify its durable translation continues in the
   Queue; reopen the document and reuse its completed result.
5. Lexical search remains available during incomplete embeddings.

## Browser

1. Exercise the bottom-right Queue for every visible class and its task-specific destination.
2. Verify Draft/Refine inline completion and close/supersede cancellation.
3. Verify Image Summary preserves scroll and its result link opens the exact Work entry.
4. Verify Completion Review produces one toast, one unread bell item, and correct result navigation.
5. Verify keyboard use, textual status, focus preservation, reduced motion, and Korean `사용자` terminology.

## Stable checks

Run separately:

```bash
uv run pytest -q
```

```bash
node -e "const fs=require('fs'); const s=fs.readFileSync('llm_wiki/static/index.html','utf8').match(/<script>([\\s\\S]*)<\\/script>/)[1]; new Function(s); console.log('browser script parses')"
```

```bash
git diff --check
```
