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
    [snapshot, setSnapshot] = useState<WorkbenchSnapshot>(),
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
  const detailRef = useRef<TaskDetailHandle>(null);
  const refinementRef = useRef<RefinementPanelHandle>(null);
  const [detailRefresh, setDetailRefresh] = useState(0);
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
  const save = async (event: React.FormEvent) => {
    event.preventDefault();
    if (!input.trim() || busy) return;
    setBusy(true);
    setError("");
    try {
      await (mode === "capture"
        ? taskClient.createCapture(input.trim())
        : taskClient.createTask(input.trim()));
      setInput("");
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
        : text.ready;
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
  return (
    <section
      id="workbench"
      className={`view task-workbench${active ? " active" : ""}`}
      data-task-workbench="true"
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
              onKeyDown={(event) => {
                if (event.key === "Enter" && !event.shiftKey && !event.nativeEvent.isComposing) {
                  event.preventDefault();
                  event.currentTarget.form?.requestSubmit();
                }
              }}
              placeholder={text.placeholder}
              rows={2}
              required
            />
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
                disabled={busy || !input.trim()}
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
        <>
          {trackedTitle && <p className="region-empty">{trackedTitle}</p>}
          <section className="shortcut-region" aria-label={text.active}>
            <header>
              <small>{text.resume}</small>
              <h2>{text.active}</h2>
            </header>
            {snapshot.activeShortcuts.length ? (
              <div className="shortcut-list">
                {snapshot.activeShortcuts.map((shortcut) => {
                  const task = taskFor(shortcut.id);
                  return (
                    <button
                      data-control="task-shortcut-open"
                      data-entity-id={shortcut.id}
                      type="button"
                      className={`shortcut-card${detail === shortcut.id ? " selected" : ""}`}
                      key={shortcut.id}
                      onClick={(event) => selectDetail(shortcut.id, event.currentTarget)}
                    >
                      <small>{text.inProgress} · r{shortcut.taskRevision}</small>
                      <strong>{task?.title ?? shortcut.id}</strong>
                      <span>{text.open}</span>
                    </button>
                  );
                })}
              </div>
            ) : (
              <p className="region-empty">{text.noActive}</p>
            )}
          </section>
          <section
            className="shortcut-region refinement-shortcuts"
            aria-label={text.refining}
          >
            <header>
              <small>{text.refining}</small>
              <h2>{text.refining}</h2>
            </header>
            {snapshot.refiningShortcuts.length ? (
              <div className="shortcut-list">
                {snapshot.refiningShortcuts.map((shortcut) => {
                  const task =
                      shortcut.kind === "task"
                        ? taskFor(shortcut.id)
                        : undefined,
                    capture =
                      shortcut.kind === "capture"
                        ? captureFor(shortcut.id)
                        : undefined;
                  return (
                    <button
                      data-control="task-shortcut-refine"
                      data-entity-id={shortcut.id}
                      type="button"
                      className={`shortcut-card subtle${refining?.kind === shortcut.kind && refining.id === shortcut.id ? " selected" : ""}`}
                      key={`${shortcut.kind}-${shortcut.id}`}
                      onClick={(event) =>
                        openRefinement({ kind: shortcut.kind, id: shortcut.id }, event.currentTarget)
                      }
                    >
                      <small>
                        {(shortcut.kind === "task" ? text.task : text.capture)} · draft r{shortcut.draftRevision}
                      </small>
                      <strong>
                        {task?.title ?? capture?.text ?? shortcut.id}
                      </strong>
                      <span>{text.refine}</span>
                    </button>
                  );
                })}
              </div>
            ) : (
              <p className="region-empty">{text.noRefining}</p>
            )}
          </section>
          <section className="canonical-work">
            <header>
              <h2>{text.categories}</h2>
            </header>
            {snapshot.categories.length ? (
              snapshot.categories.map((category) => (
                <section className="category-group" key={category.id}>
                  <header>
                    <h3>{category.label}</h3>
                    <span>{category.items.length}</span>
                  </header>
                  <div className="category-grid">
                    {category.items.map((item) => (
                      <article
                        className={`canonical-card${(item.kind === "task" && detail === item.id) || (refining?.kind === item.kind && refining.id === item.id) ? " selected" : ""}`}
                        key={`${item.kind}-${item.id}`}
                      >
                        <small>
                          {item.kind === "task"
                            ? stateLabel(item as TaskCard)
                            : item.kind === "capture"
                              ? text.capture
                              : text.refining}
                        </small>
                        <h3>{itemTitle(item)}</h3>
                          {item.kind === "task" && (
                          <p>
                            {(item as TaskCard).readiness
                              ? `${(item as TaskCard).readiness!.missing} ${text.readinessItemsRemain}`
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
                                data-control="task-card-refine"
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
                              data-control="task-card-refine"
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
                              data-control="task-card-refine"
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
                      </article>
                    ))}
                  </div>
                </section>
              ))
            ) : (
              <p className="region-empty">{text.empty}</p>
            )}
          </section>
        </>
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
          onRefine={() => openRefinement({ kind: "task", id: detail })}
          refreshKey={detailRefresh}
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
          subjectTitle={refining.kind === "task" ? taskFor(refining.id)?.title : captureFor(refining.id)?.text}
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
