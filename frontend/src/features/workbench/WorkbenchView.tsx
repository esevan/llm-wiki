import { localizedTask } from "./taskContent";
import { completedTasksNewestFirst } from "./completedTasks";
import { InputImageAttachment } from "./InputImageAttachment";
import { useInputImage } from "./useInputImage";
import type { InputImage } from "../../types/taskWorkbench";
import { useCallback, useEffect, useRef, useState } from "react";
import { taskClient } from "../../services/taskClient";
import type {
  CaptureCard,
  TaskCard,
  WorkbenchItem,
  WorkbenchSnapshot,
} from "../../types/taskWorkbench";
import { DeleteItemDialog } from "./DeleteItemDialog";
import { RefinementPanel, type RefinementPanelHandle } from "./RefinementPanel";
import { TaskDetail, type DetailSession, type TaskDetailHandle } from "./TaskDetail";
import { useTaskWorkbenchText } from "./taskWorkbenchText";
import "./task-workbench.css";
const itemTitle = (item: WorkbenchItem) =>
  item.kind === "capture" ? item.text : item.title;
export function WorkbenchView({ active }: { active: boolean }) {
  const text = useTaskWorkbenchText(),
    [storedSnapshot, setSnapshot] = useState<WorkbenchSnapshot>(),
    [input, setInput] = useState(""),
    [mode, setMode] = useState<"capture" | "task">("capture"),
    [busy, setBusy] = useState(false),
    [error, setError] = useState(""),
    [detail, setDetail] = useState<string>(),
    [refining, setRefining] = useState<{
      kind: "capture" | "task";
      id: string;
    }>(),
    [deleteTarget, setDeleteTarget] = useState<{ entityType: "captures" | "problems" | "tasks"; id: string; title: string }>(),
    [deleteBusy, setDeleteBusy] = useState(false),
    [deleteError, setDeleteError] = useState("");
  const snapshot = storedSnapshot && { ...storedSnapshot, categories: storedSnapshot.categories.map(category => ({
    ...category, items: category.items.map(item => item.kind === "task" ? localizedTask(item) : item),
  })) };
  const captureImage = useInputImage();
  const refinementImages = useRef(new Map<string, InputImage>());
  const [focusActive, setFocusActive] = useState(false);
  const detailRef = useRef<TaskDetailHandle>(null);
  const refinementRef = useRef<RefinementPanelHandle>(null);
  const [detailRefresh, setDetailRefresh] = useState(0);
  const [queueImageSummary, setQueueImageSummary] = useState<{ taskId: string; entryId: string }>();
  const [queueKnowledgeDraft, setQueueKnowledgeDraft] = useState<({ taskId: string } & NonNullable<DetailSession["knowledgeDraft"]>)>();
  const detailSessions = useRef(new Map<string, DetailSession>());
  const refinementMessages = useRef(new Map<string, string>());
  const detailTrigger = useRef<HTMLElement | null>(null);
  const restoreDetailFocus = () => requestAnimationFrame(() => {
    const target = detailTrigger.current?.isConnected && !detailTrigger.current.closest(".view:not(.active), [hidden]") ? detailTrigger.current : document.querySelector<HTMLElement>("#workbench.active h1");
    target?.focus({ preventScroll: true });
  });
  const closeLegacyDock = () => {
    const dialog = document.querySelector<HTMLDialogElement>('#chat-modal[data-workspace-dock="true"][open]');
    dialog?.close();
  };
  const selectDetail = (id: string | undefined, trigger?: HTMLElement) => {
    if (id === detail) return;
    const proceed = () => {
      closeLegacyDock();
      setRefining(undefined);
      setDetail(id);
      if (id) detailTrigger.current = trigger ?? null;
      else restoreDetailFocus();
    };
    const leaveTask = () => { if (detailRef.current) detailRef.current.requestLeave(proceed); else proceed(); };
    if (refinementRef.current) refinementRef.current.requestLeave(leaveTask); else leaveTask();
  };
  const openRefinement = (next: { kind: "capture" | "task"; id: string }, trigger?: HTMLElement) => {
    if (refining?.kind === next.kind && refining.id === next.id) return;
    const proceed = () => {
      closeLegacyDock();
      setDetail(next.kind === "task" ? next.id : undefined);
      if (trigger) detailTrigger.current = trigger;
      setRefining(next);
    };
    const leaveTask = () => {
      if (detailRef.current && detail !== (next.kind === "task" ? next.id : undefined)) detailRef.current.requestLeave(proceed);
      else proceed();
    };
    if (refinementRef.current) refinementRef.current.requestLeave(leaveTask); else leaveTask();
  };
  const openLegacyRefinement = (problemId: string, problemRevision?: number, sourceTitle?: string) => {
    const proceed = () => {
      setRefining(undefined); setDetail(undefined);
      window.openChat?.("problems", problemId, { problemRevision, sourceTitle, workspaceDock: true });
    };
    const leaveTask = () => { if (detailRef.current) detailRef.current.requestLeave(proceed); else proceed(); };
    if (refinementRef.current) refinementRef.current.requestLeave(leaveTask); else leaveTask();
  };
  const load = useCallback(async () => {
    try {
      setError("");
      const next = await taskClient.workbench();
      setSnapshot({
        ...next,
        activeShortcuts: next.activeShortcuts ?? [],
        refiningShortcuts: next.refiningShortcuts ?? [],
        categories: next.categories ?? [],
      });
    } catch (e) {
      setError(String(e instanceof Error ? e.message : e));
    }
  }, []);
  useEffect(() => {
    if (active) void load();
    else closeLegacyDock();
  }, [active, load]);
  useEffect(() => {
    const refresh = () => active && void load();
    window.addEventListener("llm-wiki:task-workbench-refresh", refresh);
    return () =>
      window.removeEventListener("llm-wiki:task-workbench-refresh", refresh);
  }, [active, load]);
  useEffect(() => {
    window.llmWikiOpenKnowledgeDraft = (draft) => {
      const current = detailSessions.current.get(draft.taskId);
      if (current?.knowledgeDraft && current.knowledgeDraft.savedBodyMarkdown !== undefined
          && current.knowledgeDraft.bodyMarkdown !== current.knowledgeDraft.savedBodyMarkdown) {
        setError(text.saveDraftBeforeQueueResult);
        return;
      }
      closeLegacyDock();
      setRefining(undefined);
      setQueueKnowledgeDraft({ ...draft });
      setDetail(draft.taskId);
      setDetailRefresh((value) => value + 1);
    };
    return () => { delete window.llmWikiOpenKnowledgeDraft; };
  }, [text.saveDraftBeforeQueueResult]);
  useEffect(() => {
    window.llmWikiOpenTaskImageSummary = (taskId, entryId) => {
      closeLegacyDock();
      setRefining(undefined);
      setDetail(taskId);
      setDetailRefresh((value) => value + 1);
      setQueueImageSummary({ taskId, entryId });
    };
    return () => { delete window.llmWikiOpenTaskImageSummary; };
  }, []);
  const save = async (event: React.FormEvent) => {
    event.preventDefault();
    if ((!input.trim() && !(mode === "capture" && captureImage.image)) || busy || captureImage.reading) return;
    setBusy(true);
    setError("");
    try {
      await (mode === "capture"
        ? taskClient.createCapture(input.trim(), captureImage.image)
        : taskClient.createTask(input.trim()));
      setInput("");
      if (mode === "capture") captureImage.clear();
      await load();
    } catch (e) {
      setError(String(e instanceof Error ? e.message : e));
    } finally {
      setBusy(false);
    }
  };
  const taskFor = (id: string) =>
    snapshot?.categories
      .flatMap((group) => group.items)
      .find((item): item is TaskCard => item.kind === "task" && item.id === id);
  const captureFor = (id: string) =>
    snapshot?.categories
      .flatMap((group) => group.items)
      .find(
        (item): item is CaptureCard =>
          item.kind === "capture" && item.id === id,
      );
  const trackedTitle = (
    snapshot as
      (WorkbenchSnapshot & { activeSelection?: { title?: string } }) | undefined
  )?.activeSelection?.title;
  const stateLabel = (task: TaskCard) =>
    task.state === "completed"
      ? text.completed
      : task.state === "in_progress"
        ? text.inProgress
        : task.refinedRevision ? `${text.refined} ${task.refinedRevision}` : text.ready;
  const requestDelete = (entityType: "captures" | "problems" | "tasks", id: string, title: string) => {
    const proceed = () => {
      setDeleteError("");
      setDeleteTarget({ entityType, id, title });
    };
    if (entityType === "tasks" && detail === id && detailRef.current) detailRef.current.requestLeave(proceed);
    else proceed();
  };
  const confirmDelete = async () => {
    if (!deleteTarget || deleteBusy) return;
    setDeleteBusy(true);
    setDeleteError("");
    try {
      await taskClient.deleteItem(deleteTarget.entityType, deleteTarget.id);
      if (detail === deleteTarget.id) setDetail(undefined);
      await load();
      setDeleteTarget(undefined);
    } catch (e) {
      setDeleteError(String(e instanceof Error ? e.message : e) || text.deleteFailure);
    } finally {
      setDeleteBusy(false);
    }
  };
  const items = snapshot?.categories.flatMap((category) => category.items) ?? [];
  const categoryLabels = new Map(snapshot?.categories.flatMap((category) =>
    category.items.map((item) => [`${item.kind}:${item.id}`, category.label] as const)));
  const refiningIds = new Set(snapshot?.refiningShortcuts.map((item) => `${item.kind}:${item.id}`));
  const activeTasks = items.filter((item) => item.kind === "task" && item.state === "in_progress");
  const completedTasks = completedTasksNewestFirst(items).slice(0, 5);
  const allTasks = items.filter((item): item is TaskCard => item.kind === "task");
  const pendingItems = items.filter((item) => (item.kind !== "task" || item.state === "task") && (item.kind !== "task" || !item.parentTaskId || !allTasks.some(parent => parent.id === item.parentTaskId)));
  const lanes = [
    { id: "inbox", title: text.inbox, hint: text.inboxHint, empty: text.noInbox,
      items: pendingItems.filter((item) => item.kind === "capture" && !refiningIds.has(`capture:${item.id}`)) },
    { id: "refining", title: text.refining, hint: text.refiningHint, empty: text.noRefining,
      items: pendingItems.filter((item) => item.kind === "refinement" || refiningIds.has(`${item.kind}:${item.id}`)) },
    { id: "tasks", title: text.refinedTasks, hint: text.refinedTasksHint, empty: text.noReadyTasks,
      items: pendingItems.filter((item) => item.kind === "task" && !refiningIds.has(`task:${item.id}`)) },
  ];
  const visibleIds = new Set(lanes.flatMap(lane => lane.items.map(item => `${item.kind}:${item.id}`)));
  const categories = (snapshot?.categories ?? []).filter(category => category.items.some(item => visibleIds.has(`${item.kind}:${item.id}`)));
  const generalIndex = categories.findIndex(category => category.id.toLowerCase() === "general" || category.label === "General");
  if (generalIndex >= 0) categories.unshift(...categories.splice(generalIndex, 1));
  const renderCard = (item: WorkbenchItem): React.ReactNode => (
    <article
      className={`canonical-card${(item.kind === "task" && detail === item.id) || (refining?.kind === item.kind && refining.id === item.id) ? " selected" : ""}`}
      key={`${item.kind}-${item.id}`}
      data-entity-id={item.id}
    >
      <small>
        {item.kind === "task"
          ? stateLabel(item)
          : item.kind === "capture"
            ? text.capture
            : text.refining}
      </small>
      <h3>{item.kind === "task" && item.state === "in_progress" ? (
        <button className="active-task-title" data-control="task-shortcut-open" data-entity-id={item.id} onClick={(event) => selectDetail(item.id, event.currentTarget)}>{itemTitle(item)}</button>
      ) : itemTitle(item) || text.imageCapture}</h3>
      {item.kind === "task" && item.originCaptureText && <p className="workbench-card-category">{text.originalCapture}: {item.originCaptureText}</p>}
      {item.kind === "capture" && item.hasImage && <small>{text.attachedImage}</small>}
      {item.kind === "task" && item.parentTaskId && <small>{text.subtask}</small>}
      <p className="workbench-card-category">{categoryLabels.get(`${item.kind}:${item.id}`)}</p>
        {item.kind === "task" && (
        <p>
          {item.readiness
            ? `${item.readiness.missing} ${text.readinessItemsRemain}`
            : ""}
        </p>
      )}
      <footer>
        {item.kind === "task" ? (
          <>
            <button
              data-control="task-card-open"
              data-entity-id={item.id}
              type="button"
              onClick={(event) => selectDetail(item.id, event.currentTarget)}
            >
              {text.open}
            </button>
            <button
              data-control={refiningIds.has(`${item.kind}:${item.id}`) ? "task-shortcut-refine" : "task-card-refine"}
              data-entity-id={item.id}
              type="button"
              onClick={(event) =>
                openRefinement({ kind: "task", id: item.id }, event.currentTarget)
              }
            >
              {text.refine}
            </button>
            <button type="button" data-control="task-card-delete" data-entity-id={item.id} onClick={() => requestDelete("tasks", item.id, item.title)}>{text.delete}</button>
          </>
        ) : item.kind === "capture" ? (
          <>
          <button
            data-control={refiningIds.has(`${item.kind}:${item.id}`) ? "task-shortcut-refine" : "task-card-refine"}
            data-entity-id={item.id}
            type="button"
            onClick={(event) =>
              openRefinement({ kind: "capture", id: item.id }, event.currentTarget)
            }
          >
            {text.refine}
          </button>
          <button type="button" data-control="task-card-delete" data-entity-id={item.id} onClick={() => requestDelete("captures", item.id, item.text)}>{text.delete}</button>
          </>
        ) : (
          <>
          <button
            type="button"
            data-control={refiningIds.has(`${item.kind}:${item.id}`) ? "task-shortcut-refine" : "task-card-refine"}
            data-entity-id={item.problemId}
            onClick={() =>
              openLegacyRefinement(item.problemId, item.problemRevision, item.title)
            }
          >
            {text.refine}
          </button>
          <button type="button" data-control="task-card-delete" data-entity-id={item.problemId} onClick={() => requestDelete("problems", item.problemId, item.title)}>{text.delete}</button>
          </>
        )}
      </footer>
      {item.kind === "task" && allTasks.some(child => child.parentTaskId === item.id) && <details className="subtask-tree" data-control="task-subtasks-expand">
        <summary>{text.subtasks} · {allTasks.filter(child => child.parentTaskId === item.id).length}</summary>
        <p>{text.subtaskHint}</p>
        <div className="subtask-children">{allTasks.filter(child => child.parentTaskId === item.id).map(child => renderCard(child))}</div>
      </details>}
    </article>
  );
  return (
    <section
      id="workbench"
      className={`view task-workbench${active ? " active" : ""}`}
      data-task-workbench="true"
      data-focus-active={focusActive}
      data-refining-kind={refining?.kind}
      aria-busy={!snapshot && !error}
    >
      <header className="top">
        <div>
          <div className="eyebrow">{text.workspace}</div>
          <h1 tabIndex={-1}>{text.heading}</h1>
        </div>
        <div className="status">{text.vaultStatus}</div>
      </header>
      <div className="workbench-main">
      <section className="task-capture">
        <form onSubmit={save}>
          <fieldset disabled={busy}>
            <legend className="sr-only">{text.entryMode}</legend>
            <textarea
              data-control="task-entry-text"
              id="task-input-text"
              aria-label={text.entryLabel}
              value={input}
              onChange={(event) => setInput(event.target.value)}
              onPaste={mode === "capture" && !busy ? captureImage.paste : undefined}
              onKeyDown={(event) => {
                if (event.key === "Enter" && !event.shiftKey && !event.nativeEvent.isComposing) {
                  event.preventDefault();
                  event.currentTarget.form?.requestSubmit();
                }
              }}
              placeholder={text.placeholder}
              rows={2}
              required={mode !== "capture" || !captureImage.image}
            />
            {mode === "capture" && <InputImageAttachment attachment={captureImage} disabled={busy} />}
            <div className="task-entry-actions">
              <div className="task-entry-modes">
                <label>
                  <input
                    data-control="task-entry-capture-mode"
                    type="radio"
                    name="entry-mode"
                    value="capture"
                    checked={mode === "capture"}
                    onChange={() => setMode("capture")}
                  />
                  {text.capture}
                </label>
                <label>
                  <input
                    data-control="task-entry-task-mode"
                    type="radio"
                    name="entry-mode"
                    value="task"
                    checked={mode === "task"}
                    onChange={() => setMode("task")}
                  />
                  {text.task}
                </label>
              </div>
              <button
                className="primary"
                type="submit"
                data-control="task-entry-save"
                disabled={busy || captureImage.reading || (!input.trim() && !(mode === "capture" && captureImage.image))}
              >
                {busy ? "…" : text.save}
              </button>
            </div>
          </fieldset>
        </form>
        {error && (
          <div className="workbench-error" role="alert">
            {error}
            <button type="button" data-control="task-workbench-retry" onClick={() => void load()}>
              {text.retry}
            </button>
          </div>
        )}
      </section>
      {snapshot && (
        <div className="workbench-current">
        <section className="workbench-active shortcut-region" aria-labelledby="workbench-active-title">
          <header className="workbench-active-header">
            <div><small>{text.resume}</small><h2 id="workbench-active-title">{text.active}</h2></div>
            <button type="button" data-control="workbench-focus-active" aria-pressed={focusActive} onClick={() => setFocusActive(value => !value)}>{focusActive ? text.showAllWork : text.focus}</button>
            <span className="workbench-count">{activeTasks.length}</span>
          </header>
          {activeTasks.length ? <div className="workbench-active-cards">{activeTasks.map(renderCard)}</div> : <p className="region-empty">{text.noActive}</p>}
        </section>
        <aside className="workbench-recent" aria-labelledby="recent-completed-title">
          <h2 id="recent-completed-title">{text.recentCompleted}</h2>
          {completedTasks.length ? <ol className="completed-task-list">{completedTasks.map(task => (
            <li key={task.id}><button type="button" data-control="workbench-completed-open" onClick={event => selectDetail(task.id, event.currentTarget)}>{task.title}</button></li>
          ))}</ol> : <p className="region-empty">{text.noCompleted}</p>}
        </aside>
        </div>
      )}
      {snapshot && (
        <section className="workbench-board" aria-label={text.categories}>
          {trackedTitle && <p className="region-empty">{trackedTitle}</p>}
          {categories.map((category, categoryIndex) => {
            const categoryItems = new Set(category.items.map(item => `${item.kind}:${item.id}`));
            const belongs = (item: WorkbenchItem) => categoryItems.has(`${item.kind}:${item.id}`);
            return (
            <section className="workbench-category" data-category={category.id} aria-labelledby={`category-${categoryIndex}`} key={category.id}>
              <header className="workbench-category-header">
                <h2 id={`category-${categoryIndex}`}>{category.label}</h2>
              </header>
              <div className="workbench-category-scroll" role="region" aria-labelledby={`category-${categoryIndex}`} tabIndex={0}>
              <div className="workbench-lanes">
            {lanes.map(lane => ({ ...lane, items: lane.items.filter(belongs) })).map((lane) => (
              <section className={`workbench-lane workbench-lane-${lane.id}`} data-lane={lane.id} aria-labelledby={`lane-${categoryIndex}-${lane.id}`} key={lane.id}>
                <header className="workbench-lane-header">
                  <h3 id={`lane-${categoryIndex}-${lane.id}`}>{lane.title}</h3>
                  <span className="workbench-count">{lane.items.length}</span>
                </header>
                <p className="workbench-lane-hint">{lane.hint}</p>
                <div className="workbench-lane-cards">
                  {lane.items.length ? lane.items.map(renderCard) : <p className="region-empty">{lane.empty}</p>}
                </div>

              </section>
            ))}
              </div>
              </div>
            </section>
            );
          })}
        </section>
      )}
      </div>
      {detail && (
        <TaskDetail
          key={detail}
          ref={detailRef}
          taskId={detail}
          onClose={() => {
            const leave = () => { setRefining(undefined); setDetail(undefined); restoreDetailFocus(); };
            if (refinementRef.current) refinementRef.current.requestLeave(leave); else leave();
          }}
          onOpenTask={(id) => selectDetail(id)}
          onRefine={() => openRefinement({ kind: "task", id: detail })}
          refreshKey={detailRefresh}
          queueImageSummary={queueImageSummary?.taskId === detail ? queueImageSummary : undefined}
          queueKnowledgeDraft={queueKnowledgeDraft?.taskId === detail ? queueKnowledgeDraft : undefined}
          suppressInitialFocus={Boolean(refining)}
          onChanged={() => void load()}
          sessions={detailSessions.current}
          onRequestDelete={(title) => requestDelete("tasks", detail, title)}
        />
      )}
      {deleteTarget && (
        <DeleteItemDialog
          title={deleteTarget.title}
          busy={deleteBusy}
          error={deleteError}
          onConfirm={() => void confirmDelete()}
          onCancel={() => setDeleteTarget(undefined)}
        />
      )}
      {refining && (
        <RefinementPanel
          key={`${refining.kind}:${refining.id}`}
          ref={refinementRef}
          messageDrafts={refinementMessages.current}
          imageDrafts={refinementImages.current}
          subjectTitle={refining.kind === "task" ? taskFor(refining.id)?.title : captureFor(refining.id)?.text || text.imageCapture}
          onApplied={() => { setDetailRefresh(value => value + 1); void load(); }}
          kind={refining.kind}
          subjectId={refining.id}
          onClose={() => {
            setRefining(undefined);
            restoreDetailFocus();
            void load();
          }}
        />
      )}{" "}
    </section>
  );
}
