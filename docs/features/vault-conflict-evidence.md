# Evidence-rich Vault conflict review

**English** | [한국어](vault-conflict-evidence.ko.md)

Conflict review now shows what it is doing while it runs. The review panel reports the number of indexed Vault documents, current embedding coverage, merged candidate count, retained/reviewed progress, current phase, and separate search, screening, and strong-review timings.

![The Workbench keeps an unresolved Solution visible and exposes an explicit Conflict review action](images/02-workbench-board.png)

![Conflict review is queued without blocking the Workbench](images/09-background-job-queued.png)

The service searches lexical and semantic evidence independently across the Vault. It splits the Solution into reviewable claims, combines candidate passages, removes duplicate Raw/canonical records, and keeps exact path, line range, source hash, and passage text. Potential conflicts appear as soon as they are validated, without waiting for every remaining candidate.

Semantic indexing and review searches share one loaded embedding model. A review embeds all Solution claims in one batch, then reuses the existing model session for later searches. This lets large Solutions move out of the search phase without repeated model-startup or per-claim inference delays. If a Vault watcher removes a document during the search, that stale candidate is skipped instead of failing the whole review.

## Result meanings

- **Reviewing**: candidates remain, so clear is unavailable.
- **Potential conflict**: at least one exact cited passage may contradict a Solution claim.
- **No conflict found**: nothing has been found yet, but the review cannot claim absence.
- **Clear**: embedding coverage is complete, evidence was found, and every retained candidate finished without a conflict.
- **Insufficient evidence**: coverage, candidates, model output, or citations cannot support clear.
- **Cancelled/failed**: review stopped without a reliable recommendation.

Fast screening may remove only an explicit, well-formed non-conflict. Missing or ambiguous screening output stays in the strong-review queue. Browser cancellation is also sent to the server, preventing later model calls in that run. Unchanged Solution and Vault hashes reuse a completed review; changes invalidate it.

AI output remains evidence, not authority. Only a person may declare the Solution clear or conflicted and advance workflow state.

See [specification](../../specs/008-vault-conflict-evidence/spec.md) and [API contract](../../specs/008-vault-conflict-evidence/contracts/conflict-review-api.md).
