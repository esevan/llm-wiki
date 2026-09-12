import { useCallback, useEffect, useRef, useState } from "react";
import { taskClient } from "../../services/taskClient";
import type {
  ConflictReview,
  ConflictReviewHistory,
} from "../../types/taskWorkbench";
import { useTaskWorkbenchText } from "./taskWorkbenchText";

const taskSubject = (taskId: string, taskRevision: number) => ({
  kind: "task_revision",
  taskId,
  taskRevision,
});

function ReviewResult({ review }: { review: ConflictReview }) {
  const text = useTaskWorkbenchText();
  const status = {
    clear: text.reviewClear,
    findings: text.reviewFindings,
    insufficient_evidence: text.reviewInsufficient,
    queued: text.reviewQueued,
    running: text.reviewRunning,
    failed: text.reviewFailed,
    cancelled: text.reviewCancelled,
    stale: text.reviewStale,
  }[review.status] ?? review.status.replaceAll("_", " ");
  return (
    <>
      <strong data-review-status={review.status}>{status}</strong>
      {review.safeError && <p>{review.safeError}</p>}
      {review.findings?.map((finding) => (
        <article
          className="citation"
          key={finding.id}
          data-citation-path={finding.path}
        >
          <strong>{finding.path ?? finding.sourceId}</strong>
          <p>{finding.excerpt?.replace(/<\/?mark>/gi, "")}</p>
        </article>
      ))}
    </>
  );
}

export function ConflictReviewPanel({
  taskId,
  taskRevision,
}: {
  taskId: string;
  taskRevision: number;
}) {
  const text = useTaskWorkbenchText();
  const [history, setHistory] = useState<ConflictReviewHistory>({
    attempts: [],
  });
  const [error, setError] = useState("");
  const refreshSequence = useRef(0);
  const refresh = useCallback(async () => {
    const sequence = ++refreshSequence.current;
    try {
      const next = await taskClient.reviewHistory(
        taskSubject(taskId, taskRevision),
      );
      // An empty history is a valid response for a new subject.  Keep the
      // rendered panel usable if an older native endpoint omits attempts.
      if (sequence === refreshSequence.current)
        setHistory({ ...next, attempts: next.attempts ?? [] });
    } catch (e) {
      if (sequence === refreshSequence.current)
        setError(String(e instanceof Error ? e.message : e));
    }
  }, [taskId, taskRevision]);
  useEffect(() => {
    setHistory({ attempts: [] });
    setError("");
    void refresh();
  }, [refresh]);
  const active = history.attempts.some((review) =>
    ["queued", "running"].includes(review.status),
  );
  useEffect(() => {
    if (!active) return;
    const interval = window.setInterval(() => void refresh(), 350);
    return () => window.clearInterval(interval);
  }, [active, refresh]);
  const begin = async () => {
    try {
      setError("");
      await taskClient.review(taskSubject(taskId, taskRevision));
      await refresh();
    } catch (e) {
      setError(String(e instanceof Error ? e.message : e));
    }
  };
  const newest = history.attempts[0];
  return (
    <section
      className="task-panel review-panel"
      aria-label={text.review}
      aria-busy={active}
    >
      <header>
        <div>
          <small>{text.asyncReview}</small>
          <h3>{text.review}</h3>
        </div>
        {active && newest ? (
          <button
            type="button"
            data-control="conflict-review-cancel"
            onClick={() =>
              void taskClient
                .cancelReview(newest.id)
                .then(refresh)
                .catch((e) => setError(String(e.message ?? e)))
            }
          >
            {text.cancel}
          </button>
        ) : (
          <button
            type="button"
            data-control={history.attempts.length ? "conflict-review-retry" : "conflict-review-run"}
            onClick={() => void begin()}
          >
            {history.attempts.length ? text.retryReview : text.runReview}
          </button>
        )}
      </header>
      <p>{text.nonblocking}</p>
      {error && <p role="alert">{error}</p>}
      {history.currentResult && (
        <div
          className="review-current"
          role="status"
          aria-label={text.currentReviewResult}
        >
          <small>{text.currentReviewResult}</small>
          {" "}
          <ReviewResult review={history.currentResult} />
        </div>
      )}
      {history.attempts.length > 0 && (
        <ol className="review-attempts" aria-label={text.reviewAttempts}>
          {history.attempts.map((review) => (
            <li key={review.id}>
              <small>{text.reviewAttempt}</small>
              {" "}
              <ReviewResult review={review} />
            </li>
          ))}
        </ol>
      )}
    </section>
  );
}
