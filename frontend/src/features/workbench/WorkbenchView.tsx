import { useCallback, useEffect, useRef, useState } from "react";
import { taskClient } from "../../services/taskClient";
import type {
  CaptureCard,
  TaskCard,
  WorkbenchItem,
  WorkbenchSnapshot,
} from "../../types/taskWorkbench";
import { DeleteItemDialog } from "./DeleteItemDialog";
import { RefinementPanel } from "./RefinementPanel";
import { TaskDetail, type TaskDetailHandle } from "./TaskDetail";
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
  const detailTrigger = useRef<HTMLElement | null>(null);
  const selectDetail = (id: string | undefined, trigger?: HTMLElement) => {
    if (id === detail) return;
    const proceed = () => {
      setDetail(id);
      if (id) detailTrigger.current = trigger ?? null;
      else {
        const target = detailTrigger.current?.isConnected && !detailTrigger.current.closest(".view:not(.active), [hidden]") ? detailTrigger.current : document.querySelector<HTMLElement>("#workbench.active h1");
        target?.focus();
      }
    };
    if (detailRef.current) detailRef.current.requestLeave(proceed);
    else proceed();
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
      aria-busy={!snapshot && !error}
    >
      <header className="top">
        <div>
          <div className="eyebrow">{text.workspace}</div>
          <h1 tabIndex={-1}>{text.heading}</h1>
        </div>
        <div className="status">{text.vaultStatus}</div>
      </header>
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
                      className="shortcut-card"
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
                      className="shortcut-card subtle"
                      key={`${shortcut.kind}-${shortcut.id}`}
                      onClick={() =>
                        setRefining({ kind: shortcut.kind, id: shortcut.id })
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
                        className="canonical-card"
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
                                onClick={() =>
                                  setRefining({ kind: "task", id: item.id })
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
                              onClick={() =>
                                setRefining({ kind: "capture", id: item.id })
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
                                window.openChat?.("problems", item.problemId, {
                                  problemRevision: item.problemRevision,
                                  sourceTitle: item.title,
                                })
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
      {refining && (
        <RefinementPanel
          kind={refining.kind}
          subjectId={refining.id}
          onClose={() => {
            setRefining(undefined);
            void load();
          }}
        />
      )}{" "}
      {detail && (
        <TaskDetail
          key={detail}
          ref={detailRef}
          taskId={detail}
          onClose={() => {
            setDetail(undefined);
            const target = detailTrigger.current?.isConnected && !detailTrigger.current.closest(".view:not(.active), [hidden]") ? detailTrigger.current : document.querySelector<HTMLElement>("#workbench.active h1");
            target?.focus();
          }}
          onChanged={() => void load()}
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
    </section>
  );
}
