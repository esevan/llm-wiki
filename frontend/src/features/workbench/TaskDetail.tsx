import { useCallback, useEffect, useImperativeHandle, useLayoutEffect, useRef, useState, type ReactNode, type Ref } from "react";
import { createPortal } from "react-dom";
import { taskClient } from "../../services/taskClient";
import { formatSystemTime } from "../../services/systemTime";
import type { LineageSnapshot, TaskAggregate } from "../../types/taskWorkbench";
import { ConflictReviewPanel } from "./ConflictReviewPanel";
import { RefinementPanel } from "./RefinementPanel";
import { useTaskWorkbenchText } from "./taskWorkbenchText";

import { acknowledgeSave, baseline, definitionOf, editDraft, keepEdits, mergeSnapshot, type DefinitionField, type DetailState } from "./taskDraft";

export type TaskDetailHandle = { requestLeave: (proceed: () => void) => void };

type DetailScrollLock = {
  count: number;
  rootOverflow: string;
  bodyOverflow: string;
};

let detailScrollLock: DetailScrollLock | undefined;

type DetailTab = "work" | "details" | "review";
type KnowledgeDraft = {
  draftRevision: number;
  bodyMarkdown: string;
  savedBodyMarkdown?: string;
  contentHash: string;
  sourceHash?: string;
  state: string;
};

const orderedWorkLog = (workLog: TaskAggregate["workLog"] = []) =>
  workLog
    .map((log, index) => ({ log, index }))
    .sort((left, right) => {
      const leftTime = left.log.createdAt ? Date.parse(left.log.createdAt) : Number.NaN;
      const rightTime = right.log.createdAt ? Date.parse(right.log.createdAt) : Number.NaN;
      if (Number.isNaN(leftTime) || Number.isNaN(rightTime)) {
        return Number.isNaN(leftTime) === Number.isNaN(rightTime) ? left.index - right.index : Number.isNaN(leftTime) ? 1 : -1;
      }
      return rightTime - leftTime || left.index - right.index;
    })
    .map(({ log }) => log);

const workLogTimestamp = (createdAt?: string) =>
  createdAt
    ? formatSystemTime(createdAt, document.documentElement.lang || navigator.language, {
        dateStyle: "medium",
        timeStyle: "short",
      })
    : "";
export type DetailSession = {
  detailState?: DetailState;
  entry: string;
  check: string;
  decision: string;
  attachment?: File;
  comments: Record<string, string>;
  completionEvidence: string;
  problemId: string;
  newProblem: string;
  problemStatement: string;
  problemRevision: string;
  relatedTaskId: string;
  relationshipKind: "prerequisite" | "split_from" | "related";
  readinessReasons: Record<string, string>;
  knowledgeDraft?: KnowledgeDraft;
  tab: DetailTab;
  editing: boolean;
  scrollTop: number;
};

