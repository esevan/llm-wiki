import { useCallback, useEffect, useRef, useState } from "react";
import { taskClient } from "../../services/taskClient";
import type { LineageSnapshot, TaskAggregate } from "../../types/taskWorkbench";
import { ConflictReviewPanel } from "./ConflictReviewPanel";
import { RefinementPanel } from "./RefinementPanel";
import { useTaskWorkbenchText } from "./taskWorkbenchText";

export function TaskDetail({
  taskId,
  onClose,
  onChanged,
}: {
  taskId: string;
  onClose: () => void;
  onChanged: () => void;
}) {
  const text = useTaskWorkbenchText();
  const [task, setTask] = useState<TaskAggregate>();
  const [error, setError] = useState("");
  const [entry, setEntry] = useState("");
  const [check, setCheck] = useState("");
  const [decision, setDecision] = useState("");
  const [attachment, setAttachment] = useState<File>();
  const [comments, setComments] = useState<Record<string, string>>({});
  const [refining, setRefining] = useState(false);
  const [completionEvidence, setCompletionEvidence] = useState("");
  const [problemId, setProblemId] = useState("");
  const [newProblem, setNewProblem] = useState("");
  const [problemStatement, setProblemStatement] = useState("");
  const [problemRevision, setProblemRevision] = useState("1");
  const [relatedTaskId, setRelatedTaskId] = useState("");
  const [relationshipKind, setRelationshipKind] = useState<
    "prerequisite" | "split_from" | "related"
  >("related");
  const [readinessReasons, setReadinessReasons] = useState<
    Record<string, string>
  >({});
  const [knowledgeDraft, setKnowledgeDraft] = useState<{
    draftRevision: number;
    bodyMarkdown: string;
    contentHash: string;
    state: string;
  }>();
  const [knowledgeBusy, setKnowledgeBusy] = useState(false);
  const [mutationBusy, setMutationBusy] = useState(false);
  const [lineage, setLineage] = useState<LineageSnapshot>();
  const loadSequence = useRef(0);
  const mutationQueue = useRef<Promise<void>>(Promise.resolve());
  const mutationBusyRef = useRef(false);
  const load = useCallback(async () => {
    const sequence = ++loadSequence.current;
    try {
      const next = await taskClient.task(taskId);
      if (sequence === loadSequence.current) setTask(next);
    } catch (e) {
      if (sequence === loadSequence.current)
        setError(String(e instanceof Error ? e.message : e));
    }
  }, [taskId]);
  useEffect(() => {
    void load();
  }, [load]);
  const update = (operation: () => Promise<TaskAggregate>) => {
    if (mutationBusyRef.current) return Promise.resolve();
    mutationBusyRef.current = true;
    setMutationBusy(true);
    const pending = mutationQueue.current.then(async () => {
      await operation();
      await load();
      onChanged();
    });
    mutationQueue.current = pending.catch((e) => {
      setError(String(e instanceof Error ? e.message : e));
    });
    void pending
      .finally(() => {
        mutationBusyRef.current = false;
        setMutationBusy(false);
      })
      .catch(() => undefined);
    return pending;
  };
  const encodeAttachment = async () => {
    if (!attachment) return undefined;
    const bytes = new Uint8Array(await attachment.arrayBuffer());
    let binary = "";
    bytes.forEach((byte) => {
      binary += String.fromCharCode(byte);
    });
    return {
      name: attachment.name,
      mediaType: attachment.type,
      data: btoa(binary),
    };
  };
  const correctKnowledge = async () => {
    if (!knowledgeDraft || knowledgeBusy) return;
    setKnowledgeBusy(true);
    try {
      setKnowledgeDraft(
        await taskClient.correctKnowledge(
          taskId,
          knowledgeDraft.draftRevision,
          knowledgeDraft.contentHash,
          knowledgeDraft.bodyMarkdown,
        ),
      );
    } catch (e) {
      setError(String(e instanceof Error ? e.message : e));
    } finally {
      setKnowledgeBusy(false);
    }
  };
  const publishKnowledge = async () => {
    if (!knowledgeDraft || knowledgeBusy) return;
    setKnowledgeBusy(true);
    try {
      await taskClient.publish(
        taskId,
        knowledgeDraft.draftRevision,
        knowledgeDraft.contentHash,
      );
      setKnowledgeDraft(undefined);
      await load();
    } catch (e) {
      setError(String(e instanceof Error ? e.message : e));
    } finally {
      setKnowledgeBusy(false);
    }
  };
  if (!task)
    return (
      <aside className="task-detail" aria-live="polite">
        {error || text.loading}
      </aside>
    );
  const revision = task.taskRevision;
  const latestProblemRevisions = new Map<string, number>();
  for (const link of task.problemLinks ?? []) {
    latestProblemRevisions.set(
      link.problemId,
      Math.max(latestProblemRevisions.get(link.problemId) ?? 0, link.problemRevision),
    );
  }
  return (
    <aside
      className="task-detail"
      aria-label={task.title}
      data-task-state={task.state}
      data-task-revision={revision}
      aria-busy={mutationBusy}
      inert={mutationBusy}
    >
      <header>
        <div>
          <small>
            {task.state === "completed"
              ? text.completed
              : task.state === "in_progress"
                ? text.inProgress
                : text.ready}
          </small>
          <h2>{task.title}</h2>
        </div>
        <button type="button" data-control="task-detail-close" aria-label={text.closeTaskDetail} onClick={onClose}>
          ×
        </button>
      </header>
      {error && <p role="alert">{error}</p>}
      <section className="task-actions">
        <button type="button" data-control="task-detail-refine" onClick={() => setRefining(true)}>
          {text.refine}
        </button>
        {task.state === "task" && (
          <button
            type="button"
            data-control="task-transition-start"
            onClick={() =>
              void update(() =>
                taskClient.transition(task.id, revision, "in_progress"),
              )
            }
          >
            {text.start}
          </button>
        )}
        {task.state === "in_progress" && (
          <button
            type="button"
            data-control="task-transition-complete-focus"
            aria-label="Add completion evidence"
            onClick={() =>
              document.getElementById("task-completion-evidence")?.focus()
            }
          >
            {text.complete}
          </button>
        )}
        {task.state === "completed" && (
          <button
            type="button"
            data-control="task-transition-reopen"
            onClick={() =>
              void update(() =>
                taskClient.transition(task.id, revision, "reopen"),
              )
            }
          >
            {text.reopen}
          </button>
        )}
      </section>
      {refining && (
        <RefinementPanel
          kind="task"
          subjectId={task.id}
          onClose={() => setRefining(false)}
        />
      )}
      <section className="task-panel">
        <h3>{text.taskDetails}</h3>
        <label>
          {text.title}
          <input
            data-control="task-revision-title"
            value={task.title}
            onChange={(event) =>
              setTask({ ...task, title: event.target.value })
            }
          />
        </label>
        <label>
          {text.detail}
          <textarea
            data-control="task-revision-detail"
            value={task.detail ?? ""}
            onChange={(event) =>
              setTask({ ...task, detail: event.target.value })
            }
          />
        </label>
        <label>
          {text.outcome}
          <textarea
            data-control="task-revision-outcome"
            value={task.outcome ?? ""}
            onChange={(event) =>
              setTask({ ...task, outcome: event.target.value })
            }
          />
        </label>
        <label>
          {text.scope}
          <textarea
            data-control="task-revision-scope"
            value={task.scope ?? ""}
            onChange={(event) =>
              setTask({ ...task, scope: event.target.value })
            }
          />
        </label>
        <label>
          {text.nonGoals}
          <textarea
            data-control="task-revision-non-goals"
            value={task.nonGoals ?? ""}
            onChange={(event) =>
              setTask({ ...task, nonGoals: event.target.value })
            }
          />
        </label>
        <label>
          {text.criteria}
          <textarea
            data-control="task-revision-criteria"
            value={task.validationCriteria ?? ""}
            onChange={(event) =>
              setTask({ ...task, validationCriteria: event.target.value })
            }
          />
        </label>
        <button
          type="button"
          data-control="task-revision-save"
          onClick={() =>
            void update(() =>
              taskClient.revise(task.id, revision, {
                title: task.title,
                detail: task.detail ?? "",
                outcome: task.outcome ?? "",
                scope: task.scope ?? "",
                nonGoals: task.nonGoals ?? "",
                validationCriteria: task.validationCriteria ?? "",
              }),
            )
          }
        >
          {text.saveChanges}
        </button>
      </section>
      <section className="task-panel">
        <h3>{text.worklog}</h3>
        <textarea
          data-control="task-worklog-text"
          aria-label={text.worklogEntry}
          value={entry}
          onChange={(event) => setEntry(event.target.value)}
        />
        <input
          data-control="task-worklog-file"
          aria-label={text.attach}
          type="file"
          onChange={(event) => setAttachment(event.currentTarget.files?.[0])}
        />
        <button
          type="button"
          data-control="task-worklog-add"
          disabled={!entry.trim()}
          onClick={() =>
            void encodeAttachment().then((file) =>
              update(() =>
                taskClient
                  .workLog(task.id, revision, entry, file)
                  .then((next) => {
                    setEntry("");
                    setAttachment(undefined);
                    return next;
                  }),
              ),
            )
          }
        >
          {text.add}
        </button>
        {task.workLog?.map((log) => (
          <article className="log-entry" key={log.id}>
            <p>{log.body}</p>
            {log.attachment && (
              <small>{log.attachment.name ?? log.attachment.mediaType}</small>
            )}
            {log.comments?.map((comment) => (
              <p className="comment" key={comment.id}>
                {comment.body}
              </p>
            ))}
            <div className="inline-form">
              <input
                data-control="task-comment-text"
                data-record-id={log.id}
                aria-label={`${text.comment} ${log.id}`}
                value={comments[log.id] ?? ""}
                onChange={(event) =>
                  setComments({ ...comments, [log.id]: event.target.value })
                }
              />
              <button
                type="button"
                data-control="task-comment-add"
                data-record-id={log.id}
                disabled={!(comments[log.id] ?? "").trim()}
                onClick={() =>
                  void taskClient
                    .comment(log.id, comments[log.id])
                    .then(() => {
                      setComments({ ...comments, [log.id]: "" });
                      return load();
                    })
                    .catch((error) =>
                      setError(
                        String(error instanceof Error ? error.message : error),
                      ),
                    )
                }
              >
                {text.comment}
              </button>
            </div>
          </article>
        ))}
      </section>
      <section className="task-panel">
        <h3>{text.checklist}</h3>
        {task.checklist?.map((item) => (
          <label key={item.id}>
            <input
              type="checkbox"
              data-control="task-checklist-toggle"
              data-record-id={item.id}
              checked={item.checked}
              onChange={(event) => {
                const checked = event.currentTarget.checked;
                void update(() =>
                  taskClient.updateChecklist(
                    task.id,
                    revision,
                    item.id,
                    checked,
                    item.body,
                  ),
                );
              }}
            />
            {item.body}
          </label>
        ))}
        <div className="inline-form">
          <input
            data-control="task-checklist-text"
            aria-label={text.checklistItem}
            value={check}
            onChange={(event) => setCheck(event.target.value)}
          />
          <button
            type="button"
            data-control="task-checklist-add"
            disabled={!check.trim()}
            onClick={() =>
              void update(() =>
                taskClient.checklist(task.id, revision, check).then((next) => {
                  setCheck("");
                  return next;
                }),
              )
            }
          >
            {text.add}
          </button>
        </div>
      </section>
      <section className="task-panel">
        <h3>{text.decisions}</h3>
        <div className="inline-form">
          <input
            data-control="task-decision-text"
            aria-label={text.decisionEntry}
            value={decision}
            onChange={(event) => setDecision(event.target.value)}
          />
          <button
            type="button"
            data-control="task-decision-add"
            disabled={!decision.trim()}
            onClick={() =>
              void update(() =>
                taskClient
                  .decision(task.id, revision, decision)
                  .then((next) => {
                    setDecision("");
                    return next;
                  }),
              )
            }
          >
            {text.add}
          </button>
        </div>
        {task.decisions?.map((item) => (
          <p key={item.id}>{item.body ?? item.kind}</p>
        ))}
      </section>
      <section className="task-panel">
        <h3>{text.readiness}</h3>
        {task.readinessEntries?.map((item) => (
          <article key={item.key}>
            <strong>
              {{
                outcome: text.readinessOutcome,
                scope: text.readinessScope,
                validationCriteria: text.readinessCriteria,
                prerequisites: text.readinessPrerequisites,
              }[item.key] ?? item.key}
            </strong>
            <span>
              {{
                resolved: text.statusResolved,
                missing: text.statusMissing,
                not_applicable: text.statusNotApplicable,
              }[item.status]}
            </span>
            {item.reason && <p>{item.reason}</p>}
            {item.status === "missing" && (
              <div className="inline-form">
                <input
                  data-control="task-readiness-reason"
                  data-record-id={item.key}
                  aria-label={`Reason ${item.key}`}
                  value={readinessReasons[item.key] ?? ""}
                  onChange={(event) =>
                    setReadinessReasons({
                      ...readinessReasons,
                      [item.key]: event.target.value,
                    })
                  }
                />
                <button
                  type="button"
                  data-control="task-readiness-not-applicable"
                  data-record-id={item.key}
                  disabled={!(readinessReasons[item.key] ?? "").trim()}
                  onClick={() =>
                    void update(() =>
                      taskClient.readinessDecision(
                        task.id,
                        revision,
                        item.key,
                        readinessReasons[item.key],
                      ),
                    )
                  }
                >
                  {text.notApplicable}
                </button>
              </div>
            )}
          </article>
        ))}
      </section>
      <section className="task-panel">
        <h3>{text.relationships}</h3>
        {task.problemLinks?.map((link) => (
          <p key={link.id}>
            {text.problem} · {text.problemRevision} {link.problemRevision}
            <button
              type="button"
              data-control="task-problem-unlink"
              data-record-id={link.id}
              onClick={() =>
                void update(() =>
                  taskClient.unlinkProblem(task.id, link.id, revision),
                )
              }
            >
              {text.unlink}
            </button>
            <button
              type="button"
              data-control="task-problem-resolve"
              data-record-id={link.id}
              disabled={link.problemRevision < (latestProblemRevisions.get(link.problemId) ?? link.problemRevision)}
              onClick={() =>
                void taskClient
                  .resolveProblem(
                    link.problemId,
                    link.problemRevision,
                    "Resolved after this Task",
                  )
                  .catch((error) => setError(String(error)))
              }
            >
              {text.resolveProblem}
            </button>
          </p>
        ))}
        {task.relationships?.map((link) => (
          <p key={link.id}>
            {link.kind.replace("_", " ")}
            <button
              type="button"
              data-control="task-relationship-unlink"
              data-record-id={link.id}
              onClick={() =>
                void update(() =>
                  taskClient.unlinkRelationship(task.id, link.id, revision),
                )
              }
            >
              {text.unlink}
            </button>
          </p>
        ))}
        <details className="connection-details" data-control="task-connection-details">
          <summary>{text.connectionDetails}</summary>
          <div className="inline-form">
            <input
              data-control="task-problem-create-text"
              aria-label={text.newProblemStatement}
              placeholder={text.newProblemPlaceholder}
              value={newProblem}
              onChange={(event) => setNewProblem(event.target.value)}
            />
            <button
              type="button"
              data-control="task-problem-create"
              disabled={!newProblem.trim()}
              onClick={() =>
                void taskClient
                  .createProblem(newProblem)
                  .then((problem) =>
                    taskClient
                      .problemLink(
                        task.id,
                        revision,
                        problem.id,
                        problem.problemRevision,
                      )
                      .then((next) => ({ next, problem })),
                  )
                  .then(({ problem }) => {
                    setNewProblem("");
                    setProblemId(problem.id);
                    setProblemRevision(String(problem.problemRevision));
                    void load();
                    onChanged();
                  })
                  .catch((e) =>
                    setError(String(e instanceof Error ? e.message : e)),
                  )
              }
            >
              {text.createAndLinkProblem}
            </button>
          </div>
          <div className="inline-form">
            <input
              data-control="task-problem-revision-text"
              aria-label={text.problemRevisionStatement}
              placeholder={text.problemRevisionPlaceholder}
              value={problemStatement}
              onChange={(event) => setProblemStatement(event.target.value)}
            />
            <button
              type="button"
              data-control="task-problem-revise"
              disabled={!problemId.trim() || !problemStatement.trim()}
              onClick={() =>
                void taskClient
                  .reviseProblem(problemId, problemStatement)
                  .then((problem) => {
                    setProblemRevision(String(problem.problemRevision));
                    setProblemStatement("");
                  })
                  .catch((e) =>
                    setError(String(e instanceof Error ? e.message : e)),
                  )
              }
            >
              {text.reviseProblem}
            </button>
          </div>
          <div className="inline-form">
            <input
              data-control="task-problem-link-id"
              aria-label={text.problemId}
              placeholder={text.problemId}
              value={problemId}
              onChange={(event) => setProblemId(event.target.value)}
            />
            <input
              data-control="task-problem-link-revision"
              aria-label={text.problemRevision}
              type="number"
              min="1"
              value={problemRevision}
              onChange={(event) => setProblemRevision(event.target.value)}
            />
            <button
              type="button"
              data-control="task-problem-link"
              disabled={!problemId.trim()}
              onClick={() =>
                void update(() =>
                  taskClient.problemLink(
                    task.id,
                    revision,
                    problemId,
                    Number(problemRevision),
                  ),
                )
              }
            >
              {text.linkProblem}
            </button>
          </div>
          <div className="inline-form">
            <input
              data-control="task-relationship-target"
              aria-label={text.relatedTaskId}
              placeholder={text.taskId}
              value={relatedTaskId}
              onChange={(event) => setRelatedTaskId(event.target.value)}
            />
            <select
              data-control="task-relationship-kind"
              aria-label={text.relationshipKind}
              value={relationshipKind}
              onChange={(event) =>
                setRelationshipKind(event.target.value as typeof relationshipKind)
              }
            >
              <option value="related">{text.related}</option>
              <option value="prerequisite">{text.prerequisite}</option>
              <option value="split_from">{text.splitFrom}</option>
            </select>
            <button
              type="button"
              data-control="task-relationship-link"
              disabled={!relatedTaskId.trim()}
              onClick={() =>
                void update(() =>
                  taskClient.relationship(
                    task.id,
                    revision,
                    relatedTaskId,
                    relationshipKind,
                  ),
                )
              }
            >
              {text.linkTask}
            </button>
          </div>
        </details>
      </section>
      <ConflictReviewPanel taskId={task.id} taskRevision={revision} />
      <section className="task-panel">
        <h3>{text.completion}</h3>
        {task.completion ? (
          <p>{task.completion.evidence ?? task.completion.report}</p>
        ) : (
          <>
            <p>{text.completionHint}</p>
            <textarea
              id="task-completion-evidence"
              data-control="task-completion-evidence"
              aria-label={text.completionEvidence}
              value={completionEvidence}
              onChange={(event) => setCompletionEvidence(event.target.value)}
              placeholder={text.completionPlaceholder}
            />
            <button
              type="button"
              data-control="task-completion-complete"
              disabled={
                task.state !== "in_progress" || !completionEvidence.trim()
              }
              onClick={() =>
                void update(() =>
                  taskClient.complete(task.id, revision, completionEvidence),
                )
              }
            >
              {text.complete}
            </button>
          </>
        )}
      </section>
      <section className="task-panel">
        <h3>{text.knowledge}</h3>
        <p>{text.publishHint}</p>
        <button
          type="button"
          data-control="task-knowledge-draft"
          disabled={task.state !== "completed"}
          onClick={() =>
            void taskClient
              .knowledgeDraft(task.id, revision)
              .then((draft) => {
                setKnowledgeDraft(draft);
                return load();
              })
              .catch((e) => setError(String(e.message ?? e)))
          }
        >
          {text.createDraft}
        </button>
        {knowledgeDraft && (
          <article
            className="knowledge-draft"
            data-content-hash={knowledgeDraft.contentHash}
          >
            <h4>
              {text.draft} · r{knowledgeDraft.draftRevision}
            </h4>
            <textarea
              data-control="task-knowledge-draft-body"
              aria-label={text.knowledgeDraftBody}
              value={knowledgeDraft.bodyMarkdown}
              onChange={(event) =>
                setKnowledgeDraft({
                  ...knowledgeDraft,
                  bodyMarkdown: event.target.value,
                })
              }
            />
            <button
              type="button"
              data-control="task-knowledge-correct"
              disabled={knowledgeBusy}
              aria-busy={knowledgeBusy}
              onClick={() => void correctKnowledge()}
            >
              {text.saveDraftCorrection}
            </button>
            <button
              type="button"
              data-control="task-knowledge-publish"
              disabled={knowledgeBusy}
              onClick={() => void publishKnowledge()}
            >
              {text.publish}
            </button>
          </article>
        )}
        {task.publication?.draftRevision && task.publication.contentHash && (
          <div
            className="inline-form publication-controls"
            data-publication-revision={task.publication.draftRevision}
            data-publication-state={task.publication.state}
          >
            <button
              type="button"
              data-control="task-knowledge-publish"
              onClick={() =>
                void taskClient
                  .publish(
                    task.id,
                    task.publication!.draftRevision!,
                    task.publication!.contentHash!,
                  )
                  .then(() => load())
                  .catch((e) => setError(String(e.message ?? e)))
              }
            >
              {text.publish}
            </button>
            <button
              type="button"
              data-control="task-knowledge-regenerate"
              onClick={() =>
                void taskClient
                  .regenerateKnowledge(
                    task.id,
                    task.publication!.draftRevision!,
                    task.taskRevision,
                  )
                  .then((draft) =>
                    setKnowledgeDraft(draft as typeof knowledgeDraft),
                  )
                  .catch((e) => setError(String(e.message ?? e)))
              }
            >
              {text.regenerateDraft}
            </button>
            {task.publication.state === "published" && (
              <button
                type="button"
                data-control="task-knowledge-withdraw"
                onClick={() =>
                  void taskClient
                    .withdrawKnowledge(
                      task.id,
                      task.publication!.draftRevision!,
                    )
                    .then(() => load())
                    .catch((e) => setError(String(e.message ?? e)))
                }
              >
                {text.withdrawKnowledge}
              </button>
            )}
          </div>
        )}
      </section>
      <section className="task-panel">
        <header>
          <h3>{text.flow}</h3>
          <button
            type="button"
            data-control="task-lineage-load"
            onClick={() =>
              void taskClient
                .lineage(task.id)
                .then(setLineage)
                .catch((e) => setError(String(e.message ?? e)))
            }
          >
            {text.flow}
          </button>
        </header>
        {lineage && (
          <ol className="lineage-flow">
            {lineage.nodes?.map((node) => (
              <li key={node.id}>
                <strong>{node.kind.replaceAll("_", " ")}</strong>
                <span>{node.title ?? text.recordedWork}</span>
              </li>
            ))}
          </ol>
        )}
      </section>
    </aside>
  );
}
