import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { TaskDetail } from "./TaskDetail";

const task = {
  id: "task-1",
  kind: "task" as const,
  taskRevision: 2,
  state: "in_progress" as const,
  title: "Independent work",
  workLog: [],
  checklist: [],
  decisions: [],
  readinessEntries: [
    { key: "scope", status: "missing" as const },
    { key: "validationCriteria", status: "resolved" as const },
  ],
};
const response = (body: unknown) => ({
  ok: true,
  status: 200,
  json: async () => body,
  text: async () => "",
  body: null,
});

describe("Task detail", () => {
  it("keeps dependent mutations inert until the current revision is rendered", async () => {
    let finishWorkLog!: (value: ReturnType<typeof response>) => void;
    const workLog = new Promise<ReturnType<typeof response>>((resolve) => {
      finishWorkLog = resolve;
    });
    const request = vi.fn().mockImplementation(({ path, method }: { path: string; method?: string }) =>
      path === "/tasks/task-1/work-log" && method === "POST"
        ? workLog
        : Promise.resolve(response(task)),
    );
    window.llmWikiApplication = { request };
    render(
      <TaskDetail taskId="task-1" onClose={vi.fn()} onChanged={vi.fn()} />,
    );
    const workLogField = await screen.findByLabelText("Work Log entry");
    fireEvent.change(workLogField, { target: { value: "Revision-bound evidence" } });
    fireEvent.click(workLogField.parentElement!.querySelector("button")!);
    const detail = screen.getByLabelText("Independent work");
    await waitFor(() => expect(detail).toHaveAttribute("aria-busy", "true"));

    const decisionField = screen.getByLabelText("Decision");
    fireEvent.change(decisionField, { target: { value: "Must wait" } });
    fireEvent.click(decisionField.parentElement!.querySelector("button")!);
    expect(request.mock.calls.some(([input]) => input.path === "/tasks/task-1/decisions")).toBe(false);

    finishWorkLog(response(task));
    await waitFor(() => expect(detail).toHaveAttribute("aria-busy", "false"));
  });

  it("requires authored completion evidence and permits work before readiness is resolved", async () => {
    const request = vi.fn().mockResolvedValue(response(task));
    window.llmWikiApplication = { request };
    render(
      <TaskDetail taskId="task-1" onClose={vi.fn()} onChanged={vi.fn()} />,
    );
    await screen.findByText("Independent work");
    expect(screen.getAllByText("Scope")).toHaveLength(2);
    expect(screen.getAllByText("Validation criteria")).toHaveLength(2);
    expect(screen.getByText("Missing")).toBeVisible();
    expect(screen.queryByText("validationCriteria")).not.toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Complete Task" }),
    ).toBeDisabled();
    fireEvent.change(screen.getByLabelText("Completion evidence"), {
      target: { value: "Validated in a real workflow" },
    });
    expect(screen.getByRole("button", { name: "Complete Task" })).toBeEnabled();
    fireEvent.change(screen.getByLabelText("Work Log entry"), {
      target: { value: "Started gathering evidence" },
    });
    fireEvent.click(screen.getAllByRole("button", { name: "Add" })[0]);
    await waitFor(() =>
      expect(request).toHaveBeenCalledWith(
        expect.objectContaining({
          path: "/tasks/task-1/work-log",
          method: "POST",
        }),
      ),
    );
  });

  it("sends the current Task revision and the checkbox event value", async () => {
    const checklistTask = {
      ...task,
      checklist: [{ id: "check-1", body: "Verify result", checked: false }],
    };
    const request = vi.fn().mockResolvedValue(response(checklistTask));
    window.llmWikiApplication = { request };
    render(
      <TaskDetail taskId="task-1" onClose={vi.fn()} onChanged={vi.fn()} />,
    );

    fireEvent.click(await screen.findByRole("checkbox", { name: "Verify result" }));

    await waitFor(() => {
      const checklistUpdate = request.mock.calls
        .map(([input]) => input)
        .find(
          (input) =>
            input.path === "/tasks/task-1/checklist/check-1" &&
            input.method === "PUT",
        );
      expect(JSON.parse(checklistUpdate.body)).toMatchObject({
        expectedTaskRevision: 2,
        checked: true,
        body: "Verify result",
      });
    });
  });

  it("maps an authored decision to the append-only decision contract", async () => {
    const request = vi.fn().mockResolvedValue(response(task));
    window.llmWikiApplication = { request };
    render(
      <TaskDetail taskId="task-1" onClose={vi.fn()} onChanged={vi.fn()} />,
    );
    const decisionField = await screen.findByLabelText("Decision");
    fireEvent.change(decisionField, {
      target: { value: "Ship deliberately" },
    });
    fireEvent.click(decisionField.parentElement!.querySelector("button")!);

    await waitFor(() => {
      const decision = request.mock.calls
        .map(([input]) => input)
        .find((input) => input.path === "/tasks/task-1/decisions");
      expect(JSON.parse(decision.body)).toMatchObject({
        expectedTaskRevision: 2,
        kind: "user",
        payload: { body: "Ship deliberately" },
      });
    });
  });

  it("retains a dirty Task definition while a Work Log refreshes the persisted snapshot", async () => {
    const refreshed = { ...task, taskRevision: 3, workLog: [{ id: "log-1", body: "Independent evidence" }] };
    const request = vi.fn().mockImplementation(({ path, method }: { path: string; method?: string }) =>
      path === "/tasks/task-1/work-log" && method === "POST"
        ? Promise.resolve(response({ id: "task-1", taskRevision: 3 }))
        : Promise.resolve(response(refreshed)),
    );
    window.llmWikiApplication = { request };
    render(<TaskDetail taskId="task-1" onClose={vi.fn()} onChanged={vi.fn()} />);
    const title = await screen.findByLabelText("Title");
    fireEvent.change(title, { target: { value: "Keep this edited title" } });
    fireEvent.change(screen.getByLabelText("Work Log entry"), { target: { value: "Independent evidence" } });
    fireEvent.click(document.querySelector('[data-control="task-worklog-add"]')!);
    await waitFor(() => expect(screen.getByLabelText("Title")).toHaveValue("Keep this edited title"));
    expect(screen.getByText("Independent evidence")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Save changes" }));
    await waitFor(() => {
      const revise = request.mock.calls.map(([input]) => input).find((input) => input.path === "/tasks/task-1/revisions");
      expect(JSON.parse(revise.body)).toMatchObject({ expectedTaskRevision: 3, patch: { title: "Keep this edited title" } });
    });
  });

  it("keeps the panel open when a dirty close is cancelled", async () => {
    const onClose = vi.fn();
    window.llmWikiApplication = { request: vi.fn().mockResolvedValue(response(task)) };
    render(<TaskDetail taskId="task-1" onClose={onClose} onChanged={vi.fn()} />);
    fireEvent.change(await screen.findByLabelText("Title"), { target: { value: "Unsaved title" } });
    fireEvent.click(screen.getByRole("button", { name: "Close Task detail" }));
    expect(screen.getByRole("alertdialog")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Keep editing" }));
    expect(onClose).not.toHaveBeenCalled();
    expect(screen.getByLabelText("Title")).toHaveValue("Unsaved title");
  });
});

const control = (id: string) => document.querySelector<HTMLElement>(`[data-control="${id}"]`)!;
describe("Task draft protection", () => {
  it("keeps all definition inputs through independent Work Log and comment refresh", async () => {
    let current = { ...task, detail: "Original", workLog: [{ id: "log", body: "Existing", comments: [] }] };
    const request = vi.fn().mockImplementation(({ method }: { method?: string }) => {
      if (method === "POST") {
        current = { ...current, workLog: [...current.workLog, { id: `added-${current.workLog.length}`, body: "Added", comments: [] }] };
        return Promise.resolve(response({ id: "new-entry" }));
      }
      return Promise.resolve(response(current));
    });
    window.llmWikiApplication = { request };
    render(<TaskDetail taskId={task.id} onClose={vi.fn()} onChanged={vi.fn()} />);
    await screen.findByText(task.title);
    for (const name of ["title", "detail", "outcome", "scope", "non-goals", "criteria"]) {
      fireEvent.change(control(`task-revision-${name}`), { target: { value: `Edited ${name}` } });
    }
    fireEvent.change(control("task-worklog-text"), { target: { value: "Add work" } });
    fireEvent.click(control("task-worklog-add"));
    await waitFor(() => expect(document.querySelector(".task-detail")).toHaveAttribute("aria-busy", "false"));
    fireEvent.change(control("task-comment-text"), { target: { value: "Comment" } });
    fireEvent.click(control("task-comment-add"));
    await waitFor(() => expect(request.mock.calls.some(([arg]) => arg.path === "/work-log/log/comments")).toBe(true));
    for (const name of ["title", "detail", "outcome", "scope", "non-goals", "criteria"]) expect(control(`task-revision-${name}`)).toHaveValue(`Edited ${name}`);
    expect(document.querySelector(".task-draft-conflict")).not.toBeInTheDocument();
  });

  it("uses the original revision, retains failed input, then requires explicit overlapping rebase", async () => {
    let current = { ...task, detail: "Original" };
    let conflict = true;
    const request = vi.fn().mockImplementation(({ path, method, body }: { path: string; method?: string; body?: string }) => {
      if (path.endsWith("/revisions") && method === "POST") {
        if (conflict) {
          current = { ...current, taskRevision: 3, detail: "External" };
          return Promise.resolve({ ...response({ detail: "head_conflict: currentRevision=3" }), ok: false, status: 409 });
        }
        const patch = JSON.parse(body!).patch;
        current = { ...current, ...patch, taskRevision: 4 };
        return Promise.resolve(response({ id: task.id, taskRevision: 4, title: current.title, detail: current.detail, outcome: "", scope: "", nonGoals: "", validationCriteria: "" }));
      }
      return Promise.resolve(response(current));
    });
    const close = vi.fn();
    window.llmWikiApplication = { request };
    render(<TaskDetail taskId={task.id} onClose={close} onChanged={vi.fn()} />);
    await screen.findByText(task.title);
    fireEvent.change(control("task-revision-detail"), { target: { value: "Local" } });
    fireEvent.click(control("task-detail-close"));
    fireEvent.click(control("task-draft-guard-save"));
    await screen.findByText("External");
    expect(close).not.toHaveBeenCalled();
    expect(control("task-revision-detail")).toHaveValue("Local");
    expect(JSON.parse(request.mock.calls.find(([arg]) => arg.path.endsWith("/revisions"))![0].body).expectedTaskRevision).toBe(2);
    expect(control("task-draft-guard-save")).toBeDisabled();
    fireEvent.click(control("task-draft-keep-mine"));
    conflict = false;
    fireEvent.click(control("task-draft-guard-save"));
    await waitFor(() => expect(close).toHaveBeenCalledOnce());
    const writes = request.mock.calls.filter(([arg]) => arg.path.endsWith("/revisions"));
    expect(JSON.parse(writes[1][0].body).expectedTaskRevision).toBe(3);
  });

  it("saves normalized response fields and retains aggregate state when the follow-up GET fails", async () => {
    let saved = false;
    window.llmWikiApplication = { request: vi.fn().mockImplementation(({ path }: { path: string }) => {
      if (path.endsWith("/revisions")) {
        saved = true;
        return Promise.resolve(response({ id: task.id, taskRevision: 3, title: task.title, detail: "Normalized", outcome: "", scope: "", nonGoals: "", validationCriteria: "" }));
      }
      if (saved) return Promise.reject(new Error("Offline"));
      return Promise.resolve(response(task));
    }) };
    render(<TaskDetail taskId={task.id} onClose={vi.fn()} onChanged={vi.fn()} />);
    await screen.findByText(task.title);
    fireEvent.change(control("task-revision-detail"), { target: { value: "  Normalized  " } });
    fireEvent.click(control("task-revision-save"));
    await screen.findByText(/Changes saved, but/);
    expect(control("task-revision-detail")).toHaveValue("Normalized");
    expect(document.querySelector(".task-detail")).toHaveAttribute("data-task-state", "in_progress");
    expect(control("task-revision-save")).toBeDisabled();
  });

  it("keeps editing on Escape prompt cancellation and closes only after discard", async () => {
    window.llmWikiApplication = { request: vi.fn().mockResolvedValue(response(task)) };
    const close = vi.fn();
    render(<TaskDetail taskId={task.id} onClose={close} onChanged={vi.fn()} />);
    await screen.findByText(task.title);
    fireEvent.change(control("task-revision-detail"), { target: { value: "Uncommitted" } });
    fireEvent.keyDown(control("task-revision-detail"), { key: "Escape" });
    fireEvent.click(control("task-draft-guard-keep-editing"));
    expect(close).not.toHaveBeenCalled();
    expect(control("task-revision-detail")).toHaveValue("Uncommitted");
    fireEvent.click(control("task-detail-close"));
    fireEvent.click(control("task-draft-guard-discard"));
    expect(close).toHaveBeenCalledOnce();
  });
});

it("localizes calculated readiness text but preserves authored reasons", async () => {
  document.documentElement.lang = "ko";
  try {
    window.llmWikiApplication = { request: vi.fn().mockResolvedValue(response({ ...task, readinessEntries: [
      { key: "prerequisites", status: "resolved", reason: "No explicit prerequisite", provenance: "calculated" },
      { key: "scope", status: "missing", reason: "User-authored rationale" },
    ] })) };
    render(<TaskDetail taskId={task.id} onClose={vi.fn()} onChanged={vi.fn()} />);
    await screen.findByText("명시된 선행 조건 없음");
    expect(screen.getByText("User-authored rationale")).toBeVisible();
    expect(screen.queryByText("No explicit prerequisite")).not.toBeInTheDocument();
    expect(screen.getByLabelText("범위 사유")).toBeInTheDocument();
  } finally { document.documentElement.lang = "en"; }
});

it("rebases a disjoint refresh before save and lets the user discard overlapping values", async () => {
  let current = { ...task, detail: "Original" };
  let overlap = false;
  const request = vi.fn().mockImplementation(({ path, method, body }: { path: string; method?: string; body?: string }) => {
    if (path.endsWith("/work-log") && method === "POST") {
      current = { ...current, taskRevision: current.taskRevision + 1, title: "External title", detail: overlap ? "External detail" : current.detail };
      return Promise.resolve(response({ id: "entry" }));
    }
    if (path.endsWith("/revisions")) {
      current = { ...current, ...JSON.parse(body!).patch, taskRevision: current.taskRevision + 1 };
      return Promise.resolve(response({ ...current, outcome: "", scope: "", nonGoals: "", validationCriteria: "" }));
    }
    return Promise.resolve(response(current));
  });
  window.llmWikiApplication = { request };
  render(<TaskDetail taskId={task.id} onClose={vi.fn()} onChanged={vi.fn()} />);
  await screen.findByText(task.title);
  fireEvent.change(control("task-revision-detail"), { target: { value: "Local" } });
  fireEvent.change(control("task-worklog-text"), { target: { value: "Evidence" } });
  fireEvent.click(control("task-worklog-add"));
  await screen.findByText("External title");
  await waitFor(() => expect(document.querySelector(".task-detail")).toHaveAttribute("aria-busy", "false"));
  fireEvent.click(control("task-revision-save"));
  await waitFor(() => expect(control("task-revision-save")).toBeDisabled());
  expect(JSON.parse(request.mock.calls.find(([arg]) => arg.path.endsWith("/revisions"))![0].body)).toMatchObject({ expectedTaskRevision: 3, patch: { detail: "Local" } });
  await waitFor(() => expect(document.querySelector(".task-detail")).toHaveAttribute("aria-busy", "false"));
  overlap = true;
  fireEvent.change(control("task-revision-detail"), { target: { value: "Another local" } });
  fireEvent.change(control("task-worklog-text"), { target: { value: "More evidence" } });
  fireEvent.click(control("task-worklog-add"));
  await screen.findByText("External detail");
  await waitFor(() => expect(document.querySelector(".task-detail")).toHaveAttribute("aria-busy", "false"));
  fireEvent.click(control("task-draft-use-latest"));
  expect(control("task-revision-detail")).toHaveValue("External detail");
  expect(control("task-revision-save")).toBeDisabled();
});