export function TaskDetail({
  taskId,
  onClose,
  onChanged,
  onRequestDelete,
  onRefine,
  onOpenTask,
  refreshKey,
  suppressInitialFocus,
  sessions,
  queueKnowledgeDraft,
  ref,
}: {
  taskId: string;
  onOpenTask?: (id: string) => void;
  onClose: () => void;
  onChanged: () => void;
  onRequestDelete?: (title: string) => void;
  onRefine?: () => void;
  refreshKey?: number;
  suppressInitialFocus?: boolean;
  sessions?: Map<string, DetailSession>;
  queueKnowledgeDraft?: KnowledgeDraft;
  ref?: Ref<TaskDetailHandle>;
}) {
  const text = useTaskWorkbenchText();
  const fallbackSessions = useRef(new Map<string, DetailSession>());
  const detailSessions = sessions ?? fallbackSessions.current;
  const session = detailSessions.get(taskId);
  const initialScrollTop = useRef(session?.scrollTop ?? 0);
  const restoredScroll = useRef(false);
  const hadSession = useRef(Boolean(session));
  const [detailState, setDetailState] = useState<DetailState | undefined>(session?.detailState);
  const loaded = Boolean(detailState);
  const [error, setError] = useState("");
  const [entry, setEntry] = useState(session?.entry ?? "");
  const [check, setCheck] = useState(session?.check ?? "");
  const [decision, setDecision] = useState(session?.decision ?? "");
  const [attachment, setAttachment] = useState<File | undefined>(session?.attachment);
  const [comments, setComments] = useState<Record<string, string>>(session?.comments ?? {});
  const [refining, setRefining] = useState(false);
  const [completionEvidence, setCompletionEvidence] = useState(session?.completionEvidence ?? "");
  const [problemId, setProblemId] = useState(session?.problemId ?? "");
  const [newProblem, setNewProblem] = useState(session?.newProblem ?? "");
  const [problemStatement, setProblemStatement] = useState(session?.problemStatement ?? "");
  const [problemRevision, setProblemRevision] = useState(session?.problemRevision ?? "1");
  const [relatedTaskId, setRelatedTaskId] = useState(session?.relatedTaskId ?? "");
  const [relationshipKind, setRelationshipKind] = useState<
    "prerequisite" | "split_from" | "related"
  >(session?.relationshipKind ?? "related");
  const [readinessReasons, setReadinessReasons] = useState<
    Record<string, string>
  >(session?.readinessReasons ?? {});
  const [knowledgeDraft, setKnowledgeDraft] = useState<KnowledgeDraft | undefined>(() =>
    session?.knowledgeDraft && {
      ...session.knowledgeDraft,
      savedBodyMarkdown: session.knowledgeDraft.savedBodyMarkdown ?? session.knowledgeDraft.bodyMarkdown,
    },
  );
  const [tab, setTab] = useState<DetailTab>(session?.tab ?? "work");
  const [editing, setEditing] = useState(session?.editing ?? false);
  const [showCompletedChecklist, setShowCompletedChecklist] = useState(false);
  const [knowledgeAction, setKnowledgeAction] = useState<"create" | "correct" | "publish">();
  const knowledgeBusy = Boolean(knowledgeAction);
  const [knowledgeStatus, setKnowledgeStatus] = useState<keyof typeof text | "">("");
  const [knowledgeError, setKnowledgeError] = useState("");
  const [knowledgeRetry, setKnowledgeRetry] = useState<"create" | "correct" | "publish">();
  const [knowledgeQueued, setKnowledgeQueued] = useState(false);
  const [mutationBusy, setMutationBusy] = useState(false);
  const [lineage, setLineage] = useState<LineageSnapshot>();
  const [closePrompt, setClosePrompt] = useState(false);
  const pendingLeave = useRef<(() => void) | undefined>(undefined);
  const panelRef = useRef<HTMLElement>(null);
  const headingRef = useRef<HTMLHeadingElement>(null);
  const pendingTabFocus = useRef<string | undefined>(undefined);
  const focusInTab = (nextTab: DetailTab, selector: string) => {
    if (tab === nextTab) panelRef.current?.querySelector<HTMLElement>(selector)?.focus();
    else { pendingTabFocus.current = selector; setTab(nextTab); }
  };
  useLayoutEffect(() => {
    if (!pendingTabFocus.current) return;
    panelRef.current?.querySelector<HTMLElement>(pendingTabFocus.current)?.focus();
    pendingTabFocus.current = undefined;
  }, [tab]);
  const loadSequence = useRef(0);
  const mutationQueue = useRef<Promise<void>>(Promise.resolve());
  const mutationBusyRef = useRef(false);
  useEffect(() => {
    detailSessions.set(taskId, { detailState, entry, check, decision, attachment, comments, completionEvidence, problemId, newProblem, problemStatement, problemRevision, relatedTaskId, relationshipKind, readinessReasons, knowledgeDraft, tab, editing, scrollTop: restoredScroll.current ? panelRef.current?.scrollTop ?? 0 : initialScrollTop.current });
  }, [attachment, check, comments, completionEvidence, decision, detailSessions, detailState, editing, entry, knowledgeDraft, newProblem, problemId, problemRevision, problemStatement, readinessReasons, relatedTaskId, relationshipKind, session?.scrollTop, tab, taskId]);
  useEffect(() => {
    const panel = panelRef.current;
    if (loaded && panel && !restoredScroll.current) {
      panel.scrollTop = initialScrollTop.current;
      restoredScroll.current = true;
    }
    return () => {
      const current = detailSessions.get(taskId);
      if (current) current.scrollTop = panel?.scrollTop ?? current.scrollTop;
    };
  }, [detailSessions, loaded, taskId]);
  const load = useCallback(async () => {
    const sequence = ++loadSequence.current;
    try {
      const next = await taskClient.task(taskId);
      if (sequence === loadSequence.current) {
        setError("");
        setDetailState((current) => current ? mergeSnapshot(current, next) : baseline(next));
        const publication = next.publication;
        if (publication?.state === "draft" && publication.draftRevision && publication.contentHash && publication.bodyMarkdown) {
          const persistedDraft = {
            draftRevision: publication.draftRevision,
            bodyMarkdown: publication.bodyMarkdown,
            savedBodyMarkdown: publication.bodyMarkdown,
            contentHash: publication.contentHash,
            sourceHash: publication.sourceHash,
            state: publication.state,
          };
          setKnowledgeDraft((current) => current ?? persistedDraft);
        }
        if (!hadSession.current) {
          setTab(next.state === "completed" ? "review" : next.state === "task" ? "details" : "work");
          hadSession.current = true;
        }
      }
    } catch (e) {
      if (sequence === loadSequence.current)
        setError(String(e instanceof Error ? e.message : e));
    }
  }, [taskId]);
  useEffect(() => {
    const sequences = loadSequence;
    void load();
    return () => { ++sequences.current; };
  }, [load, refreshKey]);
  useEffect(() => {
    if (!queueKnowledgeDraft) return;
    setKnowledgeQueued(false);
    setKnowledgeStatus("");
    setKnowledgeDraft((current) => current?.draftRevision === queueKnowledgeDraft.draftRevision && current.contentHash === queueKnowledgeDraft.contentHash
      ? current
      : { ...queueKnowledgeDraft, savedBodyMarkdown: queueKnowledgeDraft.bodyMarkdown });
    setTab("review");
  }, [queueKnowledgeDraft]);
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
  const editDefinition = (field: DefinitionField, value: string) => {
    if (mutationBusyRef.current) return;
    setDetailState((current) => current && editDraft(current, field, value));
  };
  const requestLeave = useCallback((proceed: () => void) => {
    if (mutationBusyRef.current) return;
    if (detailState?.dirty.size) {
      pendingLeave.current = proceed;
      setClosePrompt(true);
    } else proceed();
  }, [detailState]);
  useImperativeHandle(ref, () => ({ requestLeave }), [requestLeave]);
  const finishLeave = () => {
    const proceed = pendingLeave.current;
    pendingLeave.current = undefined;
    setClosePrompt(false);
    proceed?.();
  };
  useEffect(() => {
    if (closePrompt) panelRef.current?.querySelector<HTMLButtonElement>("[data-control='task-draft-guard-save']")?.focus();
  }, [closePrompt]);
  useEffect(() => {
    if (loaded && !suppressInitialFocus && !panelRef.current?.closest(".view:not(.active)")) headingRef.current?.focus({ preventScroll: true });
  }, [loaded, suppressInitialFocus]);
  useEffect(() => {
    const application = document.querySelector<HTMLElement>(".app");
    const wasInert = application?.inert;
    if (application) application.inert = true;
    const root = document.documentElement;
    const body = document.body;
    if (detailScrollLock) detailScrollLock.count += 1;
    else {
      detailScrollLock = {
        count: 1,
        rootOverflow: root.style.overflow,
        bodyOverflow: body.style.overflow,
      };
      root.style.overflow = "hidden";
      body.style.overflow = "hidden";
    }
    return () => {
      if (application) application.inert = wasInert ?? false;
      const lock = detailScrollLock;
      if (!lock || --lock.count > 0) return;
      root.style.overflow = lock.rootOverflow;
      body.style.overflow = lock.bodyOverflow;
      detailScrollLock = undefined;
    };
  }, []);
  const focusedQueueResult = useRef<KnowledgeDraft | undefined>(undefined);
  useEffect(() => {
    if (!queueKnowledgeDraft || !loaded || tab !== "review"
        || knowledgeDraft?.draftRevision !== queueKnowledgeDraft.draftRevision
        || focusedQueueResult.current === queueKnowledgeDraft) return;
    const frame = requestAnimationFrame(() => {
      const panel = panelRef.current;
      const preview = panel?.querySelector<HTMLElement>(".knowledge-draft-preview");
      if (!panel || !preview) return;
      const headerHeight = panel.querySelector<HTMLElement>(".task-detail-header")?.offsetHeight ?? 0;
      panel.scrollTop += preview.getBoundingClientRect().top - panel.getBoundingClientRect().top - headerHeight - 12;
      preview.focus({ preventScroll: true });
      focusedQueueResult.current = queueKnowledgeDraft;
    });
    return () => cancelAnimationFrame(frame);
  }, [knowledgeDraft?.draftRevision, loaded, queueKnowledgeDraft, tab]);
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
    if (!knowledgeDraft.sourceHash) {
      setError("This Knowledge draft has no source hash. Refresh it before saving a correction.");
      return;
    }
    setKnowledgeAction("correct");
    setKnowledgeError("");
    setKnowledgeRetry(undefined);
    setKnowledgeStatus("savingDraft");
    try {
      const corrected = await taskClient.correctKnowledge(
          taskId,
          knowledgeDraft.draftRevision,
          knowledgeDraft.contentHash,
          knowledgeDraft.sourceHash,
          knowledgeDraft.bodyMarkdown,
        );
      setKnowledgeDraft({ ...corrected, savedBodyMarkdown: corrected.bodyMarkdown });
      await load();
      setKnowledgeStatus("draftSaved");
    } catch (e) {
      setKnowledgeStatus("");
      setKnowledgeError(String(e instanceof Error ? e.message : e));
      setKnowledgeRetry("correct");
    } finally {
      setKnowledgeAction(undefined);
    }
  };
  const publishKnowledge = async () => {
    if (!knowledgeDraft || knowledgeBusy) return;
    if (!knowledgeDraft.sourceHash) {
      setError("This Knowledge draft has no source hash. Refresh it before publishing.");
      return;
    }
    setKnowledgeAction("publish");
    setKnowledgeError("");
    setKnowledgeRetry(undefined);
    setKnowledgeStatus("publishingDraft");
    try {
      await taskClient.publish(
        taskId,
        knowledgeDraft.draftRevision,
        knowledgeDraft.contentHash,
        knowledgeDraft.sourceHash,
      );
      setKnowledgeDraft(undefined);
      await load();
      setKnowledgeStatus("draftPublished");
    } catch (e) {
      setKnowledgeStatus("");
      setKnowledgeError(String(e instanceof Error ? e.message : e));
      setKnowledgeRetry("publish");
    } finally {
      setKnowledgeAction(undefined);
    }
  };
  const createKnowledgeDraft = async () => {
    if (!task || task.state !== "completed" || knowledgeBusy) return;
    setKnowledgeAction("create");
    setKnowledgeError("");
    setKnowledgeRetry(undefined);
    setKnowledgeStatus("creatingDraft");
    try {
      await taskClient.knowledgeDraft(task.id, revision);
      setKnowledgeQueued(true);
      setKnowledgeStatus("draftQueued");
    } catch (e) {
      setKnowledgeStatus("");
      setKnowledgeError(String(e instanceof Error ? e.message : e));
      setKnowledgeRetry("create");
    } finally {
      setKnowledgeAction(undefined);
    }
  };
  const retryKnowledge = () => {
    if (knowledgeRetry === "create") void createKnowledgeDraft();
    if (knowledgeRetry === "correct") void correctKnowledge();
    if (knowledgeRetry === "publish") void publishKnowledge();
  };
  const knowledgeEdited = Boolean(knowledgeDraft && knowledgeDraft.bodyMarkdown !== knowledgeDraft.savedBodyMarkdown);
  const task = detailState && { ...detailState.persisted, ...detailState.draft };
  const knowledgeDraftIsCurrent = Boolean(
    knowledgeDraft && task?.publication?.draftRevision === knowledgeDraft.draftRevision
      && task.publication.contentHash === knowledgeDraft.contentHash
      && task.publication.state === "draft",
  );
  const addWorkLog = () => {
    if (!task || (!entry.trim() && !attachment) || mutationBusyRef.current) return;
    void encodeAttachment()
      .then((file) =>
        update(() =>
          taskClient.workLog(task.id, revision, entry, file).then((next) => {
            setEntry("");
            setAttachment(undefined);
            return next;
          }),
        ),
      )
      .catch((e) => setError(String(e instanceof Error ? e.message : e)));
  };
  const saveDraft = async () => {
    if (!detailState || !detailState.dirty.size || detailState.conflicts.length || mutationBusyRef.current) return false;
    mutationBusyRef.current = true;
    setMutationBusy(true);
    setError("");
    try {
      const patch = Object.fromEntries([...detailState.dirty].map((field) => [field, detailState.draft[field]]));
      const partial = await taskClient.revise(detailState.persisted.id, detailState.baseRevision, patch);
      acknowledgeSave(detailState, partial); // Validate before treating the write as acknowledged.
      setDetailState((current) => acknowledgeSave(detailState, partial, current?.persisted));
      // Invalidate pre-save GETs before performing the ordered refresh.
      const sequence = ++loadSequence.current;
      try {
        const refreshed = await taskClient.task(detailState.persisted.id);
        if (sequence === loadSequence.current) {
          setDetailState((current) => current && mergeSnapshot(current, refreshed));
        }
      } catch (refreshError) {
        setError(`${text.savedRefreshFailed} ${String(refreshError instanceof Error ? refreshError.message : refreshError)}`);
      }
      onChanged();
      return true;
    } catch (saveError) {
      setError(String(saveError instanceof Error ? saveError.message : saveError));
      // Fetch a comparison after stale revision failure; merge keeps dirty input
      // and the original base whenever a server edit overlaps it.
      if (String(saveError).includes("head_conflict")) await load();
      return false;
    } finally {
      mutationBusyRef.current = false;
      setMutationBusy(false);
    }
  };
  const modal = (content: ReactNode) => createPortal(
    <div className="task-detail-modal-layer" data-task-detail-modal="true">
      <div className="task-detail-modal-backdrop" aria-hidden="true" />
      {content}
    </div>,
    document.body,
  );
  if (!task)
    return modal(
      <aside className="task-detail" role="dialog" aria-modal="true" aria-label={text.taskDetails} aria-live="polite">
        <header><h2>{error || text.loading}</h2><button type="button" data-control="task-detail-close" aria-label={text.closeTaskDetail} onClick={onClose}>×</button></header>
        {error && <button type="button" data-control="task-detail-retry" onClick={() => void load()}>{text.retry}</button>}
      </aside>,
    );
  const revision = detailState.persisted.taskRevision;
  const definitions: Array<{ field: DefinitionField; label: string; control: string; value: string }> = [
    { field: "title", label: text.title, control: "task-revision-title", value: task.title },
    { field: "detail", label: text.detail, control: "task-revision-detail", value: task.detail ?? "" },
    { field: "outcome", label: text.outcome, control: "task-revision-outcome", value: task.outcome ?? "" },
    { field: "scope", label: text.scope, control: "task-revision-scope", value: task.scope ?? "" },
    { field: "nonGoals", label: text.nonGoals, control: "task-revision-non-goals", value: task.nonGoals ?? "" },
    { field: "validationCriteria", label: text.criteria, control: "task-revision-criteria", value: task.validationCriteria ?? "" },
  ];
  const latestProblemRevisions = new Map<string, number>();
  for (const link of task.problemLinks ?? []) {
    latestProblemRevisions.set(
      link.problemId,
      Math.max(latestProblemRevisions.get(link.problemId) ?? 0, link.problemRevision),
    );
  }
  return modal(
    <aside
      ref={panelRef}
      onKeyDown={(event) => {
        if (event.key === "Escape" && !event.nativeEvent.isComposing && !event.defaultPrevented && !refining) {
          event.preventDefault();
          event.stopPropagation();
          if (closePrompt) { pendingLeave.current = undefined; setClosePrompt(false); }
          else requestLeave(onClose);
        }
        if (event.key !== "Tab") return;
        const focusable = Array.from(event.currentTarget.querySelectorAll<HTMLElement>(
          "button:not(:disabled), textarea, input, select, summary, [tabindex]:not([tabindex='-1'])",
        )).filter((element) =>
          !element.closest("[hidden], [inert]")
          && !element.hasAttribute("disabled")
          && (!element.closest("details:not([open])") || element.matches("summary")),
        );
        if (!focusable.length) return;
        const first = focusable[0];
        const last = focusable[focusable.length - 1];
        if (event.shiftKey && (document.activeElement === first || document.activeElement === headingRef.current)) { event.preventDefault(); last.focus(); }
        else if (!event.shiftKey && document.activeElement === last) { event.preventDefault(); first.focus(); }
      }}
      className="task-detail"
      role="dialog"
      aria-modal="true"
      aria-label={task.title}
      data-task-state={task.state}
      data-task-revision={revision}
      aria-busy={mutationBusy}
      inert={mutationBusy}
      onScroll={(event) => { const current = detailSessions.get(taskId); if (current && restoredScroll.current) current.scrollTop = event.currentTarget.scrollTop; }}
    >
      <div className="task-detail-header"><header>
        <div>
          <small>
            {task.state === "completed"
              ? text.completed
              : task.state === "in_progress"
                ? text.inProgress
                : text.ready}
          </small>
          <h2 ref={headingRef} tabIndex={-1} title={task.title}>{task.title}</h2>
          <p className="task-goal" title={task.outcome}>{task.outcome || text.goalMissing}</p>
        </div>
        <button type="button" data-control="task-detail-close" aria-label={text.closeTaskDetail} onClick={() => requestLeave(onClose)}>
          <span aria-hidden="true">×</span><span className="task-detail-back">{text.back}</span>
        </button>
      </header>
      {task.refinedRevision && <p className="refined-status">{text.refined} {task.refinedRevision}</p>}
      {task.hierarchy && (task.hierarchy.parent || task.hierarchy.children.length > 0) && <details className="task-hierarchy-summary" data-control="task-hierarchy-details">
        <summary>{text.boundaryContext}</summary>
        {task.hierarchy.parent && <p>{text.parentTask}: <button type="button" data-control="task-parent-open" onClick={() => onOpenTask?.(task.hierarchy!.parent!.id)}>{task.hierarchy.parent.title}</button></p>}
        {[...task.hierarchy.siblings,...task.hierarchy.children].map(related => <p key={related.id}>
          {related.parentTaskId === task.id || task.hierarchy!.children.some(child => child.id === related.id) ? text.subtask : text.siblingTasks}: <button type="button" data-control="task-family-open" onClick={() => onOpenTask?.(related.id)}>{related.title}</button>
        </p>)}
        <p>{text.subtaskHint}</p>
      </details>}
      <nav className="task-detail-tabs" aria-label={text.taskDetails} role="tablist" onKeyDown={(event) => {
        const tabs: DetailTab[] = ["work", "details", "review"];
        const current = tabs.indexOf(tab);
        const next = event.key === "Home" ? 0 : event.key === "End" ? tabs.length - 1 : event.key === "ArrowRight" ? (current + 1) % tabs.length : event.key === "ArrowLeft" ? (current + tabs.length - 1) % tabs.length : -1;
        if (next >= 0) { event.preventDefault(); focusInTab(tabs[next], `[data-control="task-detail-tab-${tabs[next]}"]`); }
      }}>
        <button id="task-detail-tab-work" type="button" role="tab" tabIndex={tab === "work" ? 0 : -1} data-control="task-detail-tab-work" aria-controls="task-tab-work" aria-selected={tab === "work"} onClick={() => setTab("work")}>{text.work}</button>
        <button id="task-detail-tab-details" type="button" role="tab" tabIndex={tab === "details" ? 0 : -1} data-control="task-detail-tab-details" aria-controls="task-tab-details" aria-selected={tab === "details"} onClick={() => setTab("details")}>{text.detailsTab}</button>
        <button id="task-detail-tab-review" type="button" role="tab" tabIndex={tab === "review" ? 0 : -1} data-control="task-detail-tab-review" aria-controls="task-tab-review" aria-selected={tab === "review"} onClick={() => setTab("review")}>{text.reviewTab}</button>
      </nav></div>
      {error && <p role="alert">{error}</p>}
      {detailState.conflicts.length > 0 && (
        <section className="task-draft-conflict" role="alert">
          <p>{text.draftConflict}</p>
          {detailState.conflicts.map((field) => (
            <div key={field}>
              <strong>{{ title: text.title, detail: text.detail, outcome: text.outcome, scope: text.scope, nonGoals: text.nonGoals, validationCriteria: text.criteria }[field]}</strong>
              <p>{text.latestValue}: <span>{definitionOf(detailState.persisted)[field]}</span></p>
              <p>{text.editedValue}: <span>{detailState.draft[field]}</span></p>
            </div>
          ))}
          <button type="button" data-control="task-draft-keep-mine" onClick={() => setDetailState((current) => current && keepEdits(current))}>{text.keepMyEdits}</button>
          <button type="button" data-control="task-draft-use-latest" onClick={() => setDetailState((current) => current && baseline(current.persisted))}>{text.useLatestVersion}</button>
        </section>
      )}
      {closePrompt && (
        <section className="task-draft-guard" role="alertdialog" aria-label={text.unsavedTaskChanges}>
          <p>{text.saveBeforeClosing}</p>
          <button type="button" data-control="task-draft-guard-save" disabled={mutationBusy || detailState.conflicts.length > 0} onClick={() => {
            if (!detailState.dirty.size) finishLeave();
            else void saveDraft().then((saved) => { if (saved) finishLeave(); });
          }}>{text.guardSave}</button>
          <button type="button" data-control="task-draft-guard-discard" disabled={mutationBusy} onClick={() => { setDetailState((current) => { if (!current) return current; const reset = baseline(current.persisted); const cached = detailSessions.get(taskId); if (cached) detailSessions.set(taskId, { ...cached, detailState: reset, editing: false }); return reset; }); setEditing(false); finishLeave(); }}>{text.discard}</button>
          <button type="button" data-control="task-draft-guard-keep-editing" disabled={mutationBusy} onClick={() => { pendingLeave.current = undefined; setClosePrompt(false); headingRef.current?.focus(); }}>{text.keepEditing}</button>
        </section>
      )}
      <section className="task-actions">
        <button type="button" data-control="task-detail-refine" onClick={() => onRefine ? onRefine() : setRefining(true)}>
          {text.refine}
        </button>
        {task.state === "task" && (
          <button
            type="button"
            data-control="task-transition-start"
            className="primary"
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
            className="primary"
            aria-label={text.addCompletionEvidence}
            onClick={() => focusInTab("review", "#task-completion-evidence")}
          >
          {text.reviewCompletion}
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
        {onRequestDelete && <button type="button" data-control="task-detail-delete" onClick={() => onRequestDelete(task.title)}>
          {text.deleteItem}
        </button>}
      </section>
      {refining && (
        <RefinementPanel
          kind="task"
          subjectId={task.id}
          onClose={() => setRefining(false)}
        />
      )}
      <div id="task-tab-details" aria-labelledby="task-detail-tab-details" className="task-tab-panel" role="tabpanel" data-task-tab="details" hidden={tab !== "details"}>
      <section className="task-panel task-definition-panel">
        <h3>{text.taskDetails}</h3>
        <div className="task-definition-actions">
          <button type="button" data-control="task-definition-edit" onClick={() => setEditing(true)} hidden={editing}>{text.edit}</button>
          {editing && <button type="button" data-control="task-definition-cancel" onClick={() => { setDetailState((current) => current && baseline(current.persisted)); setEditing(false); }}>{text.cancel}</button>}
        </div>
        {editing ? definitions.map(({ field, label, control, value }) => (
          <label key={field}>{label}<textarea data-control={control} value={value} onChange={(event) => editDefinition(field, event.target.value)} /></label>
        )) : (
          <dl className="task-definition-read">
            {definitions.filter(({ field, value }) => field !== "title" && value.trim()).map(({ field, label, value }) => <div key={field}><dt>{label}</dt><dd>{value}</dd></div>)}
          </dl>
        )}
        <button
          type="button"
          data-control="task-revision-save"
          hidden={!editing}
          disabled={!editing || !detailState.dirty.size || detailState.conflicts.length > 0 || mutationBusy}
          onClick={() => void saveDraft().then((saved) => saved && setEditing(false))}
        >
          {text.saveChanges}
        </button>
      </section>
      </div>
      <div id="task-tab-work" aria-labelledby="task-detail-tab-work" className="task-tab-panel" role="tabpanel" data-task-tab="work" hidden={tab !== "work"}>
      <section className="task-panel">
        <header><h3>{text.checklist}</h3><small>{task.checklist?.filter((item) => item.checked).length ?? 0} / {task.checklist?.length ?? 0}</small></header>
        {task.checklist?.filter((item) => showCompletedChecklist || !item.checked).slice(0, showCompletedChecklist ? undefined : 5).map((item) => (
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
        {(task.checklist?.some((item) => item.checked) || (task.checklist?.length ?? 0) > 5) && <button type="button" data-control="task-checklist-completed-toggle" aria-expanded={showCompletedChecklist} onClick={() => setShowCompletedChecklist((shown) => !shown)}>{(task.checklist?.length ?? 0) > 5 ? showCompletedChecklist ? text.collapseChecklist : text.showAllChecklist : showCompletedChecklist ? text.hideCompleted : text.showCompleted}</button>}
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
        <h3>{text.worklog}</h3>
        <textarea
          data-control="task-worklog-text"
          aria-label={text.worklogEntry}
          value={entry}
          onChange={(event) => setEntry(event.target.value)}
          onPaste={(event) => {
            const imageItem = [...event.clipboardData.items].find((item) => item.type.startsWith("image/"));
            const pastedImage = imageItem?.getAsFile() ?? Array.from(event.clipboardData.files ?? []).find((file) => file.type.startsWith("image/"));
            if (pastedImage) {
              event.preventDefault();
              setAttachment(pastedImage);
            }
          }}
          onKeyDown={(event) => {
            if (event.key === "Enter" && !event.shiftKey && !event.nativeEvent.isComposing) {
              event.preventDefault();
              addWorkLog();
            }
          }}
        />
        <input
          data-control="task-worklog-file"
          aria-label={text.attach}
          type="file"
          onChange={(event) => setAttachment(event.currentTarget.files?.[0])}
        />
        {attachment && <p className="task-attachment-name">{text.attach}: {attachment.name}</p>}
        <button
          type="button"
          data-control="task-worklog-add"
          disabled={!entry.trim() && !attachment}
          onClick={addWorkLog}
        >
          {text.add}
        </button>
        {orderedWorkLog(task.workLog).map((log) => (
          <article className="log-entry" key={log.id}>
            {log.createdAt && <time dateTime={log.createdAt}>{workLogTimestamp(log.createdAt)}</time>}
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
        {!task.workLog?.length && <p className="region-empty">{text.noWorkLog}</p>}
      </section>
      <details className="task-panel" data-control="task-decisions-details">
        <summary>{text.decisions}</summary>
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
      </details>
      </div>
      <div id="task-tab-review" aria-labelledby="task-detail-tab-review" className="task-tab-panel" role="tabpanel" data-task-tab="review" hidden={tab !== "review"}>
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
            {item.reason && <p>{item.key === "prerequisites" && item.status === "resolved" && item.reason === "No explicit prerequisite" ? text.noExplicitPrerequisite : item.reason}</p>}
            {item.status === "missing" && (
              <div className="inline-form">
                <input
                  data-control="task-readiness-reason"
                  data-record-id={item.key}
                  aria-label={`${{ outcome: text.readinessOutcome, scope: text.readinessScope, validationCriteria: text.readinessCriteria, prerequisites: text.readinessPrerequisites }[item.key] ?? item.key} ${text.readinessReason}`}
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
      </div>
      <div className="task-tab-panel" data-task-tab="details" hidden={tab !== "details"}>
      <section className="task-panel task-relationships-panel">
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
      </div>
      <div className="task-tab-panel" data-task-tab="review" hidden={tab !== "review"}>
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
        <p>{task.hierarchy?.children.length ? text.subtaskHint : text.publishHint}</p>
        {task.autoPublicationError && <p role="alert">{text.autoPublishFailure}: {task.autoPublicationError}</p>}
        <button
          type="button"
          data-control="task-knowledge-draft"
          disabled={task.state !== "completed" || knowledgeBusy || knowledgeQueued}
          aria-busy={knowledgeBusy}
          onClick={() => void createKnowledgeDraft()}
        >
          {knowledgeAction === "create" ? text.creatingDraft : text.createDraft}
        </button>
        {knowledgeStatus && <p className="knowledge-draft-status" role="status">{text[knowledgeStatus]}</p>}
        {knowledgeError && (
          <div className="knowledge-draft-error" role="alert">
            <p>{knowledgeError}</p>
            <button type="button" data-control="task-knowledge-draft-retry" disabled={knowledgeBusy || !knowledgeRetry} onClick={retryKnowledge}>
              {text.retry}
            </button>
          </div>
        )}
        {knowledgeDraft && (
          <article
            className="knowledge-draft"
            data-content-hash={knowledgeDraft.contentHash}
          >
            <h4>
              {text.draft} · r{knowledgeDraft.draftRevision}
            </h4>
            <section className="knowledge-draft-preview" aria-label={text.draftPreview} tabIndex={-1}>
              <header><h5>{text.draftPreview}</h5><p>{text.draftPreviewHint}</p></header>
              <pre>{knowledgeDraft.bodyMarkdown}</pre>
            </section>
            <label className="knowledge-draft-editor">
              <span>{text.editDraft}</span>
            <textarea
              data-control="task-knowledge-draft-body"
              aria-label={text.knowledgeDraftBody}
              value={knowledgeDraft.bodyMarkdown}
              readOnly={!knowledgeDraftIsCurrent}
              onChange={(event) =>
                setKnowledgeDraft({
                  ...knowledgeDraft,
                  bodyMarkdown: event.target.value,
                })
              }
            />
            </label>
            {knowledgeEdited && <p className="knowledge-draft-status">{text.saveDraftBeforePublish}</p>}
            <button
              type="button"
              data-control="task-knowledge-correct"
              disabled={knowledgeBusy || !knowledgeDraft.sourceHash || !knowledgeDraftIsCurrent}
              aria-busy={knowledgeBusy}
              onClick={() => void correctKnowledge()}
            >
              {text.saveDraftCorrection}
            </button>
            <button
              type="button"
              data-control="task-knowledge-publish"
              disabled={knowledgeBusy || !knowledgeDraft.sourceHash || knowledgeEdited || !knowledgeDraftIsCurrent}
              onClick={() => void publishKnowledge()}
            >
              {text.publish}
            </button>
          </article>
        )}
        {!knowledgeDraft && task.publication?.draftRevision && task.publication.contentHash && (
          <div
            className="inline-form publication-controls"
            data-publication-revision={task.publication.draftRevision}
            data-publication-state={task.publication.state}
          >
            <button
              type="button"
              data-control="task-knowledge-publish"
              disabled={knowledgeBusy || !task.publication.sourceHash || knowledgeEdited}
              onClick={() =>
                void taskClient
                  .publish(
                    task.id,
                    task.publication!.draftRevision!,
                    task.publication!.contentHash!,
                    task.publication!.sourceHash!,
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
                  .then(async (draft) => {
                    const regenerated = draft as KnowledgeDraft;
                    setKnowledgeDraft({ ...regenerated, savedBodyMarkdown: regenerated.bodyMarkdown });
                    await load();
                  })
                  .catch((e) => setError(String(e.message ?? e)))
              }
            >
              {text.regenerateDraft}
            </button>
            {task.publication.state === "published" && (
              <button
                type="button"
                data-control="task-knowledge-withdraw"
                disabled={knowledgeBusy || !task.publication.sourceHash}
                onClick={() =>
                  void taskClient
                    .withdrawKnowledge(
                      task.id,
                      task.publication!.draftRevision!,
                      task.publication!.contentHash!,
                      task.publication!.sourceHash!,
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
      </div>
      <div className="task-tab-panel" data-task-tab="details" hidden={tab !== "details"}>
      <details className="task-panel" data-control="task-lineage-details"><summary>{text.flow}</summary>
        <header>
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
      </details>
      </div>
    </aside>,
  );
}
